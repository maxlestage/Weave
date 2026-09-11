/**
 * Les demandes : écrire à quelqu'un pour venir à son plan, et répondre à celles
 * qu'on reçoit.
 *
 * C'est ici que tient l'invariant central de Weave : **on ne peut pas
 * arroser**. Chaque demande coûte une unité d'un quota journalier borné, à tous
 * les paliers sans exception. Le quota vit dans Redis et se décrémente de façon
 * atomique ; une demande retirée avant d'avoir été lue est rendue, une demande
 * déjà lue ne l'est pas.
 *
 * Il n'existe aucune route pour demander sans écrire. Le message est
 * obligatoire et sa longueur minimale n'est pas décorative : elle est ce qui
 * distingue une demande d'un geste.
 */
import { Elysia, t } from "elysia";
import { REQUEST_MAX_CHARS, REQUEST_MIN_CHARS, type JoinRequest } from "@weave/contracts";
import { consumeRequest, invalidateFeed, refundRequest, requestsLeft } from "../lib/cache.ts";
import {
  alreadyRequested,
  forbidden,
  invalid,
  noRequestsLeft,
  notFound,
  planClosed,
} from "../lib/errors.ts";
import { prisma } from "../lib/prisma.ts";
import { ageFrom, localDay, secondsUntilMidnight } from "../lib/time.ts";
import { authPlugin } from "../plugins/auth.ts";
import { consume, RULES } from "../plugins/rate-limit.ts";
import { entitlementsFor } from "./entitlements.ts";
import { publishLiveActivityState, startLiveActivitiesFor } from "./live-activity.service.ts";
import { photoSignee } from "./plans.service.ts";

/** Projection commune : une demande vue par son auteur. */
type LigneDemande = {
  id: string;
  planId: string;
  message: string;
  state: string;
  sentAt: Date;
  decidedAt: Date | null;
  plan: {
    title: string;
    startsAt: Date;
    author: {
      id: string;
      displayName: string;
      birthDate: Date;
      verified: boolean;
      profile: { photoKey: string | null } | null;
    };
  };
  conversation: { id: string } | null;
};

function versDemande(ligne: LigneDemande): JoinRequest {
  return {
    id: ligne.id,
    planId: ligne.planId,
    planTitle: ligne.plan.title,
    planStartsAt: ligne.plan.startsAt.toISOString(),
    author: {
      id: ligne.plan.author.id,
      displayName: ligne.plan.author.displayName,
      age: ageFrom(ligne.plan.author.birthDate),
      photoUrl: photoSignee(ligne.plan.author.profile?.photoKey ?? null),
      verified: ligne.plan.author.verified,
    },
    message: ligne.message,
    state: ligne.state as JoinRequest["state"],
    sentAt: ligne.sentAt.toISOString(),
    decidedAt: ligne.decidedAt?.toISOString() ?? null,
    conversationId: ligne.conversation?.id ?? null,
  };
}

