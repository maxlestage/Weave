/**
 * Le métier : lire ses trois fils, y répondre, en dénouer un, le prolonger.
 *
 * Il n'existe volontairement aucune route pour « aimer », « passer » ou
 * « rembobiner ». Un fil s'engage en répondant à un fragment ; il se dénoue en
 * l'expliquant ou en laissant le temps faire.
 */
import { Elysia, t } from "elysia";
import {
  REVEAL_STEPS,
  RESPONSE_MAX_CHARS,
  RESPONSE_MIN_CHARS,
  THREAD_TTL_SECONDS,
  type Loom,
  type ThreadCard,
} from "@weave/contracts";
import { extendThread, readThread, removeThread, updateThread } from "../lib/cache.ts";
import { invalid, threadGone } from "../lib/errors.ts";
import { prisma } from "../lib/prisma.ts";
import { authPlugin } from "../plugins/auth.ts";
import { consume, RULES } from "../plugins/rate-limit.ts";
import { requireCredit } from "./entitlements.ts";
import { clearRefill, getLoom } from "./loom.service.ts";
import { publishLiveActivityState } from "./live-activity.service.ts";

/** Retrouve la ligne de registre d'un fil, en vérifiant qu'il appartient bien à la personne. */
async function ledgerFor(accountId: string, threadId: string) {
  const ledger = await prisma.threadLedger.findUnique({ where: { cacheKey: threadId } });
  if (ledger === null || ledger.viewerId !== accountId) throw threadGone();
  return ledger;
}

export const loomRoutes = new Elysia({ prefix: "/v1/loom", tags: ["Métier"] })
  .use(authPlugin)

  .get(
    "/",
    async ({ requireAccount }): Promise<Loom> => {
      const account = requireAccount();
      await consume(RULES.weave, account.id);
      const loom = await getLoom(account);
      await publishLiveActivityState(account.id, loom);
      return loom;
    },
    {
      detail: {
        summary: "Lire ses fils",
        description:
          "Renvoie au plus trois fils, lus depuis le cache. Si une place est libre et que le délai de regarnissage est écoulé, un nouveau fil est tissé à cette occasion.",
      },
    },
  )

  .post(
    "/threads/:id/respond",
    async ({ requireAccount, params, body }) => {
      const account = requireAccount();
      await consume(RULES.message, account.id);

      const card = await readThread(params.id);
      if (card === null) throw threadGone();
      const ledger = await ledgerFor(account.id, params.id);

      if (!card.awaitingYou) {
        throw invalid("Vous avez déjà répondu ; c'est au tour de l'autre personne.");
      }
      if (!card.fragments.some((fragment) => fragment.id === body.fragmentId)) {
        throw invalid("Ce fragment n'appartient pas à ce fil.");
      }

      const exchanges = card.exchanges + 1;
      const revealIndex = Math.min(exchanges, REVEAL_STEPS.length - 1);

      const updated: ThreadCard = {
        ...card,
        state: exchanges >= 2 ? "tisse" : "engage",
        exchanges,
        revealPercent: REVEAL_STEPS[revealIndex]!,
        awaitingYou: false,
      };

      const stored = await updateThread(updated);
      if (!stored) throw threadGone();

      await prisma.threadLedger.update({
        where: { id: ledger.id },
        data: { outcome: updated.state },
      });

      // Au deuxième échange, le fil est tissé : la conversation devient
      // persistante et sort du régime « cache uniquement ».
      let conversationId: string | null = null;
      if (updated.state === "tisse") {
        const woven = await prisma.wovenThread.upsert({
          where: {
            initiatorId_responderId: {
              initiatorId: ledger.viewerId,
              responderId: ledger.candidateId,
            },
          },
          create: {
            initiatorId: ledger.viewerId,
            responderId: ledger.candidateId,
            exchanges,
            revealPercent: updated.revealPercent,
            lastMessageAt: new Date(),
          },
          update: {
            exchanges,
            revealPercent: updated.revealPercent,
            lastMessageAt: new Date(),
          },
          select: { id: true },
        });
        conversationId = woven.id;

        await prisma.message.create({
          data: { threadId: woven.id, authorId: account.id, body: body.body },
        });
      }

      return { thread: updated, conversationId };
    },
    {
      params: t.Object({ id: t.String() }),
      body: t.Object({
        fragmentId: t.String(),
        body: t.String({ minLength: RESPONSE_MIN_CHARS, maxLength: RESPONSE_MAX_CHARS }),
      }),
      detail: {
        summary: "Répondre à un fragment",
        description:
          "Seule façon d'engager un fil : une réponse écrite. Chaque échange abouti dévoile un cran de la photo.",
      },
    },
  )

  .post(
    "/threads/:id/release",
    async ({ requireAccount, params, body }) => {
      const account = requireAccount();
      const ledger = await ledgerFor(account.id, params.id);

      await removeThread(account.id, params.id);
      await prisma.threadLedger.update({
        where: { id: ledger.id },
        data: { outcome: "relache", resolvedAt: new Date() },
      });

      // Le motif du dénouage n'est jamais transmis à l'autre personne : il ne
      // sert qu'à améliorer la composition et la modération.
      return { ok: true, reason: body.reason ?? null };
    },
    {
      params: t.Object({ id: t.String() }),
      body: t.Object({ reason: t.Optional(t.String({ maxLength: 120 })) }),
      detail: {
        summary: "Dénouer un fil",
        description: "Libère une place sur le métier. Le fil disparaît définitivement du cache.",
      },
    },
  )

  .post(
    "/threads/:id/extend",
    async ({ requireAccount, params }) => {
      const account = requireAccount();
      await ledgerFor(account.id, params.id);
      await requireCredit(account.id, "prolonge");

      const until = await extendThread(account.id, params.id, THREAD_TTL_SECONDS);
      if (until === null) {
        throw invalid("Ce fil a déjà été prolongé au maximum, ou il vient de se dénouer.");
      }
      return { ok: true, expiresAt: until.toISOString() };
    },
    {
      params: t.Object({ id: t.String() }),
      detail: {
        summary: "Prolonger un fil (+24 h)",
        description:
          "Consomme un crédit « Prolonge ». La durée totale d'un fil ne dépasse jamais 48 h.",
      },
    },
  )

  .post(
    "/relais",
    async ({ requireAccount }) => {
      const account = requireAccount();
      await requireCredit(account.id, "relais");

      // Le « Relais » supprime l'attente, jamais le plafond : si les trois fils
      // sont déjà là, il n'y a rien à regarnir.
      await clearRefill(account.id);
      const loom = await getLoom(account);
      await publishLiveActivityState(account.id, loom);
      return loom;
    },
    {
      detail: {
        summary: "Regarnir immédiatement une place libre",
        description:
          "Consomme un crédit « Relais ». N'augmente jamais le nombre de fils : le plafond de trois reste absolu.",
      },
    },
  );
