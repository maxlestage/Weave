/**
 * Le métier : les trois fils d'un utilisateur, en cache et rien qu'en cache.
 *
 * C'est le module le plus important du service. Il matérialise l'invariant
 * produit de Weave : un utilisateur détient au plus TROIS fils actifs, et le
 * contenu de ces fils (prénom, motif, fragments, photo) n'existe QUE dans
 * Redis, avec une durée de vie. Rien de tout cela n'est écrit dans la base
 * relationnelle — celle-ci ne conserve qu'un registre d'identifiants
 * (`ThreadLedger`) pour ne jamais reproposer la même personne.
 *
 * Le plafond de trois est appliqué par un script Lua exécuté côté Redis, donc
 * de façon atomique : deux tissages concurrents ne peuvent pas produire un
 * quatrième fil.
 */
import type { ThreadCard } from "@weave/contracts";
import { MAX_ACTIVE_THREADS, THREAD_TTL_MAX_SECONDS } from "@weave/contracts";
import { getJson, redis, setJson } from "./redis.ts";

const NS = "weave:v1";

export const keys = {
  /** ZSET des fils actifs d'un compte, score = date d'expiration en ms. */
  loom: (accountId: string) => `${NS}:loom:${accountId}`,
  /** Carte d'un fil (JSON). Porte son propre TTL. */
  thread: (threadId: string) => `${NS}:thread:${threadId}`,
  /** Identité résumée, pour éviter un aller-retour base à chaque requête. */
  identity: (accountId: string) => `${NS}:me:${accountId}`,
  /** Vivier de candidats pré-calculé pour un compte. */
  pool: (accountId: string) => `${NS}:pool:${accountId}`,
  /** Date à laquelle une place libérée sera regarnie. */
  refill: (accountId: string) => `${NS}:refill:${accountId}`,
  /** Dernier état poussé vers la Live Activity. */
  liveActivity: (accountId: string) => `${NS}:la:${accountId}`,
  /** Résumé compact destiné à watchOS. */
  watch: (accountId: string) => `${NS}:watch:${accountId}`,
  /** Compteur de limitation de débit. */
  rateLimit: (bucket: string, subject: string) => `${NS}:rl:${bucket}:${subject}`,
  /** Question du jour, partagée par tous les comptes d'une même locale. */
  promptOfDay: (locale: string, day: string) => `${NS}:prompt:${locale}:${day}`,
  /** Verrou de tissage, empêche deux compositions simultanées. */
  weaveLock: (accountId: string) => `${NS}:lock:weave:${accountId}`,
} as const;

/**
 * Ajoute un fil au métier si — et seulement si — il reste une place.
 *
 * Le script purge d'abord les fils expirés (score dépassé), puis vérifie le
 * plafond avant d'écrire. L'ensemble est atomique côté Redis.
 *
 * Renvoie le nombre de fils actifs après ajout, ou `null` si le métier est plein.
 */
const ADD_THREAD_LUA = `
local now = tonumber(ARGV[1])
local maximum = tonumber(ARGV[2])
local threadId = ARGV[3]
local payload = ARGV[4]
local ttl = tonumber(ARGV[5])
local expiresAt = tonumber(ARGV[6])
local loomTtl = tonumber(ARGV[7])

redis.call('ZREMRANGEBYSCORE', KEYS[1], '-inf', now)
if redis.call('ZCARD', KEYS[1]) >= maximum then
  return -1
end

redis.call('ZADD', KEYS[1], expiresAt, threadId)
redis.call('EXPIRE', KEYS[1], loomTtl)
redis.call('SET', KEYS[2], payload, 'EX', ttl)
return redis.call('ZCARD', KEYS[1])
`;

export async function addThread(
  accountId: string,
  card: ThreadCard,
  ttlSeconds: number,
): Promise<number | null> {
  const now = Date.now();
  const expiresAt = new Date(card.expiresAt).getTime();
  const reply = await redis.send("EVAL", [
    ADD_THREAD_LUA,
    "2",
    keys.loom(accountId),
    keys.thread(card.id),
    String(now),
    String(MAX_ACTIVE_THREADS),
    card.id,
    JSON.stringify(card),
    String(ttlSeconds),
    String(expiresAt),
    String(THREAD_TTL_MAX_SECONDS),
  ]);
  const count = Number(reply);
  return count < 0 ? null : count;
}

