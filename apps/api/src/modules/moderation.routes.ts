/**
 * Sécurité des personnes : blocage et signalement.
 *
 * Un blocage est immédiat et réciproque dans ses effets : les deux comptes
 * cessent d'être proposables l'un à l'autre, et le fil éventuellement en cours
 * disparaît du cache sans notification.
 */
import { Elysia, t } from "elysia";
import { clearLoom, readLoom, removeThread } from "../lib/cache.ts";
import { invalid } from "../lib/errors.ts";
import { prisma } from "../lib/prisma.ts";
import { authPlugin } from "../plugins/auth.ts";
import { consume, RULES } from "../plugins/rate-limit.ts";
import { invalidatePool } from "./loom.service.ts";

const REASONS = [
  "propos_deplaces",
  "photo_inappropriee",
  "faux_profil",
  "mineur",
  "harcelement",
  "arnaque",
  "autre",
] as const;

/** Retire du cache tout fil liant deux comptes, dans les deux sens. */
async function unravelBetween(a: string, b: string): Promise<void> {
  const ledgers = await prisma.threadLedger.findMany({
    where: {
      OR: [
        { viewerId: a, candidateId: b },
        { viewerId: b, candidateId: a },
      ],
    },
    select: { viewerId: true, cacheKey: true },
  });

  for (const ledger of ledgers) {
    await removeThread(ledger.viewerId, ledger.cacheKey);
  }
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

      await unravelBetween(account.id, body.accountId);
      await invalidatePool(account.id);
      await invalidatePool(body.accountId);

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
      await invalidatePool(account.id);
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

      // Un signalement bloque d'office : la personne n'a pas à revoir le profil
      // qu'elle vient de signaler pendant que l'équipe examine le dossier.
      await prisma.block.upsert({
        where: { authorId_targetId: { authorId: account.id, targetId: body.accountId } },
        create: { authorId: account.id, targetId: body.accountId },
        update: {},
      });
      await unravelBetween(account.id, body.accountId);
      await invalidatePool(account.id);

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

      // En pause, on ne propose plus et on n'est plus proposé : le métier est vidé.
      if (paused) await clearLoom(account.id);

      const { invalidateAccountCache } = await import("../plugins/auth.ts");
      await invalidateAccountCache(account.id);

      return {
        ok: true,
        status: paused ? "paused" : "active",
        threads: await readLoom(account.id),
      };
    },
    {
      body: t.Object({ paused: t.Boolean() }),
      detail: {
        summary: "Mettre son compte en pause",
        description: "En pause, vous ne recevez plus de fils et n'êtes plus proposé.",
      },
    },
  );
