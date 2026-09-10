/**
 * Le moteur de tissage : compose les fils, et rien d'autre.
 *
 * Ce que ce module NE fait pas, volontairement :
 *  • il ne construit pas de pile de cartes à balayer ;
 *  • il n'expose ni « qui vous a aimé », ni file d'attente de likes reçus ;
 *  • il n'offre aucune mise en avant payante d'un profil dans le vivier ;
 *  • il n'écrit jamais le contenu d'un profil proposé dans la base.
 *
 * Ce qu'il fait : à l'heure de tissage choisie par la personne, il compose au
 * plus trois fils, les dépose en cache avec une durée de vie, et n'enregistre
 * en base qu'une ligne de registre par proposition.
 */
import {
  FRAGMENTS_PER_THREAD,
  MAX_ACTIVE_THREADS,
  MOTIF_TAGS,
  REVEAL_STEPS,
  THREAD_TTL_SECONDS,
  type Fragment,
  type Loom,
  type ThreadCard,
} from "@weave/contracts";
import { env } from "../env.ts";
import { addThread, freeSlots, keys, readLoom, withWeaveLock } from "../lib/cache.ts";
import { signMediaUrl, uuid } from "../lib/crypto.ts";
import { log } from "../lib/log.ts";
import { prisma } from "../lib/prisma.ts";
import { getJson, redis, setJson } from "../lib/redis.ts";
import { ageFrom, distanceKm, nextWeavingAt, refillAt } from "../lib/time.ts";
import type { AuthenticatedAccount } from "../plugins/auth.ts";
import { entitlementsFor } from "./entitlements.ts";

/** Nombre de candidats extraits de la base avant scoring fin en mémoire. */
const CANDIDATE_FETCH_LIMIT = 200;

/** Durée de vie du vivier pré-calculé, en secondes. */
const POOL_TTL_SECONDS = 30 * 60;

interface Candidate {
  accountId: string;
  displayName: string;
  age: number;
  city: string;
  distanceKm: number;
  motif: string[];
  photoKey: string | null;
  fragments: { promptText: string; kind: string; body: string; durationSeconds: number | null }[];
  score: number;
}

/* ------------------------------------------------------------------ */
/* Sélection des candidats                                             */
/* ------------------------------------------------------------------ */

/**
 * Boîte englobante autour d'un point, pour préfiltrer en SQL sans extension
 * géospatiale : le schéma doit rester identique sur PostgreSQL et SQLite.
 * La distance exacte est recalculée ensuite en mémoire.
 */
function boundingBox(lat: number, lon: number, radiusKm: number) {
  const latDelta = radiusKm / 111;
  const lonDelta = radiusKm / Math.max(1, 111 * Math.cos((lat * Math.PI) / 180));
  return {
    minLat: lat - latDelta,
    maxLat: lat + latDelta,
    minLon: lon - lonDelta,
    maxLon: lon + lonDelta,
  };
}

/**
 * Score de composition, sur 1000.
 *
 * Trois termes, tous explicables à la personne : la proximité de motif, la
 * proximité géographique, et la fraîcheur du profil. Aucun terme n'est
 * achetable : un abonnement ne fait pas remonter quelqu'un dans le vivier.
 */
function scoreCandidate(
  viewerMotif: readonly string[],
  candidateMotif: readonly string[],
  distance: number,
  maxDistance: number,
  lastSeenAt: Date | null,
): number {
  const shared = candidateMotif.filter((tag) => viewerMotif.includes(tag)).length;
  const motifScore = (shared / Math.max(1, MOTIF_TAGS)) * 550;

  const proximity = 1 - Math.min(1, distance / Math.max(1, maxDistance));
  const proximityScore = proximity * 300;

  const days =
    lastSeenAt === null ? 30 : (Date.now() - lastSeenAt.getTime()) / (24 * 60 * 60 * 1000);
  const freshnessScore = Math.max(0, 1 - days / 30) * 150;

  return Math.round(motifScore + proximityScore + freshnessScore);
}

/**
 * Construit le vivier de candidats d'une personne. Le résultat est mis en cache
 * une demi-heure : recomposer trois fils ne doit pas coûter une requête lourde
 * à chaque ouverture de l'application.
 */
