/**
 * Les plans : publier, consulter le fil, annuler, voir les demandes reçues.
 *
 * Il n'existe volontairement aucune route pour « aimer », « passer » ou faire
 * remonter un plan. On publie ce qu'on compte faire, et on demande à venir en
 * écrivant.
 */
import { Elysia, t } from "elysia";
import {
  MAX_OPEN_PLANS,
  PLAN_CAPACITY_GROUP_MAX,
  PLAN_CAPACITY_SOLO,
  PLAN_CATEGORIES,
  PLAN_MIN_LEAD_MINUTES,
  PLAN_NOTE_MAX_CHARS,
  PLAN_TITLE_MAX_CHARS,
  PLAN_TITLE_MIN_CHARS,
  type Feed,
} from "@weave/contracts";
import { invalidateAllFeeds } from "../lib/cache.ts";
import { forbidden, invalid, notFound, tooManyPlans } from "../lib/errors.ts";
import { prisma } from "../lib/prisma.ts";
import { ageFrom } from "../lib/time.ts";
import { authPlugin } from "../plugins/auth.ts";
import { consume, RULES } from "../plugins/rate-limit.ts";
import { entitlementsFor, requireCredit } from "./entitlements.ts";
import { buildFeed, photoSignee } from "./plans.service.ts";
import { publishLiveActivityState } from "./live-activity.service.ts";

