/**
 * Limitation de débit adossée à Redis.
 *
 * Fenêtre fixe, compteur par sujet (compte authentifié, sinon adresse IP). Le
 * cache étant déjà une dépendance dure du produit, aucune bibliothèque
 * supplémentaire n'est nécessaire.
 */
import { keys } from "../lib/cache.ts";
import { rateLimited } from "../lib/errors.ts";
import { redis } from "../lib/redis.ts";

export interface RateLimitRule {
  /** Nom du seau : sert de préfixe de clé et de libellé de journalisation. */
  bucket: string;
  /** Nombre d'actions autorisées par fenêtre. */
  limit: number;
  /** Durée de la fenêtre, en secondes. */
  windowSeconds: number;
}

export interface RateLimitState {
  remaining: number;
  resetInSeconds: number;
}

/** Incrémente le compteur et lève une erreur 429 si le seuil est franchi. */
export async function consume(rule: RateLimitRule, subject: string): Promise<RateLimitState> {
  const key = keys.rateLimit(rule.bucket, subject);
  const count = Number(await redis.incr(key));

  if (count === 1) {
    await redis.expire(key, rule.windowSeconds);
  }

  const ttl = Number(await redis.ttl(key));
  const resetInSeconds = ttl > 0 ? ttl : rule.windowSeconds;

  if (count > rule.limit) {
    throw rateLimited(
      `Limite atteinte pour « ${rule.bucket} ». Réessayez dans ${resetInSeconds} s.`,
    );
  }

  return { remaining: Math.max(0, rule.limit - count), resetInSeconds };
}

/** Règles appliquées par le service. */
export const RULES = {
  /** Demande de code de connexion : protège la boîte mail et le coût d'envoi. */
  otpRequest: { bucket: "otp-request", limit: 5, windowSeconds: 15 * 60 },
  /** Vérification du code : ralentit une attaque par force brute. */
  otpVerify: { bucket: "otp-verify", limit: 10, windowSeconds: 15 * 60 },
  /**
   * Lecture du métier. C'est une lecture de cache, appelée à chaque ouverture
   * de l'application, au retour d'arrière-plan et par la montre : la limite est
   * là contre l'emballement d'un client, pas contre l'usage normal. La
   * composition elle-même est de toute façon bornée par le plafond de trois
   * fils et par le délai de regarnissage.
   */
  weave: { bucket: "weave", limit: 240, windowSeconds: 60 * 60 },
  /** Envoi de messages : borne haute très large, uniquement anti-abus. */
  message: { bucket: "message", limit: 240, windowSeconds: 60 * 60 },
  /** Signalements : évite le harcèlement par signalement en masse. */
  report: { bucket: "report", limit: 20, windowSeconds: 24 * 60 * 60 },
} as const satisfies Record<string, RateLimitRule>;