/** Nombre de fils actifs, après purge des expirés. */
export async function activeThreadCount(accountId: string): Promise<number> {
  const key = keys.loom(accountId);
  await redis.send("ZREMRANGEBYSCORE", [key, "-inf", String(Date.now())]);
  return Number(await redis.send("ZCARD", [key]));
}

/** Places disponibles sur le métier (0 à 3). */
export async function freeSlots(accountId: string): Promise<number> {
  return Math.max(0, MAX_ACTIVE_THREADS - (await activeThreadCount(accountId)));
}

/**
 * Lit les fils actifs, du plus proche de l'expiration au plus lointain.
 *
 * Les fils dont la carte a expiré côté Redis sont retirés du métier à la volée :
 * le cache est la seule source de vérité, il n'y a rien à réconcilier ailleurs.
 */
export async function readLoom(accountId: string): Promise<ThreadCard[]> {
  const loomKey = keys.loom(accountId);
  const now = Date.now();
  await redis.send("ZREMRANGEBYSCORE", [loomKey, "-inf", String(now)]);

  const ids = (await redis.send("ZRANGE", [loomKey, "0", "-1"])) as string[] | null;
  if (ids === null || ids.length === 0) return [];

  const payloads = (await redis.send(
    "MGET",
    ids.map((id) => keys.thread(id)),
  )) as (string | null)[];

  const cards: ThreadCard[] = [];
  const orphans: string[] = [];

  ids.forEach((id, index) => {
    const raw = payloads[index];
    if (raw == null) {
      orphans.push(id);
      return;
    }
    try {
      cards.push(JSON.parse(raw) as ThreadCard);
    } catch {
      orphans.push(id);
    }
  });

  if (orphans.length > 0) await redis.send("ZREM", [loomKey, ...orphans]);
  return cards;
}

export async function readThread(threadId: string): Promise<ThreadCard | null> {
  return getJson<ThreadCard>(keys.thread(threadId));
}

/** Remplace la carte d'un fil en conservant sa durée de vie restante. */
export async function updateThread(card: ThreadCard): Promise<boolean> {
  const key = keys.thread(card.id);
  const ttl = Number(await redis.send("TTL", [key]));
  if (ttl <= 0) return false;
  await setJson(key, card, ttl);
  return true;
}

/** Retire un fil du métier (dénouage, blocage, signalement). */
export async function removeThread(accountId: string, threadId: string): Promise<void> {
  await redis.send("ZREM", [keys.loom(accountId), threadId]);
  await redis.del(keys.thread(threadId));
}

/**
 * Prolonge un fil. Le plafond absolu de 48 h est appliqué ici, jamais dépassé,
 * même par achats répétés.
 */
export async function extendThread(
  accountId: string,
  threadId: string,
  addSeconds: number,
): Promise<Date | null> {
  const card = await readThread(threadId);
  if (card === null) return null;

  const current = new Date(card.expiresAt).getTime();
  const ceiling = Date.now() + THREAD_TTL_MAX_SECONDS * 1000;
  const target = Math.min(current + addSeconds * 1000, ceiling);
  if (target <= current) return null;

  const extended: ThreadCard = { ...card, expiresAt: new Date(target).toISOString() };
  const ttl = Math.ceil((target - Date.now()) / 1000);
  await setJson(keys.thread(threadId), extended, ttl);
  await redis.send("ZADD", [keys.loom(accountId), String(target), threadId]);
  return new Date(target);
}

/** Vide entièrement le métier d'un compte (suppression, mise en pause). */
export async function clearLoom(accountId: string): Promise<void> {
  const loomKey = keys.loom(accountId);
  const ids = (await redis.send("ZRANGE", [loomKey, "0", "-1"])) as string[] | null;
  if (ids && ids.length > 0) {
    await redis.send("DEL", ids.map((id) => keys.thread(id)));
  }
  await redis.del(loomKey);
  await redis.del(keys.watch(accountId));
  await redis.del(keys.liveActivity(accountId));
}

/**
 * Verrou court autour d'une composition, pour qu'un client impatient ne
 * déclenche pas deux tissages en parallèle.
 */
export async function withWeaveLock<T>(
  accountId: string,
  ttlSeconds: number,
  run: () => Promise<T>,
): Promise<T | null> {
  const key = keys.weaveLock(accountId);
  const acquired = await redis.send("SET", [key, "1", "NX", "EX", String(ttlSeconds)]);
  if (acquired !== "OK") return null;
  try {
    return await run();
  } finally {
    await redis.del(key);
  }
}