export const planRoutes = new Elysia({ prefix: "/v1/plans", tags: ["Plans"] })
  .use(authPlugin)

  .get(
    "/",
    async ({ requireAccount }): Promise<Feed> => {
      const compte = requireAccount();
      await consume(RULES.feed, compte.id);
      return buildFeed(compte);
    },
    {
      detail: {
        summary: "Le fil des plans à venir",
        description:
          "Les plans autour de vous, dans vos critères, du plus imminent au plus lointain. Aucun classement payant : ni palier ni achat ne fait remonter un plan.",
      },
    },
  )

  .post(
    "/",
    async ({ requireAccount, body }) => {
      const compte = requireAccount();
      await consume(RULES.publish, compte.id);
      const droits = entitlementsFor(compte.tier);

      const debut = new Date(body.startsAt);
      if (Number.isNaN(debut.getTime())) throw invalid("Date de rendez-vous illisible.");

      const minimum = new Date(Date.now() + PLAN_MIN_LEAD_MINUTES * 60_000);
      if (debut < minimum) {
        throw invalid(`Un plan se publie au moins ${PLAN_MIN_LEAD_MINUTES} minutes à l'avance.`);
      }

      const horizon = new Date(Date.now() + droits.daysAhead * 24 * 60 * 60 * 1000);
      if (debut > horizon) {
        // Le palier borne l'horizon ; un crédit « Horizon » l'ouvre une fois.
        await requireCredit(compte.id, "horizon");
      }

      const capacite = body.capacity ?? PLAN_CAPACITY_SOLO;
      if (capacite > PLAN_CAPACITY_SOLO && !droits.groupPlans) {
        await requireCredit(compte.id, "tablee");
      }

      const ouverts = await prisma.plan.count({
        where: { authorId: compte.id, state: "ouvert", startsAt: { gt: new Date() } },
      });
      if (ouverts >= MAX_OPEN_PLANS) throw tooManyPlans(MAX_OPEN_PLANS);

      const profil = await prisma.profile.findUnique({
        where: { accountId: compte.id },
        select: { city: true, latRounded: true, lonRounded: true },
      });
      if (profil === null) throw invalid("Renseignez d'abord votre ville.");

      const plan = await prisma.plan.create({
        data: {
          authorId: compte.id,
          title: body.title.trim(),
          note: body.note?.trim() ?? "",
          category: body.category,
          startsAt: debut,
          city: body.city ?? profil.city,
          latRounded: profil.latRounded,
          lonRounded: profil.lonRounded,
          capacity: capacite,
        },
        select: { id: true, title: true, startsAt: true },
      });

      // Un plan publié doit apparaître tout de suite, pas à l'expiration des
      // fils déjà composés.
      await invalidateAllFeeds();
      await publishLiveActivityState(compte.id);

      return { id: plan.id, title: plan.title, startsAt: plan.startsAt.toISOString() };
    },
    {
      body: t.Object({
        title: t.String({ minLength: PLAN_TITLE_MIN_CHARS, maxLength: PLAN_TITLE_MAX_CHARS }),
        note: t.Optional(t.String({ maxLength: PLAN_NOTE_MAX_CHARS })),
        category: t.Union(PLAN_CATEGORIES.map((c) => t.Literal(c))),
        startsAt: t.String({ description: "Date et heure du rendez-vous, ISO 8601" }),
        city: t.Optional(t.String({ maxLength: 80 })),
        capacity: t.Optional(
          t.Integer({ minimum: PLAN_CAPACITY_SOLO, maximum: PLAN_CAPACITY_GROUP_MAX }),
        ),
      }),
      detail: {
        summary: "Publier un plan",
        description: `Au plus ${MAX_OPEN_PLANS} plans ouverts à la fois. L'horizon de publication dépend de votre offre.`,
      },
    },
  )

  .get(
    "/mine",
    async ({ requireAccount }) => {
      const compte = requireAccount();

      const lignes = await prisma.plan.findMany({
        where: { authorId: compte.id },
        orderBy: { startsAt: "asc" },
        take: 50,
        include: {
          requests: {
            where: { state: { in: ["envoyee", "acceptee"] } },
            select: { state: true },
          },
        },
      });

      return lignes.map((ligne) => {
        const acceptees = ligne.requests.filter((r) => r.state === "acceptee").length;
        const enAttente = ligne.requests.filter((r) => r.state === "envoyee").length;
        return {
          id: ligne.id,
          title: ligne.title,
          note: ligne.note,
          category: ligne.category,
          startsAt: ligne.startsAt.toISOString(),
          city: ligne.city,
          capacity: ligne.capacity,
          seatsLeft: Math.max(0, ligne.capacity - acceptees),
          state: ligne.startsAt < new Date() ? "passe" : ligne.state,
          pendingRequests: enAttente,
        };
      });
    },
    { detail: { summary: "Mes plans" } },
  )

  .get(
    "/:id/requests",
    async ({ requireAccount, params }) => {
      const compte = requireAccount();

      const plan = await prisma.plan.findUnique({
        where: { id: params.id },
        select: { authorId: true },
      });
      if (plan === null) throw notFound("Plan introuvable.");
      // Seul l'auteur voit les demandes reçues : elles ne sont pas publiques.
      if (plan.authorId !== compte.id) throw forbidden("Ce plan n'est pas le vôtre.");

      const demandes = await prisma.joinRequest.findMany({
        where: { planId: params.id, state: "envoyee" },
        orderBy: { sentAt: "asc" },
        include: {
          author: {
            select: {
              id: true,
              displayName: true,
              birthDate: true,
              verified: true,
              profile: { select: { photoKey: true } },
            },
          },
        },
      });

      return demandes.map((demande) => ({
        id: demande.id,
        message: demande.message,
        sentAt: demande.sentAt.toISOString(),
        author: {
          id: demande.author.id,
          displayName: demande.author.displayName,
          age: ageFrom(demande.author.birthDate),
          photoUrl: photoSignee(demande.author.profile?.photoKey ?? null),
          verified: demande.author.verified,
        },
      }));
    },
    {
      params: t.Object({ id: t.String() }),
      detail: { summary: "Les demandes reçues sur un de mes plans" },
    },
  )

  .delete(
    "/:id",
    async ({ requireAccount, params }) => {
      const compte = requireAccount();

      const plan = await prisma.plan.findUnique({
        where: { id: params.id },
        select: { authorId: true, state: true },
      });
      if (plan === null) throw notFound("Plan introuvable.");
      if (plan.authorId !== compte.id) throw forbidden("Ce plan n'est pas le vôtre.");

      await prisma.$transaction(async (tx) => {
        await tx.plan.update({
          where: { id: params.id },
          data: { state: "annule", cancelledAt: new Date() },
        });
        // Les demandes en attente n'ont plus d'objet : on les clôt plutôt que
        // de laisser leurs auteurs attendre une réponse qui ne viendra pas.
        await tx.joinRequest.updateMany({
          where: { planId: params.id, state: "envoyee" },
          data: { state: "expiree", decidedAt: new Date() },
        });
      });

      await invalidateAllFeeds();
      await publishLiveActivityState(compte.id);
      return { ok: true };
    },
    {
      params: t.Object({ id: t.String() }),
      detail: {
        summary: "Annuler un plan",
        description: "Les demandes encore en attente sont closes, sans notification accusatrice.",
      },
    },
  );
