/**
 * Types de transport partagés entre l'API Elysia, le site React et le client
 * Swift, dont les structures `Codable` sont le miroir exact.
 */

import type { PlanCategory } from "./invariants.ts";
import type { PlanTier, UnitSku } from "./catalog.ts";

/* ------------------------------------------------------------------ */
/* Identité                                                            */
/* ------------------------------------------------------------------ */

export type AccountStatus = "onboarding" | "active" | "paused" | "suspended" | "deleting";

export interface Session {
  readonly accessToken: string;
  readonly refreshToken: string;
  readonly expiresAt: string;
}

export interface Me {
  readonly id: string;
  readonly handle: string;
  readonly displayName: string;
  readonly age: number;
  readonly status: AccountStatus;
  readonly tier: PlanTier;
  readonly city: string;
  readonly bio: string;
  readonly photoUrl: string | null;
  readonly verified: boolean;
  /** Demandes restantes aujourd'hui. */
  readonly requestsLeftToday: number;
  readonly credits: Readonly<Record<UnitSku, number>>;
  readonly createdAt: string;
}

/** Auteur d'un plan, tel qu'affiché dans le fil. Volontairement maigre. */
export interface Author {
  readonly id: string;
  readonly displayName: string;
  readonly age: number;
  readonly photoUrl: string | null;
  readonly verified: boolean;
}

/* ------------------------------------------------------------------ */
/* Plans                                                               */
/* ------------------------------------------------------------------ */

export type PlanState =
  | "ouvert" /* publié, des places restent */
  | "complet" /* toutes les places sont prises */
  | "passe" /* l'heure du rendez-vous est dépassée */
  | "annule";

export interface Plan {
  readonly id: string;
  readonly author: Author;
  readonly title: string;
  readonly note: string;
  readonly category: PlanCategory;
  /** Date et heure du rendez-vous, ISO 8601. */
  readonly startsAt: string;
  readonly city: string;
  /** Distance arrondie en kilomètres. Weave n'expose jamais de position précise. */
  readonly distanceKm: number;
  /** Nombre de personnes attendues en plus de l'auteur. */
  readonly capacity: number;
  readonly seatsLeft: number;
  readonly state: PlanState;
  /** Vrai si la personne qui consulte a déjà demandé à venir. */
  readonly requested: boolean;
  /** Renseigné pour ses propres plans : nombre de demandes reçues non traitées. */
  readonly pendingRequests?: number;
  readonly createdAt: string;
}

/** Le fil : les plans à venir, autour de soi. */
export interface Feed {
  readonly plans: readonly Plan[];
  /** Demandes restantes aujourd'hui, pour l'afficher sans second appel. */
  readonly requestsLeftToday: number;
  /** Vrai si le fil vient du cache chaud. */
  readonly fromCache: boolean;
  readonly generatedAt: string;
}

/* ------------------------------------------------------------------ */
/* Demandes                                                            */
/* ------------------------------------------------------------------ */

export type RequestState = "envoyee" | "acceptee" | "refusee" | "expiree" | "retiree";

export interface JoinRequest {
  readonly id: string;
  readonly planId: string;
  readonly planTitle: string;
  readonly planStartsAt: string;
  readonly author: Author;
  readonly message: string;
  readonly state: RequestState;
  readonly sentAt: string;
  readonly decidedAt: string | null;
  /** Identifiant de conversation, une fois la demande acceptée. */
  readonly conversationId: string | null;
}

/* ------------------------------------------------------------------ */
/* Conversations                                                       */
/* ------------------------------------------------------------------ */

export type MessageAuthor = "moi" | "autre" | "systeme";

export interface Message {
  readonly id: string;
  readonly conversationId: string;
  readonly author: MessageAuthor;
  readonly body: string;
  readonly sentAt: string;
  readonly readAt: string | null;
}

export interface Conversation {
  readonly id: string;
  readonly planId: string;
  readonly planTitle: string;
  readonly planStartsAt: string;
  readonly other: Author;
  readonly lastMessage: string | null;
  readonly lastMessageAt: string | null;
  readonly unread: number;
  readonly closed: boolean;
}

/* ------------------------------------------------------------------ */
/* Live Activity et Apple Watch                                        */
/* ------------------------------------------------------------------ */

/**
 * État dynamique de la Live Activity. Miroir exact de
 * `WeaveActivityAttributes.ContentState` côté Swift.
 *
 * Rien de ce qui s'affiche ici ne doit trahir avec qui l'on parle : un titre de
 * plan, des compteurs, une échéance. Un écran verrouillé se lit par-dessus
 * l'épaule.
 */
export interface LiveActivityState {
  /** Plan le plus proche, publié ou rejoint. */
  readonly planTitle: string | null;
  readonly planStartsAt: string | null;
  /** Demandes reçues et non traitées sur ses propres plans. */
  readonly pendingRequests: number;
  /** Demandes envoyées et encore sans réponse. */
  readonly awaitingReply: number;
  readonly updatedAt: string;
}

/** Charge utile compacte destinée à watchOS. */
export interface WatchSummary {
  readonly pendingRequests: number;
  readonly awaitingReply: number;
  readonly nextPlan: {
    readonly title: string;
    readonly startsAt: string;
    readonly city: string;
  } | null;
  readonly generatedAt: string;
}

/* ------------------------------------------------------------------ */
/* Facturation                                                         */
/* ------------------------------------------------------------------ */

export interface Entitlement {
  readonly tier: PlanTier;
  readonly renewsAt: string | null;
  readonly credits: Readonly<Record<UnitSku, number>>;
  readonly requestsLeftToday: number;
  readonly inGracePeriod: boolean;
}
