/**
 * Les conversations : ce qui s'ouvre quand une demande est acceptée.
 *
 * Une conversation est toujours à deux, même sur un plan de groupe — on parle à
 * quelqu'un, pas à une salle. Elle naît d'une acceptation et se ferme à la
 * main ; il n'existe aucun moyen d'en ouvrir une autrement, ni d'écrire à
 * quelqu'un qui n'a pas dit oui.
 *
 * Pas d'accusé de lecture visible par l'autre, pas d'indicateur de frappe, pas
 * de « vu à ». Ces mécaniques servent à retenir, pas à se parler.
 */
import { Elysia, t } from "elysia";
import { MESSAGE_RETENTION_DAYS, type Conversation, type Message } from "@weave/contracts";
import { forbidden, invalid, notFound } from "../lib/errors.ts";
import { prisma } from "../lib/prisma.ts";
import { ageFrom } from "../lib/time.ts";
import { authPlugin } from "../plugins/auth.ts";
import { consume, RULES } from "../plugins/rate-limit.ts";
import { publishLiveActivityState } from "./live-activity.service.ts";
import { photoSignee } from "./plans.service.ts";

const MESSAGE_MAX_CHARS = 2000;

/** Page de messages renvoyée par lecture. */
const PAGE = 50;

/** Ce qu'on affiche de l'autre dans une conversation : le strict nécessaire. */
const PROFIL_BREF = {
  select: {
    id: true,
    displayName: true,
    birthDate: true,
    verified: true,
    profile: { select: { photoKey: true } },
  },
} as const;
export const conversationRoutes = new Elysia({
  prefix: "/v1/conversations",
  tags: ["Conversations"],
})
  .use(authPlugin)

  .get(
    "/",
    async ({ requireAccount }): Promise<Conversation[]> => {
      const compte = requireAccount();

      const lignes = await prisma.conversation.findMany({
        where: { OR: [{ hostId: compte.id }, { guestId: compte.id }] },
        orderBy: [{ lastMessageAt: "desc" }, { openedAt: "desc" }],
        take: 100,
        include: {
          plan: { select: { title: true, startsAt: true } },
          host: PROFIL_BREF,
          guest: PROFIL_BREF,
          messages: { orderBy: { sentAt: "desc" }, take: 1, select: { body: true } },
          _count: {
            select: {
              messages: { where: { readAt: null, authorId: { not: compte.id } } },
            },
          },
        },
      });

      return lignes.map((ligne) => {
        const autre = ligne.hostId === compte.id ? ligne.guest : ligne.host;
        return {
          id: ligne.id,
          planId: ligne.planId,
          planTitle: ligne.plan.title,
          planStartsAt: ligne.plan.startsAt.toISOString(),
          other: {
            id: autre.id,
            displayName: autre.displayName,
            age: ageFrom(autre.birthDate),
            photoUrl: photoSignee(autre.profile?.photoKey ?? null),
            verified: autre.verified,
          },
          lastMessage: ligne.messages[0]?.body ?? null,
          lastMessageAt: ligne.lastMessageAt?.toISOString() ?? null,
          unread: ligne._count.messages,
          closed: ligne.closedAt !== null,
        };
      });
    },
    { detail: { summary: "Mes conversations" } },
  )

  .get(
    "/:id/messages",
    async ({ requireAccount, params, query }): Promise<{ messages: Message[] }> => {
      const compte = requireAccount();
      const conversation = await conversationDe(params.id, compte.id);

      const lignes = await prisma.message.findMany({
        where: {
          conversationId: conversation.id,
          ...(query.before !== undefined ? { sentAt: { lt: new Date(query.before) } } : {}),
        },
        orderBy: { sentAt: "desc" },
        take: PAGE,
      });

      // Ouvrir la conversation marque comme lus les messages de l'autre : le
      // marquage n'est jamais exposé à l'expéditeur, il ne sert qu'au compteur
      // local et à la Live Activity.
      await prisma.message.updateMany({
        where: { conversationId: conversation.id, authorId: { not: compte.id }, readAt: null },
        data: { readAt: new Date() },
      });

      return {
        messages: lignes.reverse().map((ligne) => ({
          id: ligne.id,
          conversationId: ligne.conversationId,
          author: ligne.authorId === compte.id ? "moi" : "autre",
          body: ligne.body,
          sentAt: ligne.sentAt.toISOString(),
          readAt: ligne.readAt?.toISOString() ?? null,
        })),
      };
    },
    {
      params: t.Object({ id: t.String() }),
      query: t.Object({ before: t.Optional(t.String()) }),
      detail: { summary: "Lire une conversation" },
    },
  )

  .post(
    "/:id/messages",
    async ({ requireAccount, params, body }): Promise<Message> => {
      const compte = requireAccount();
      await consume(RULES.message, compte.id);

      const conversation = await conversationDe(params.id, compte.id);
      if (conversation.closedAt !== null) throw invalid("Cette conversation est close.");

      const texte = body.body.trim();
      if (texte.length === 0) throw invalid("Un message vide ne dit rien.");

      const message = await prisma.$transaction(async (tx) => {
        const cree = await tx.message.create({
          data: { conversationId: conversation.id, authorId: compte.id, body: texte },
        });
        await tx.conversation.update({
          where: { id: conversation.id },
          data: { lastMessageAt: cree.sentAt },
        });
        return cree;
      });

      const destinataire =
        conversation.hostId === compte.id ? conversation.guestId : conversation.hostId;
      await publishLiveActivityState(destinataire);

      return {
        id: message.id,
        conversationId: message.conversationId,
        author: "moi",
        body: message.body,
        sentAt: message.sentAt.toISOString(),
        readAt: null,
      };
    },
    {
      params: t.Object({ id: t.String() }),
      body: t.Object({ body: t.String({ minLength: 1, maxLength: MESSAGE_MAX_CHARS }) }),
      detail: { summary: "Écrire dans une conversation" },
    },
  )

  .delete(
    "/:id",
    async ({ requireAccount, params }) => {
      const compte = requireAccount();
      const conversation = await conversationDe(params.id, compte.id);
      if (conversation.closedAt !== null) return { ok: true };

      const fermeture = new Date();
      const purge = new Date(fermeture.getTime() + MESSAGE_RETENTION_DAYS * 24 * 60 * 60 * 1000);

      await prisma.$transaction(async (tx) => {
        await tx.conversation.update({
          where: { id: conversation.id },
          data: { closedAt: fermeture, closedBy: compte.id },
        });
        // Les messages d'une conversation close ne sont pas gardés
        // indéfiniment : la date de purge est posée dès la clôture.
        await tx.message.updateMany({
          where: { conversationId: conversation.id },
          data: { purgeAfter: purge },
        });
      });

      await publishLiveActivityState(compte.id);
      return { ok: true };
    },
    {
      params: t.Object({ id: t.String() }),
      detail: {
        summary: "Clore une conversation",
        description: `Les messages sont purgés ${MESSAGE_RETENTION_DAYS} jours après la clôture.`,
      },
    },
  );

/** Charge une conversation en vérifiant qu'on en fait partie. */
async function conversationDe(id: string, accountId: string) {
  const conversation = await prisma.conversation.findUnique({
    where: { id },
    select: { id: true, hostId: true, guestId: true, closedAt: true },
  });
  if (conversation === null) throw notFound("Conversation introuvable.");
  if (conversation.hostId !== accountId && conversation.guestId !== accountId) {
    throw forbidden("Cette conversation n'est pas la vôtre.");
  }
  return conversation;
}