async function buildCandidatePool(accountId: string): Promise<Candidate[]> {
  const cached = await getJson<Candidate[]>(keys.pool(accountId));
  if (cached !== null) return cached;

  const viewer = await prisma.account.findUnique({
    where: { id: accountId },
    select: {
      id: true,
      birthDate: true,
      profile: {
        select: {
          latRounded: true,
          lonRounded: true,
          city: true,
          motifTags: { select: { tag: true } },
        },
      },
      preference: true,
      blocksMade: { select: { targetId: true } },
      blocksReceived: { select: { authorId: true } },
      servedThreads: { select: { candidateId: true } },
    },
  });

  if (viewer?.profile == null || viewer.preference == null) return [];

  const preference = viewer.preference;
  const viewerMotif = viewer.profile.motifTags.map((t) => t.tag);
  const escaleActive =
    preference.escaleUntil !== null && preference.escaleUntil.getTime() > Date.now();

  const origin = { lat: viewer.profile.latRounded, lon: viewer.profile.lonRounded };
  const box = boundingBox(origin.lat, origin.lon, preference.maxDistanceKm);

  const seeking = safeJsonArray(preference.seekingJson);
  const intents = safeJsonArray(preference.intentsJson);

  const now = new Date();
  const maxBirth = new Date(now.getFullYear() - preference.minAge, now.getMonth(), now.getDate());
  const minBirth = new Date(
    now.getFullYear() - preference.maxAge - 1,
    now.getMonth(),
    now.getDate(),
  );

  const excluded = new Set<string>([
    viewer.id,
    ...viewer.blocksMade.map((b) => b.targetId),
    ...viewer.blocksReceived.map((b) => b.authorId),
    ...viewer.servedThreads.map((t) => t.candidateId),
  ]);

  const rows = await prisma.account.findMany({
    where: {
      id: { notIn: [...excluded] },
      status: "active",
      deletionRequestedAt: null,
      birthDate: { gte: minBirth, lte: maxBirth },
      profile: {
        is: {
          ...(escaleActive
            ? { city: preference.escaleCity! }
            : {
                latRounded: { gte: box.minLat, lte: box.maxLat },
                lonRounded: { gte: box.minLon, lte: box.maxLon },
              }),
          ...(seeking.length > 0 ? { gender: { in: seeking } } : {}),
          ...(intents.length > 0 ? { intent: { in: intents } } : {}),
          photoKey: { not: null },
        },
      },
    },
    take: CANDIDATE_FETCH_LIMIT,
    orderBy: { lastSeenAt: "desc" },
    select: {
      id: true,
      displayName: true,
      birthDate: true,
      lastSeenAt: true,
      profile: {
        select: {
          city: true,
          latRounded: true,
          lonRounded: true,
          photoKey: true,
          motifTags: { select: { tag: true }, orderBy: { weight: "desc" }, take: MOTIF_TAGS },
          fragments: {
            where: { kind: { in: ["question", "voix"] } },
            orderBy: { position: "asc" },
            take: FRAGMENTS_PER_THREAD,
            select: {
              kind: true,
              body: true,
              durationSeconds: true,
              prompt: { select: { text: true } },
            },
          },
        },
      },
    },
  });

  const pool: Candidate[] = [];

  for (const row of rows) {
    const profile = row.profile;
    // Une personne sans trame lisible n'est pas proposable : un fil s'engage par
    // une réponse à un fragment, il en faut donc au moins un.
    if (profile === null || profile.fragments.length === 0) continue;

    const distance = escaleActive
      ? 0
      : distanceKm(origin.lat, origin.lon, profile.latRounded, profile.lonRounded);
    if (!escaleActive && distance > preference.maxDistanceKm) continue;

    const motif = profile.motifTags.map((t) => t.tag);

    pool.push({
      accountId: row.id,
      displayName: row.displayName,
      age: ageFrom(row.birthDate),
      city: profile.city,
      distanceKm: distance,
      motif,
      photoKey: profile.photoKey,
      fragments: profile.fragments.map((f) => ({
        promptText: f.prompt.text,
        kind: f.kind,
        body: f.body,
        durationSeconds: f.durationSeconds,
      })),
      score: scoreCandidate(viewerMotif, motif, distance, preference.maxDistanceKm, row.lastSeenAt),
    });
  }

  pool.sort((a, b) => b.score - a.score);
  await setJson(keys.pool(accountId), pool, POOL_TTL_SECONDS);
  return pool;
}

function safeJsonArray(raw: string): string[] {
  try {
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((v): v is string => typeof v === "string") : [];
  } catch {
    return [];
  }
}

/* ------------------------------------------------------------------ */
/* Fabrication d'une carte de fil                                      */
/* ------------------------------------------------------------------ */

function buildFragments(candidate: Candidate): Fragment[] {
  const fragments: Fragment[] = candidate.fragments.map((f) => ({
    id: uuid(),
    kind: f.kind === "voix" ? "voix" : "question",
    prompt: f.promptText,
    body:
      f.kind === "voix"
        ? signMediaUrl(env.media.baseUrl, env.media.signingSecret, f.body, {
            expiresInSeconds: env.media.signedUrlTtlSeconds,
          })
        : f.body,
    ...(f.durationSeconds !== null ? { durationSeconds: f.durationSeconds } : {}),
  }));

  // Le motif ferme toujours la trame : cinq mots, pour situer sans décrire.
  fragments.push({
    id: uuid(),
    kind: "motif",
    prompt: "Motif",
    body: candidate.motif.join(" · "),
  });

  return fragments.slice(0, FRAGMENTS_PER_THREAD + 1);
}

/**
 * Compose la carte d'un fil. La photo est servie floutée à hauteur de ce qui
 * n'a pas encore été révélé : au premier contact, elle est intégralement
 * masquée côté serveur — le client ne reçoit jamais l'image nette.
 */
