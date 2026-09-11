/**
 * Son propre compte : sa fiche, ses critères, sa photo.
 *
 * La fiche est volontairement maigre — une ville, un genre, une phrase. Dans
 * Weave, ce n'est pas la fiche qui donne envie, c'est le plan. On ne remplit
 * pas un formulaire pour se rendre désirable ; on écrit ce qu'on compte faire.
 */
import { Elysia, t } from "elysia";
import { MAX_RADIUS_KM, MIN_AGE, PLAN_CATEGORIES, type Me } from "@weave/contracts";
import { invalidateFeed, requestsLeft } from "../lib/cache.ts";
import { invalid, notFound } from "../lib/errors.ts";
import { prisma } from "../lib/prisma.ts";
import { ageFrom, localDay } from "../lib/time.ts";
import { authPlugin, invalidateAccountCache } from "../plugins/auth.ts";
import { creditsFor, dailyRequestQuota } from "./entitlements.ts";
import { photoSignee } from "./plans.service.ts";

const BIO_MAX_CHARS = 160;

export const meRoutes = new Elysia({ prefix: "/v1/me", tags: ["Profil"] })
  .use(authPlugin)

  .get(
    "/",
    async ({ requireAccount }): Promise<Me> => {
      const compte = requireAccount();
      const ligne = await prisma.account.findUnique({
        where: { id: compte.id },
        select: {
          id: true,
          handle: true,
          displayName: true,
          birthDate: true,
          status: true,
          verified: true,
          createdAt: true,
          profile: { select: { city: true, bio: true, photoKey: true } },
        },
      });
      if (ligne === null) throw notFound("Compte introuvable.");

      return {
        id: ligne.id,
        handle: ligne.handle,
        displayName: ligne.displayName,
        age: ageFrom(ligne.birthDate),
        status: ligne.status as Me["status"],
        tier: compte.tier,
        city: ligne.profile?.city ?? "",
        bio: ligne.profile?.bio ?? "",
        photoUrl: photoSignee(ligne.profile?.photoKey ?? null),
        verified: ligne.verified,
        requestsLeftToday: await requestsLeft(
          compte.id,
          localDay(compte.timezone),
          await dailyRequestQuota(compte),
        ),
        credits: await creditsFor(compte.id),
        createdAt: ligne.createdAt.toISOString(),
      };
    },
    { detail: { summary: "Lire son compte" } },
  )

  .patch(
    "/",
    async ({ requireAccount, body }) => {
      const compte = requireAccount();

      await prisma.account.update({
        where: { id: compte.id },
        data: {
          ...(body.displayName !== undefined ? { displayName: body.displayName } : {}),
          ...(body.timezone !== undefined ? { timezone: body.timezone } : {}),
          ...(body.locale !== undefined ? { locale: body.locale } : {}),
        },
      });
      await invalidateAccountCache(compte.id);
      return { ok: true };
    },
    {
      body: t.Object({
        displayName: t.Optional(t.String({ minLength: 1, maxLength: 40 })),
        timezone: t.Optional(t.String({ maxLength: 64 })),
        locale: t.Optional(t.String({ maxLength: 10 })),
      }),
      detail: { summary: "Modifier son compte" },
    },
  )

  .put(
    "/profile",
    async ({ requireAccount, body }) => {
      const compte = requireAccount();

      // Les coordonnées sont arrondies au dépôt : Weave ne conserve jamais une
      // position plus précise que le kilomètre.
      const latRounded = Math.round(body.latitude * 100) / 100;
      const lonRounded = Math.round(body.longitude * 100) / 100;

      await prisma.profile.upsert({
        where: { accountId: compte.id },
        create: {
          accountId: compte.id,
          city: body.city,
          latRounded,
          lonRounded,
          gender: body.gender,
          bio: body.bio ?? "",
        },
        update: {
          city: body.city,
          latRounded,
          lonRounded,
          gender: body.gender,
          ...(body.bio !== undefined ? { bio: body.bio } : {}),
        },
      });

      // Une ville et un genre suffisent pour publier et pour demander : c'est
      // tout ce qui manque avant d'être actif.
      await prisma.account.updateMany({
        where: { id: compte.id, status: "onboarding" },
        data: { status: "active" },
      });
      await invalidateAccountCache(compte.id);
      await invalidateFeed(compte.id);
      return { ok: true };
    },
    {
      body: t.Object({
        city: t.String({ minLength: 1, maxLength: 80 }),
        latitude: t.Number({ minimum: -90, maximum: 90 }),
        longitude: t.Number({ minimum: -180, maximum: 180 }),
        gender: t.String({ maxLength: 40 }),
        bio: t.Optional(t.String({ maxLength: BIO_MAX_CHARS })),
      }),
      detail: {
        summary: "Déposer sa fiche",
        description:
          "Une ville, un genre, une phrase. Les coordonnées sont arrondies à ~1 km avant enregistrement.",
      },
    },
  )

  .get(
    "/preferences",
    async ({ requireAccount }) => {
      const compte = requireAccount();
      const pref = await prisma.preference.findUnique({ where: { accountId: compte.id } });
      if (pref === null) throw notFound("Critères introuvables.");

      return {
        minAge: pref.minAge,
        maxAge: pref.maxAge,
        maxDistanceKm: pref.maxDistanceKm,
        seeking: JSON.parse(pref.seekingJson) as string[],
        categories: JSON.parse(pref.categoriesJson) as string[],
        escaleCity: pref.escaleCity,
        escaleUntil: pref.escaleUntil?.toISOString() ?? null,
      };
    },
    { detail: { summary: "Lire les critères du fil" } },
  )

  .patch(
    "/preferences",
    async ({ requireAccount, body }) => {
      const compte = requireAccount();

      if (body.minAge !== undefined && body.maxAge !== undefined && body.minAge > body.maxAge) {
        throw invalid("L'âge minimum ne peut pas dépasser l'âge maximum.");
      }

      await prisma.preference.update({
        where: { accountId: compte.id },
        data: {
          ...(body.minAge !== undefined ? { minAge: body.minAge } : {}),
          ...(body.maxAge !== undefined ? { maxAge: body.maxAge } : {}),
          ...(body.maxDistanceKm !== undefined ? { maxDistanceKm: body.maxDistanceKm } : {}),
          ...(body.seeking !== undefined ? { seekingJson: JSON.stringify(body.seeking) } : {}),
          ...(body.categories !== undefined
            ? { categoriesJson: JSON.stringify(body.categories) }
            : {}),
        },
      });

      await invalidateFeed(compte.id);
      return { ok: true };
    },
    {
      body: t.Object({
        minAge: t.Optional(t.Integer({ minimum: MIN_AGE, maximum: 99 })),
        maxAge: t.Optional(t.Integer({ minimum: MIN_AGE, maximum: 99 })),
        maxDistanceKm: t.Optional(t.Integer({ minimum: 1, maximum: MAX_RADIUS_KM })),
        seeking: t.Optional(t.Array(t.String({ maxLength: 40 }), { maxItems: 6 })),
        categories: t.Optional(
          t.Array(t.Union(PLAN_CATEGORIES.map((c) => t.Literal(c))), { maxItems: 8 }),
        ),
      }),
      detail: { summary: "Ajuster les critères du fil" },
    },
  );
