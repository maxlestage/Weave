/**
 * Conversations tissées.
 *
 * Un fil ne devient une conversation persistée qu'au deuxième échange, quand
 * les deux personnes ont répondu. Avant cela, il n'existe qu'en cache.
 */
import { Elysia, t } from "elysia";
import {
  MESSAGE_RETENTION_DAYS,
  REVEAL_STEPS,
  RESPONSE_MAX_CHARS,
  type Message,
} from "@weave/contracts";
import { forbidden, notFound } from "../lib/errors.ts";
import { prisma } from "../lib/prisma.ts";
import { authPlugin } from "../plugins/auth.ts";
import { consume, RULES } from "../plugins/rate-limit.ts";

async function loadThread(accountId: string, threadId: string) {
  const thread = await prisma.wovenThread.findUnique({ where: { id: threadId } });
  if (thread === null) throw notFound("Conversation introuvable.");
  if (thread.initiatorId !== accountId && thread.responderId !== accountId) {
    throw forbidden("Cette conversation ne vous concerne pas.");
  }
  return thread;
}

export const threadRoutes = new Elysia({ prefix: "/v1/threads", tags: ["Conversations"] })
  .use(authPlugin)

  .get(
    "/",
    async ({ requireAccount }) => {
      const account = requireAccount();
      const rows = await prisma.wovenThread.findMany({
        where: {
          OR: [{ initiatorId: account.id }, { responderId: account.id }],
          state: { not: "clos" },
        },
        orderBy: { lastMessageAt: "desc" },
        take: 50,
        select: {
          id: true,
          state: true,
          exchanges: true,
          revealPercent: true,
          wovenAt: true,
          lastMessageAt: true,
          initiator: { select: { id: true, displayName: true } },
          responder: { select: { id: true, displayName: true } },
          messages: { orderBy: { sentAt: "desc" }, take: 1, select: { body: true, sentAt: true } },
        },
      });

      return rows.map((row) => {
        const other = row.initiator.id === account.id ? row.responder : row.initiator;
        return {
          id: row.id,
          state: row.state,
          displayName: other.displayName,
          exchanges: row.exchanges,
          revealPercent: row.revealPercent,
          wovenAt: row.wovenAt.toISOString(),
          lastMessageAt: row.lastMessageAt?.toISOString() ?? null,
          preview: row.messages[0]?.body ?? null,
        };
      });
    },
    { detail: { summary: "Lister ses conversations" } },
  )

  .get(
    "/:id/messages",
    async ({ requireAccount, params, query }) => {
      const account = requireAccount();
      await loadThread(account.id, params.id);

      const rows = await prisma.message.findMany({
        where: {
          threadId: params.id,
          ...(query.before !== undefined ? { sentAt: { lt: new Date(query.before) } } : {}),
        },
        orderBy: { sentAt: "desc" },
        take: query.limit ?? 50,
      });

      // L'accusé de lecture est posé à la lecture réelle, jamais à la réception.
      await prisma.message.updateMany({
        where: { threadId: params.id, authorId: { not: account.id }, readAt: null },
        data: { readAt: new Date() },
      });

      const messages: Message[] = rows.reverse().map((row) => ({
        id: row.id,
        threadId: row.threadId,
        author: row.authorId === account.id ? "moi" : "elle",
        body: row.body,
        sentAt: row.sentAt.toISOString(),
        readAt: row.readAt?.toISOString() ?? null,
        ...(row.durationSeconds !== null ? { durationSeconds: row.durationSeconds } : {}),
      }));

      return messages;
    },
    {
      params: t.Object({ id: t.String() }),
      query: t.Object({
        limit: t.Optional(t.Integer({ minimum: 1, maximum: 100 })),
        before: t.Optional(t.String()),
      }),
      detail: { summary: "Lire les messages d'une conversation" },
    },
  )

  .post(
    "/:id/messages",
    async ({ requireAccount, params, body }) => {
      const account = requireAccount();
      await consume(RULES.message, account.id);
      const thread = await loadThread(account.id, params.id);

      const message = await prisma.message.create({
        data: { threadId: thread.id, authorId: account.id, body: body.body },
      });

      // Chaque échange abouti — une réponse de chaque côté — dévoile un cran
      // supplémentaire de la photo.
      const lastAuthor = await prisma.message.findFirst({
        where: { threadId: thread.id, id: { not: message.id } },
        orderBy: { sentAt: "desc" },
        select: { authorId: true },
      });

      const mutual = lastAuthor !== null && lastAuthor.authorId !== account.id;
      const exchanges = mutual ? thread.exchanges + 1 : thread.exchanges;
      const revealPercent = REVEAL_STEPS[Math.min(exchanges, REVEAL_STEPS.length - 1)]!;

      await prisma.wovenThread.update({
        where: { id: thread.id },
        data: { lastMessageAt: new Date(), exchanges, revealPercent },
      });

      return {
        id: message.id,
        sentAt: message.sentAt.toISOString(),
        exchanges,
        revealPercent,
      };
    },
    {
      params: t.Object({ id: t.String() }),
      body: t.Object({ body: t.String({ minLength: 1, maxLength: RESPONSE_MAX_CHARS }) }),
      detail: { summary: "Envoyer un message" },
    },
  )

  .post(
    "/:id/close",
    async ({ requireAccount, params }) => {
      const account = requireAccount();
      const thread = await loadThread(account.id, params.id);

      const purgeAfter = new Date(Date.now() + MESSAGE_RETENTION_DAYS * 24 * 60 * 60 * 1000);

      await prisma.$transaction(async (tx) => {
        await tx.wovenThread.update({
          where: { id: thread.id },
          data: { state: "clos", closedAt: new Date(), closedBy: account.id },
        });
        // Les messages ne sont pas effacés sur-le-champ : ils restent
        // consultables par l'autre personne, puis sont purgés automatiquement.
        await tx.message.updateMany({ where: { threadId: thread.id }, data: { purgeAfter } });
      });

      return { ok: true, purgeAfter: purgeAfter.toISOString() };
    },
    {
      params: t.Object({ id: t.String() }),
      detail: {
        summary: "Clore une conversation",
        description: `Les messages sont purgés automatiquement après ${MESSAGE_RETENTION_DAYS} jours.`,
      },
    },
  );