const INCLUDE_DEMANDE = {
  plan: {
    select: {
      title: true,
      startsAt: true,
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
  },
  conversation: { select: { id: true } },
} as const;

export const requestRoutes = new Elysia({ prefix: "/v1/requests", tags: ["Demandes"] })
  .use(authPlugin)

  .post(
    "/",
    async ({ requireAccount, body }) => {
      const compte = requireAccount();
      await consume(RULES.join, compte.id);

      const message = body.message.trim();
      if (message.length < REQUEST_MIN_CHARS) {
        throw invalid(
          `Écrivez au moins ${REQUEST_MIN_CHARS} caractères : c'est ce qui distingue une demande d'un geste.`,
        );
      }

      const plan = await prisma.plan.findUnique({
        where: { id: body.planId },
        select: {
          id: true,
          authorId: true,
          state: true,
          startsAt: true,
          capacity: true,
          requests: { where: { state: "acceptee" }, select: { id: true } },
          author: { select: { status: true } },
        },
      });

      if (plan === null) throw notFound("Ce plan n'existe plus.");
      if (plan.authorId === compte.id) throw invalid("C'est votre propre plan.");
      if (plan.state !== "ouvert" || plan.author.status !== "active") {
        throw planClosed();
      }
      if (plan.startsAt.getTime() <= Date.now()) {
        throw planClosed("Ce plan a déjà eu lieu.");
      }
      if (plan.requests.length >= plan.capacity) {
        throw planClosed("Ce plan est complet.");
      }

      // Un blocage dans un sens ou dans l'autre rend la demande impossible,
      // sans dire lequel : on ne renseigne jamais quelqu'un sur son blocage.
      const blocage = await prisma.block.findFirst({
        where: {
          OR: [
            { authorId: compte.id, targetId: plan.authorId },
            { authorId: plan.authorId, targetId: compte.id },
          ],
        },
        select: { id: true },
      });
      if (blocage !== null) throw notFound("Ce plan n'existe plus.");

      const deja = await prisma.joinRequest.findUnique({
        where: { planId_authorId: { planId: plan.id, authorId: compte.id } },
        select: { state: true },
      });
      if (deja !== null) throw alreadyRequested();

      // Le quota est prélevé AVANT l'écriture : si l'insertion échoue, la
      // demande est rendue. L'inverse laisserait une demande écrite gratuite.
      const droits = entitlementsFor(compte.tier);
      const jour = localDay(compte.timezone);
      const restantes = await consumeRequest(
        compte.id,
        jour,
        droits.requestsPerDay,
        secondsUntilMidnight(compte.timezone),
      );
      if (restantes === null) throw noRequestsLeft(droits.requestsPerDay);

      let demande;
      try {
        demande = await prisma.joinRequest.create({
          data: { planId: plan.id, authorId: compte.id, message },
          select: { id: true, sentAt: true },
        });
      } catch (erreur) {
        await refundRequest(compte.id, jour);
        throw erreur;
      }

      await invalidateFeed(compte.id);
      // L'auteur du plan voit la demande arriver sur son écran verrouillé ;
      // c'est le seul moment où Weave démarre une Live Activity de lui-même.
      await startLiveActivitiesFor(plan.authorId);
      await publishLiveActivityState(compte.id);

      return {
        id: demande.id,
        sentAt: demande.sentAt.toISOString(),
        requestsLeftToday: restantes,
      };
    },
    {
      body: t.Object({
        planId: t.String(),
        message: t.String({ minLength: REQUEST_MIN_CHARS, maxLength: REQUEST_MAX_CHARS }),
      }),
      detail: {
        summary: "Demander à venir",
        description:
          "Le message est obligatoire. Chaque demande consomme une unité du quota journalier, à tous les paliers.",
      },
    },
  )

  .get(
    "/sent",
    async ({ requireAccount }): Promise<{ requests: JoinRequest[]; requestsLeftToday: number }> => {
      const compte = requireAccount();
      const droits = entitlementsFor(compte.tier);

      const [lignes, restantes] = await Promise.all([
        prisma.joinRequest.findMany({
          where: { authorId: compte.id },
          orderBy: { sentAt: "desc" },
          take: 50,
          include: INCLUDE_DEMANDE,
        }),
        requestsLeft(compte.id, localDay(compte.timezone), droits.requestsPerDay),
      ]);

      return {
        requests: lignes.map((ligne) => versDemande(ligne as LigneDemande)),
        requestsLeftToday: restantes,
      };
    },
    {
      detail: {
        summary: "Mes demandes envoyées",
        description: "Sans accusé de lecture : savoir si l'autre a lu n'aide personne à décider.",
      },
    },
  )

  .delete(
    "/:id",
    async ({ requireAccount, params }) => {
      const compte = requireAccount();

      const demande = await prisma.joinRequest.findUnique({
        where: { id: params.id },
        select: { authorId: true, state: true, sentAt: true },
      });
      if (demande === null) throw notFound("Demande introuvable.");
      if (demande.authorId !== compte.id) throw forbidden("Cette demande n'est pas la vôtre.");
      if (demande.state !== "envoyee") {
        throw invalid("Cette demande a déjà reçu une réponse.");
      }

      await prisma.joinRequest.update({
        where: { id: params.id },
        data: { state: "retiree", decidedAt: new Date() },
      });

      // Une demande retirée rend son unité : se raviser vite ne doit pas coûter
      // la journée. Le jour est celui de l'envoi, pas celui du retrait, sans
      // quoi un retrait après minuit créditerait une journée qu'on n'a pas
      // entamée.
      await refundRequest(compte.id, localDay(compte.timezone, demande.sentAt));
      await publishLiveActivityState(compte.id);

      return { ok: true };
    },
    {
      params: t.Object({ id: t.String() }),
      detail: {
        summary: "Retirer une demande",
        description: "L'unité de quota est rendue.",
      },
    },
  )

  .post(
    "/:id/accept",
    async ({ requireAccount, params }) => {
      const compte = requireAccount();

      const demande = await prisma.joinRequest.findUnique({
        where: { id: params.id },
        select: {
          id: true,
          authorId: true,
          state: true,
          planId: true,
          plan: {
            select: {
              authorId: true,
              capacity: true,
              state: true,
              requests: { where: { state: "acceptee" }, select: { id: true } },
            },
          },
        },
      });
      if (demande === null) throw notFound("Demande introuvable.");
      if (demande.plan.authorId !== compte.id) throw forbidden("Ce plan n'est pas le vôtre.");
      if (demande.state !== "envoyee") throw invalid("Cette demande est déjà tranchée.");
      if (demande.plan.state !== "ouvert") throw planClosed();
      if (demande.plan.requests.length >= demande.plan.capacity) {
        throw planClosed("Toutes les places sont prises.");
      }

      const restantApres = demande.plan.capacity - demande.plan.requests.length - 1;

      const conversation = await prisma.$transaction(async (tx) => {
        // La transaction porte l'invariant de capacité : le filtre sur `state`
        // garantit qu'une seule des deux acceptations concurrentes passe.
        const accepte = await tx.joinRequest.updateMany({
          where: { id: demande.id, state: "envoyee" },
          data: { state: "acceptee", decidedAt: new Date() },
        });
        if (accepte.count !== 1) throw invalid("Cette demande est déjà tranchée.");

        if (restantApres === 0) {
          await tx.plan.update({ where: { id: demande.planId }, data: { state: "complet" } });
          // Le plan ne prend plus personne : les demandes encore en attente
          // n'ont plus d'objet.
          await tx.joinRequest.updateMany({
            where: { planId: demande.planId, state: "envoyee" },
            data: { state: "expiree", decidedAt: new Date() },
          });
        }

        return tx.conversation.create({
          data: {
            planId: demande.planId,
            requestId: demande.id,
            hostId: compte.id,
            guestId: demande.authorId,
          },
          select: { id: true },
        });
      });

      await Promise.all([invalidateFeed(compte.id), invalidateFeed(demande.authorId)]);
      await startLiveActivitiesFor(demande.authorId);
      await publishLiveActivityState(compte.id);

      return { ok: true, conversationId: conversation.id };
    },
    {
      params: t.Object({ id: t.String() }),
      detail: {
        summary: "Accepter une demande",
        description:
          "Ouvre une conversation à deux. Quand la dernière place part, les demandes restantes sont closes.",
      },
    },
  )

  .post(
    "/:id/decline",
    async ({ requireAccount, params }) => {
      const compte = requireAccount();

      const demande = await prisma.joinRequest.findUnique({
        where: { id: params.id },
        select: { id: true, authorId: true, state: true, plan: { select: { authorId: true } } },
      });
      if (demande === null) throw notFound("Demande introuvable.");
      if (demande.plan.authorId !== compte.id) throw forbidden("Ce plan n'est pas le vôtre.");
      if (demande.state !== "envoyee") throw invalid("Cette demande est déjà tranchée.");

      await prisma.joinRequest.update({
        where: { id: demande.id },
        data: { state: "refusee", decidedAt: new Date() },
      });

      await publishLiveActivityState(compte.id);
      await publishLiveActivityState(demande.authorId);

      return { ok: true };
    },
    {
      params: t.Object({ id: t.String() }),
      detail: {
        summary: "Refuser une demande",
        description:
          "L'auteur voit sa demande close, sans motif ni notification accusatrice. L'unité de quota reste dépensée : elle a été lue.",
      },
    },
  );
