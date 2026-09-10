/**
 * Types de transport partagés entre l'API Elysia, le site React et le client
 * Swift (dont les structures `Codable` sont le miroir exact — voir
 * `apps/ios/WeaveKit/Sources/WeaveKit/Models`).
 */

import type { PlanTier, UnitSku } from "./catalog.ts";

/* ------------------------------------------------------------------ */
/* Identité                                                            */
/* ------------------------------------------------------------------ */

export type AccountStatus = "onboarding" | "active" | "paused" | "suspended" | "deleting";

export interface Session {
  readonly accessToken: string;
  readonly refreshToken: string;
  /** Expiration de l'access token, ISO 8601. */
  readonly expiresAt: string;
}

export interface Me {
  readonly id: string;
  readonly handle: string;
  readonly displayName: string;
  readonly status: AccountStatus;
  readonly plan: PlanTier;
  /** Heure de tissage locale choisie (0-23). */
  readonly weavingHour: number;
  readonly timezone: string;
  readonly verified: boolean;
  /** Motif : cinq mots-clés dérivés des réponses de la personne. */
  readonly motif: readonly string[];
  readonly credits: Readonly<Record<UnitSku, number>>;
  readonly createdAt: string;
}

/* ------------------------------------------------------------------ */
/* Fils (uniquement en cache)                                          */
/* ------------------------------------------------------------------ */

export type FragmentKind = "question" | "voix" | "motif";

export interface Fragment {
  readonly id: string;
  readonly kind: FragmentKind;
  /** Intitulé affiché (la question posée, ou le libellé du motif). */
  readonly prompt: string;
  /** Réponse de la personne proposée. Texte, ou URL signée pour la voix. */
  readonly body: string;
  /** Durée en secondes pour un fragment vocal. */
  readonly durationSeconds?: number;
}

export type ThreadState =
  | "propose" /* tissé, jamais engagé */
  | "engage" /* au moins une réponse envoyée */
  | "tisse" /* réponse mutuelle : la conversation est ouverte */
  | "denoue" /* expiré ou relâché */;

export interface ThreadCard {
  readonly id: string;
  readonly state: ThreadState;
  /** Prénom d'affichage de la personne proposée. */
  readonly displayName: string;
  readonly age: number;
  /** Distance arrondie en kilomètres (jamais de coordonnées exactes). */
  readonly distanceKm: number;
  readonly city: string;
  readonly motif: readonly string[];
  readonly fragments: readonly Fragment[];
  /** Netteté de la photo, en pourcentage : 0, 33, 66 ou 100. */
  readonly revealPercent: number;
  /** URL signée de la photo, floutée côté serveur selon `revealPercent`. */
  readonly photoUrl: string | null;
  /** Expiration du fil, ISO 8601. */
  readonly expiresAt: string;
  /** Nombre d'échanges aboutis (une réponse de chaque côté). */
  readonly exchanges: number;
  /** Vrai si c'est à vous de répondre. */
  readonly awaitingYou: boolean;
}

export interface Loom {
  /** Au plus trois. Garanti par `MAX_ACTIVE_THREADS`. */
  readonly threads: readonly ThreadCard[];
  /** Prochaine heure de tissage, ISO 8601. */
  readonly nextWeavingAt: string;
  /** Places libres sur le métier (3 - fils actifs). */
  readonly freeSlots: number;
  /** Date à laquelle la prochaine place libre sera regarnie, ISO 8601. */
  readonly nextRefillAt: string | null;
  /** Vrai si le contenu provient du cache chaud (toujours vrai en régime normal). */
  readonly fromCache: boolean;
}

/* ------------------------------------------------------------------ */
/* Conversation                                                        */
/* ------------------------------------------------------------------ */

export type MessageAuthor = "moi" | "elle" | "systeme";

export interface Message {
  readonly id: string;
  readonly threadId: string;
  readonly author: MessageAuthor;
  readonly body: string;
  readonly sentAt: string;
  readonly readAt: string | null;
  /** Renseigné pour un message vocal. */
  readonly audioUrl?: string;
  readonly durationSeconds?: number;
}

/* ------------------------------------------------------------------ */
/* Live Activity et Apple Watch                                        */
/* ------------------------------------------------------------------ */

/**
 * État dynamique d'une Live Activity « Métier ». Miroir exact de
 * `WeaveActivityAttributes.ContentState` côté Swift.
 */
export interface LiveActivityState {
  readonly activeThreads: number;
  readonly awaitingYou: number;
  /** Expiration du fil le plus proche de se dénouer, ISO 8601. */
  readonly soonestExpiryAt: string | null;
  /** Prénom du fil le plus urgent, pour l'affichage compact. */
  readonly soonestName: string | null;
  readonly nextRefillAt: string | null;
  readonly updatedAt: string;
}

/** Charge utile compacte destinée à watchOS (budget < 4 Ko). */
export interface WatchSummary {
  readonly activeThreads: number;
  readonly awaitingYou: number;
  readonly soonestExpiryAt: string | null;
  readonly entries: readonly {
    readonly id: string;
    readonly name: string;
    readonly expiresAt: string;
    readonly awaitingYou: boolean;
  }[];
  readonly generatedAt: string;
}

/* ------------------------------------------------------------------ */
/* Facturation                                                         */
/* ------------------------------------------------------------------ */

export interface Entitlement {
  readonly plan: PlanTier;
  /** Fin de période courante, ISO 8601, null pour le socle gratuit. */
  readonly renewsAt: string | null;
  readonly credits: Readonly<Record<UnitSku, number>>;
  /** Vrai si l'abonnement est en période de grâce de facturation. */
  readonly inGracePeriod: boolean;
}
