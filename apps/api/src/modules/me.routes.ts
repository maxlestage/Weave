/**
 * Profil de la personne connectée : ses propres données, ses critères, son
 * motif. Rien ici ne concerne les profils proposés — ceux-là vivent en cache.
 */
import { Elysia, t } from "elysia";
import {
  MOTIF_TAGS,
  RESPONSE_MAX_CHARS,
  VOICE_FRAGMENT_MAX_SECONDS,
  WEAVING_HOURS,
  type Me,
} from "@weave/contracts";
import { invalid, notFound } from "../lib/errors.ts";
import { prisma } from "../lib/prisma.ts";
import { isValidWeavingHour } from "../lib/time.ts";
import { authPlugin, invalidateAccountCache } from "../plugins/auth.ts";
import { creditsFor } from "./entitlements.ts";
import { invalidatePool } from "./loom.service.ts";

export const meRoutes = new Elysia({ prefix: "/v1/me", tags: ["Profil"] })
  .use(authPlugin)

  .get(
    "/",
    async ({ requireAccount }): Promise<Me> => {
      const account = requireAccount();
      const row = await prisma.account.findUnique({
        where: { id: account.id },
        select: {
          id: true,
          handle: true,
          displayName: true,
          status: true,
          weavingHour: true,
          timezone: true,
          verified: true,
          createdAt: true,
          profile: { select: { motifTags: { select: { tag: true }, orderBy: { weight: "desc" } } } },
        },
      });
      if (row === null) throw notFound("Compte introuvable.");

      return {
        id: row.id,
        handle: row.handle,
        displayName: row.displayName,
        status: row.status as Me["status"],
        plan: account.plan,
        weavingHour: row.weavingHour,
        timezone: row.timezone,
        verified: row.verified,
        motif: row.profile?.motifTags.map((t) => t.tag) ?? [],
        credits: await creditsFor(account.id),
        createdAt: row.createdAt.toISOString(),
      };
    },
    { detail: { summary: "Lire son profil" } },
  )

  .patch(
    "/",
    async ({ requireAccount, body }) => {
      const account = requireAccount();

      if (body.weavingHour !== undefined && !isValidWeavingHour(body.weavingHour)) {
        throw invalid(
          `L'heure de tissage doit faire partie des créneaux proposés : ${WEAVING_HOURS.join(", ")}.`,
        );
      }

      await prisma.account.update({
        where: { id: account.id },
        data: {
          ...(body.displayName !== undefined ? { displayName: body.displayName } : {}),
          ...(body.weavingHour !== undefined ? { weavingHour: body.weavingHour } : {}),
          ...(body.timezone !== undefined ? { timezone: body.timezone } : {}),
          ...(body.locale !== undefined ? { locale: body.locale } : {}),
        },
      });
      await invalidateAccountCache(account.id);
      return { ok: true };
    },
    {
      body: t.Object({
        displayName: t.Optional(t.String({ minLength: 1, maxLength: 40 })),
        weavingHour: t.Optional(t.Integer({ minimum: 0, maximum: 23 })),
        timezone: t.Optional(t.String({ maxLength: 64 })),
        locale: t.Optional(t.String({ maxLength: 10 })),
      }),
      detail: { summary: "Modifier son profil" },
    },
  )

  .put(
    "/profile",
    async ({ requireAccount, body }) => {
      const account = requireAccount();

      // Les coordonnées sont arrondies au dépôt : Weave ne conserve jamais une
      // position plus précise que le kilomètre.
      const latRounded = Math.round(body.latitude * 100) / 100;
      const lonRounded = Math.round(body.longitude * 100) / 100;

      await prisma.profile.upsert({
        where: { accountId: account.id },
        create: {
          accountId: account.id,
          city: body.city,
          latRounded,
          lonRounded,
          gender: body.gender,
          intent: body.intent ?? "ouverte",
          bio: body.bio ?? "",
        },
        update: {
          city: body.city,
          latRounded,
          lonRounded,
          gender: body.gender,
          ...(body.intent !== undefined ? { intent: body.intent } : {}),
          ...(body.bio !== undefined ? { bio: body.bio } : {}),
        },
      });

      await invalidatePool(account.id);
      return { ok: true };
    },
    {
      body: t.Object({
        city: t.String({ minLength: 1, maxLength: 80 }),
        latitude: t.Number({ minimum: -90, maximum: 90 }),
        longitude: t.Number({ minimum: -180, maximum: 180 }),
        gender: t.String({ maxLength: 40 }),
        intent: t.Optional(t.String({ maxLength: 40 })),
        bio: t.Optional(t.String({ maxLength: 600 })),
      }),
      detail: {
        summary: "Déposer sa fiche",
        description: "Les coordonnées sont arrondies à ~1 km avant enregistrement.",
      },
    },
  )

  .put(
    "/fragments",
    async ({ requireAccount, body }) => {
      const account = requireAccount();
      const profile = await prisma.profile.findUnique({
        where: { accountId: account.id },
        select: { id: true },
      });
      if (profile === null) throw invalid("Déposez d'abord votre fiche.");

      for (const [index, fragment] of body.fragments.entries()) {
        const prompt = await prisma.prompt.findUnique({
          where: { id: fragment.promptId },
          select: { id: true },
        });
        if (prompt === null) throw invalid(`Question inconnue : ${fragment.promptId}`);

        await prisma.profileFragment.upsert({
          where: { profileId_promptId: { profileId: profile.id, promptId: fragment.promptId } },
          create: {
            profileId: profile.id,
            promptId: fragment.promptId,
            kind: fragment.kind ?? "question",
            body: fragment.body,
            audioKey: fragment.audioKey ?? null,
            durationSeconds: fragment.durationSeconds ?? null,
            position: index,
          },
          update: {
            kind: fragment.kind ?? "question",
            body: fragment.body,
            audioKey: fragment.audioKey ?? null,
            durationSeconds: fragment.durationSeconds ?? null,
            position: index,
          },
        });
      }

      await recomputeCompleteness(profile.id, account.id);
      await invalidatePool(account.id);
      return { ok: true };
    },
    {
      body: t.Object({
        fragments: t.Array(
          t.Object({
            promptId: t.String(),
            kind: t.Optional(t.Union([t.Literal("question"), t.Literal("voix")])),
            body: t.String({ minLength: 1, maxLength: RESPONSE_MAX_CHARS }),
            audioKey: t.Optional(t.String({ maxLength: 200 })),
            durationSeconds: t.Optional(
              t.Integer({ minimum: 1, maximum: VOICE_FRAGMENT_MAX_SECONDS }),
            ),
          }),
          { minItems: 1, maxItems: 6 },
        ),
      }),
      detail: { summary: "Répondre aux questions qui composent sa trame" },
    },
  )

  .put(
    "/motif",
    async ({ requireAccount, body }) => {
      const account = requireAccount();
      const profile = await prisma.profile.findUnique({
        where: { accountId: account.id },
        select: { id: true },
      });
      if (profile === null) throw invalid("Déposez d'abord votre fiche.");

      const tags = [...new Set(body.tags.map((tag) => tag.trim().toLowerCase()))]
        .filter((tag) => tag.length > 0)
        .slice(0, MOTIF_TAGS);

      if (tags.length !== MOTIF_TAGS) {
        throw invalid(`Un motif compte exactement ${MOTIF_TAGS} mots distincts.`);
      }

      await prisma.$transaction(async (tx) => {
        await tx.motifTag.deleteMany({ where: { profileId: profile.id } });
        await tx.motifTag.createMany({
          data: tags.map((tag, index) => ({
            profileId: profile.id,
            tag,
            weight: 100 - index * 10,
          })),
        });
      });

      await recomputeCompleteness(profile.id, account.id);
      await invalidatePool(account.id);
      return { ok: true, motif: tags };
    },
    {
      body: t.Object({
        tags: t.Array(t.String({ minLength: 2, maxLength: 24 }), {
          minItems: MOTIF_TAGS,
          maxItems: MOTIF_TAGS,
        }),
      }),
      detail: { summary: "Tisser son motif (cinq mots)" },
    },
  )

  .get(
    "/preferences",
    async ({ requireAccount }) => {
      const account = requireAccount();
      const preference = await prisma.preference.findUnique({ where: { accountId: account.id } });
      if (preference === null) throw notFound("Critères introuvables.");

      return {
        minAge: preference.minAge,
        maxAge: preference.maxAge,
        maxDistanceKm: preference.maxDistanceKm,
        seeking: JSON.parse(preference.seekingJson) as string[],
        intents: JSON.parse(preference.intentsJson) as string[],
        refined: JSON.parse(preference.refinedJson) as Record<string, unknown>,
        escaleCity: preference.escaleCity,
        escaleUntil: preference.escaleUntil?.toISOString() ?? null,
      };
    },
    { detail: { summary: "Lire ses critères" } },
  )

  .patch(
    "/preferences",
    async ({ requireAccount, body }) => {
      const account = requireAccount();

      if (body.minAge !== undefined && body.maxAge !== undefined && body.minAge > body.maxAge) {
        throw invalid("L'âge minimum ne peut pas dépasser l'âge maximum.");
      }

      await prisma.preference.update({
        where: { accountId: account.id },
        data: {
          ...(body.minAge !== undefined ? { minAge: body.minAge } : {}),
          ...(body.maxAge !== undefined ? { maxAge: body.maxAge } : {}),
          ...(body.maxDistanceKm !== undefined ? { maxDistanceKm: body.maxDistanceKm } : {}),
          ...(body.seeking !== undefined ? { seekingJson: JSON.stringify(body.seeking) } : {}),
          ...(body.intents !== undefined ? { intentsJson: JSON.stringify(body.intents) } : {}),
          ...(body.refined !== undefined ? { refinedJson: JSON.stringify(body.refined) } : {}),
        },
      });

      // Les critères ont changé : le vivier pré-calculé n'est plus valable.
      await invalidatePool(account.id);
      return { ok: true };
    },
    {
      body: t.Object({
        minAge: t.Optional(t.Integer({ minimum: 18, maximum: 99 })),
        maxAge: t.Optional(t.Integer({ minimum: 18, maximum: 99 })),
        maxDistanceKm: t.Optional(t.Integer({ minimum: 1, maximum: 300 })),
        seeking: t.Optional(t.Array(t.String({ maxLength: 40 }), { maxItems: 6 })),
        intents: t.Optional(t.Array(t.String({ maxLength: 40 }), { maxItems: 6 })),
        refined: t.Optional(t.Record(t.String(), t.Unknown())),
      }),
      detail: { summary: "Ajuster ses critères" },
    },
  );

/** Recalcule un indicateur de complétude, affiché pendant l'inscription. */
async function recomputeCompleteness(profileId: string, accountId: string): Promise<void> {
  const [fragments, motif, profile] = await Promise.all([
    prisma.profileFragment.count({ where: { profileId } }),
    prisma.motifTag.count({ where: { profileId } }),
    prisma.profile.findUnique({ where: { id: profileId }, select: { photoKey: true, city: true } }),
  ]);

  let score = 0;
  if (profile?.city) score += 20;
  if (profile?.photoKey) score += 20;
  score += Math.min(3, fragments) * 15;
  if (motif === MOTIF_TAGS) score += 15;

  await prisma.profile.update({ where: { id: profileId }, data: { completeness: score } });

  // Un profil complet devient proposable ; c'est le seul déclencheur d'activation.
  if (score >= 80) {
    await prisma.account.updateMany({
      where: { id: accountId, status: "onboarding" },
      data: { status: "active" },
    });
    await invalidateAccountCache(accountId);
  }
}