function buildCard(candidate: Candidate, expiresAt: Date): ThreadCard {
  const revealPercent = REVEAL_STEPS[0];
  return {
    id: uuid(),
    state: "propose",
    displayName: candidate.displayName,
    age: candidate.age,
    distanceKm: candidate.distanceKm,
    city: candidate.city,
    motif: candidate.motif,
    fragments: buildFragments(candidate),
    revealPercent,
    photoUrl:
      candidate.photoKey === null
        ? null
        : signMediaUrl(env.media.baseUrl, env.media.signingSecret, candidate.photoKey, {
            expiresInSeconds: env.media.signedUrlTtlSeconds,
            blur: 100 - revealPercent,
          }),
    expiresAt: expiresAt.toISOString(),
    exchanges: 0,
    awaitingYou: true,
  };
}

/* ------------------------------------------------------------------ */
/* Tissage                                                             */
/* ------------------------------------------------------------------ */

/**
 * Tisse un fil si une place est libre.
 *
 * L'écriture en cache est atomique : si deux requêtes concurrentes tentent de
 * garnir la dernière place, une seule aboutit, et le plafond de trois tient.
 */
async function weaveOne(
  accountId: string,
  pool: Candidate[],
  used: Set<string>,
): Promise<ThreadCard | null> {
  for (const candidate of pool) {
    if (used.has(candidate.accountId)) continue;

    const expiresAt = new Date(Date.now() + THREAD_TTL_SECONDS * 1000);
    const card = buildCard(candidate, expiresAt);

    const count = await addThread(accountId, card, THREAD_TTL_SECONDS);
    if (count === null) return null; // métier plein : un autre appel a pris la place

    try {
      await prisma.threadLedger.create({
        data: {
          viewerId: accountId,
          candidateId: candidate.accountId,
          cacheKey: card.id,
          score: candidate.score,
          expiresAt,
        },
      });
    } catch (error) {
      // Le registre est la seule garantie d'unicité des propositions. S'il
      // refuse l'écriture (candidat déjà proposé), le fil ne doit pas subsister.
      const { removeThread } = await import("../lib/cache.ts");
      await removeThread(accountId, card.id);
      log.warn("Registre refusé, fil retiré", { accountId, error: String(error) });
      used.add(candidate.accountId);
      continue;
    }

    used.add(candidate.accountId);
    return card;
  }
  return null;
}

/** Indique si le regarnissage automatique est autorisé maintenant. */
async function refillDue(accountId: string): Promise<boolean> {
  const at = await redis.get(keys.refill(accountId));
  return at === null || Number(at) <= Date.now();
}

async function scheduleRefill(accountId: string, delayMinutes: number): Promise<Date> {
  const at = refillAt(delayMinutes);
  await redis.set(keys.refill(accountId), String(at.getTime()), "EX", THREAD_TTL_SECONDS);
  return at;
}

/** Efface le compte à rebours de regarnissage (achat d'un « Relais »). */
export async function clearRefill(accountId: string): Promise<void> {
  await redis.del(keys.refill(accountId));
}

/**
 * Renvoie le métier de la personne, en le regarnissant si l'heure est venue.
 *
 * C'est l'unique point d'entrée de lecture des fils : il garantit que le
 * plafond de trois est respecté et que rien n'est lu ailleurs que dans le cache.
 */
export async function getLoom(account: AuthenticatedAccount): Promise<Loom> {
  const entitlements = entitlementsFor(account.plan);

  const existing = await readLoom(account.id);
  let threads = existing;

  const slots = MAX_ACTIVE_THREADS - existing.length;
  if (slots > 0 && (await refillDue(account.id))) {
    const woven = await withWeaveLock(account.id, 10, async () => {
      const pool = await buildCandidatePool(account.id);
      const used = new Set(existing.map((t) => t.displayName));
      const created: ThreadCard[] = [];

      for (let i = 0; i < slots; i++) {
        const card = await weaveOne(account.id, pool, used);
        if (card === null) break;
        created.push(card);
      }
      return created;
    });

    if (woven !== null && woven.length > 0) {
      threads = await readLoom(account.id);
      // Le vivier vient d'être entamé : il sera recalculé au prochain besoin.
      await redis.del(keys.pool(account.id));
    }
  }

  const free = Math.max(0, MAX_ACTIVE_THREADS - threads.length);
  const nextRefill =
    free > 0 ? await scheduleRefill(account.id, entitlements.refillDelayMinutes) : null;

  return {
    threads,
    nextWeavingAt: nextWeavingAt(account.timezone, account.weavingHour).toISOString(),
    freeSlots: free,
    nextRefillAt: nextRefill?.toISOString() ?? null,
    fromCache: true,
  };
}

/** Invalide le vivier après un changement de critères. */
export async function invalidatePool(accountId: string): Promise<void> {
  await redis.del(keys.pool(accountId));
}

export async function currentFreeSlots(accountId: string): Promise<number> {
  return freeSlots(accountId);
}
