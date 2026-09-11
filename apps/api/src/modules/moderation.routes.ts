/**
 * Sécurité des personnes : blocage et signalement.
 *
 * Un blocage est immédiat et réciproque dans ses effets : les plans de chacun
 * disparaissent du fil de l'autre, les demandes en attente sont closes et la
 * conversation éventuelle est fermée — sans notification à la personne bloquée.
 */
import { Elysia, t } from "elysia";
import { MESSAGE_RETENTION_DAYS } from "@weave/contracts";
import { invalidateFeed } from "../lib/cache.ts";
import { invalid } from "../lib/errors.ts";
import { prisma } from "../lib/prisma.ts";
import { authPlugin, invalidateAccountCache } from "../plugins/auth.ts";
import { consume, RULES } from "../plugins/rate-limit.ts";
import { publishLiveActivityState } from "./live-activity.service.ts";

const REASONS = [
  "propos_deplaces",
  "photo_inappropriee",
  "faux_profil",
  "mineur",
  "harcelement",
  "arnaque",
  "autre",
] as const;

/**
 * Défait tout ce qui liait deux comptes : demandes en attente closes,
 * conversations fermées, fils invalidés dans les deux sens.
 *
 * Rien n'est supprimé de force — les messages déjà échangés restent lisibles
 * par celui qui bloque jusqu'à la purge, et une suppression immédiate
 * effacerait aussi les preuves d'un comportement qu'on vient de signaler.
 */
async function couperEntre(a: string, b: string): Promise<void> {
  const maintenant = new Date();
  const purge = new Date(maintenant.getTime() + MESSAGE_RETENTION_DAYS * 24 * 60 * 60 * 1000);

  await prisma.$transaction(async (tx) => {
    await tx.joinRequest.updateMany({
      where: {
        state: "envoyee",
        OR: [
          { authorId: a, plan: { authorId: b } },
          { authorId: b, plan: { authorId: a } },
        ],
      },
      data: { state: "expiree", decidedAt: maintenant },
    });

    const conversations = await tx.conversation.findMany({
      where: {
        closedAt: null,
        OR: [
          { hostId: a, guestId: b },
          { hostId: b, guestId: a },
        ],
      },
      select: { id: true },
    });
    if (conversations.length === 0) return;

    const ids = conversations.map((c) => c.id);
    await tx.conversation.updateMany({
      where: { id: { in: ids } },
      data: { closedAt: maintenant, closedBy: a },
    });
    await tx.message.updateMany({
      where: { conversationId: { in: ids } },
      data: { purgeAfter: purge },
    });
  });

  await Promise.all([invalidateFeed(a), invalidateFeed(b)]);
  await Promise.all([publishLiveActivityState(a), publishLiveActivityState(b)]);
}

export const moderationRoutes = new Elysia({ prefix: "/v1", tags: ["Sécurité"] })
  .use(authPlugin)

  .post(
    "/blocks",
    async ({ requireAccount, body }) => {
      const account = requireAccount();
      if (body.accountId === account.id)
        throw invalid("Vous ne pouvez pas vous bloquer vous-même.");

      await prisma.block.upsert({
        where: { authorId_targetId: { authorId: account.id, targetId: body.accountId } },
        create: { authorId: account.id, targetId: body.accountId },
        update: {},
      });

      await couperEntre(account.id, body.accountId);

      return { ok: true };
    },
    {
      body: t.Object({ accountId: t.String() }),
      detail: { summary: "Bloquer un compte" },
    },
  )

  .delete(
    "/blocks/:accountId",
    async ({ requireAccount, params }) => {
      const account = requireAccount();
      await prisma.block.deleteMany({
        where: { authorId: account.id, targetId: params.accountId },
      });
      await invalidateFeed(account.id);
      return { ok: true };
    },
    {
      params: t.Object({ accountId: t.String() }),
      detail: { summary: "Lever un blocage" },
    },
  )

  .post(
    "/reports",
    async ({ requireAccount, body }) => {
      const account = requireAccount();
      await consume(RULES.report, account.id);

      await prisma.report.create({
        data: {
          authorId: account.id,
          targetId: body.accountId,
          reason: body.reason,
          details: body.details ?? "",
        },
      });

      // Un signalement bloque d'office : la personne n'a pas à revoir les plans
      // de qui elle vient de signaler pendant que l'équipe examine le dossier.
      await prisma.block.upsert({
        where: { authorId_targetId: { authorId: account.id, targetId: body.accountId } },
        create: { authorId: account.id, targetId: body.accountId },
        update: {},
      });
      await couperEntre(account.id, body.accountId);

      return { ok: true };
    },
    {
      body: t.Object({
        accountId: t.String(),
        reason: t.Union(REASONS.map((reason) => t.Literal(reason))),
        details: t.Optional(t.String({ maxLength: 1000 })),
      }),
      detail: {
        summary: "Signaler un compte",
        description:
          "Le signalement entraîne un blocage immédiat, sans notification à l'autre partie.",
      },
    },
  )

  .post(
    "/me/pause",
    async ({ requireAccount, body }) => {
      const account = requireAccount();
      const paused = body.paused;

      await prisma.account.update({
        where: { id: account.id },
        data: { status: paused ? "paused" : "active" },
      });

      // En pause, ses plans ouverts sortent du fil des autres : rien ne sert de
      // laisser visible un rendez-vous auquel on ne répondra pas.
      if (paused) {
        await prisma.plan.updateMany({
          where: { authorId: account.id, state: "ouvert" },
          data: { state: "annule", cancelledAt: new Date() },
        });
        await prisma.joinRequest.updateMany({
          where: { state: "envoyee", plan: { authorId: account.id } },
          data: { state: "expiree", decidedAt: new Date() },
        });
      }

      await invalidateAccountCache(account.id);
      await invalidateFeed(account.id);
      await publishLiveActivityState(account.id);

      return { ok: true, status: paused ? "paused" : "active" };
    },
    {
      body: t.Object({ paused: t.Boolean() }),
      detail: {
        summary: "Mettre son compte en pause",
        description:
          "En pause, vos plans ouverts sont retirés et vous n'apparaissez plus dans le fil.",
      },
    },
  );
