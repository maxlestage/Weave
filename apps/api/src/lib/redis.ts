/**
 * Accès Redis.
 *
 * Weave n'embarque aucune bibliothèque cliente tierce : Bun expose un client
 * Redis natif (`RedisClient`), déjà instrumenté et sans dépendance à installer.
 * C'est cohérent avec le choix « Bun de bout en bout » et cela réduit la
 * surface de dépendances du service.
 */
import { RedisClient } from "bun";
import { env } from "../env.ts";

export const redis = new RedisClient(env.redis.url, {
  connectionTimeout: 2_000,
  idleTimeout: 0,
  autoReconnect: true,
  maxRetries: 10,
  // Voir `env.redis.tlsInsecure` : certains hébergeurs présentent un
  // certificat auto-signé sur leur réseau interne.
  ...(env.redis.tlsInsecure ? { tls: { rejectUnauthorized: false } } : {}),
});

let ready = false;

redis.onconnect = () => {
  ready = true;
};
redis.onclose = () => {
  ready = false;
};

export function redisReady(): boolean {
  return ready;
}

/**
 * Vérifie la disponibilité du cache, pour la sonde de santé.
 *
 * L'appel est borné dans le temps, et c'est le point essentiel : avec
 * `autoReconnect`, un `PING` adressé à un serveur injoignable n'échoue pas —
 * il est mis en file et attend indéfiniment. La sonde restait alors sans
 * réponse, ce qui est le seul comportement qu'elle ne peut pas se permettre :
 * vu de l'extérieur, un service qui ne répond pas est indiscernable d'un
 * service mort, et l'hébergeur finit par le tuer pour dépassement de délai.
 *
 * Un cache qui ne répond pas en deux secondes est indisponible, quelle que
 * soit la raison.
 */
export async function pingRedis(): Promise<boolean> {
  return withTimeout(
    (async () => {
      const reply = await redis.send("PING", []);
      return reply === "PONG" || reply === "OK";
    })(),
    PROBE_TIMEOUT_MS,
  );
}

/** Délai au-delà duquel une dépendance est déclarée indisponible. */
export const PROBE_TIMEOUT_MS = 2_000;

/**
 * Renvoie `false` si la promesse n'aboutit pas à temps, ou échoue.
 *
 * La promesse perdante n'est pas annulée — on ne peut pas annuler un envoi
 * déjà parti — mais son résultat est ignoré. Le rejet éventuel est absorbé,
 * sans quoi il remonterait plus tard en rejet non traité.
 */
export async function withTimeout(promesse: Promise<boolean>, ms: number): Promise<boolean> {
  let minuteur: ReturnType<typeof setTimeout> | undefined;

  const echeance = new Promise<boolean>((resolve) => {
    minuteur = setTimeout(() => resolve(false), ms);
  });

  try {
    return await Promise.race([promesse.catch(() => false), echeance]);
  } finally {
    if (minuteur !== undefined) clearTimeout(minuteur);
  }
}

/* ------------------------------------------------------------------ */
/* Aides JSON                                                          */
/* ------------------------------------------------------------------ */

/** Lit une valeur JSON. Renvoie `null` si absente ou illisible. */
export async function getJson<T>(key: string): Promise<T | null> {
  const raw = await redis.get(key);
  if (raw === null) return null;
  try {
    return JSON.parse(raw) as T;
  } catch {
    // Une entrée corrompue est traitée comme absente : le cache est reconstructible.
    await redis.del(key);
    return null;
  }
}

/** Écrit une valeur JSON avec une durée de vie obligatoire. */
export async function setJson(key: string, value: unknown, ttlSeconds: number): Promise<void> {
  if (ttlSeconds <= 0) throw new Error("Toute entrée de cache Weave doit porter un TTL positif.");
  await redis.set(key, JSON.stringify(value), "EX", ttlSeconds);
}

/**
 * Écriture conditionnelle : n'écrit que si la clé n'existe pas encore.
 * Renvoie `true` si la valeur a été posée.
 */
export async function setJsonIfAbsent(
  key: string,
  value: unknown,
  ttlSeconds: number,
): Promise<boolean> {
  const reply = await redis.send("SET", [
    key,
    JSON.stringify(value),
    "NX",
    "EX",
    String(ttlSeconds),
  ]);
  return reply === "OK";
}

/** Exécute un script Lua côté serveur, pour les opérations qui doivent être atomiques. */
export async function eval_(script: string, keys: string[], args: string[]): Promise<unknown> {
  return redis.send("EVAL", [script, String(keys.length), ...keys, ...args]);
}

export async function disconnectRedis(): Promise<void> {
  redis.close();
}
