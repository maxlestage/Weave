/**
 * Composition du fil.
 *
 * Ce que ce module NE fait pas, volontairement :
 *  • il ne classe personne devant les autres contre paiement ;
 *  • il n'expose aucune liste de « qui vous a demandé » à débloquer ;
 *  • il ne présente pas de pile de profils à balayer.
 *
 * Ce qu'il fait : rassembler les plans à venir autour de quelqu'un, dans ses
 * critères, et les ordonner par imminence puis par proximité. Deux termes
 * seulement, tous deux explicables à qui les lit.
 */
import {
  DEFAULT_RADIUS_KM,
  FEED_TTL_SECONDS,
  PLAN_GRACE_MINUTES,
  type Feed,
  type Plan,
  type PlanCategory,
  type PlanState,
} from "@weave/contracts";
import { env } from "../env.ts";
import { keys, requestsLeft } from "../lib/cache.ts";
import { signMediaUrl } from "../lib/crypto.ts";
import { prisma } from "../lib/prisma.ts";
import { getJson, setJson } from "../lib/redis.ts";
import { ageFrom, boundingBox, distanceKm, localDay } from "../lib/time.ts";
import type { AuthenticatedAccount } from "../plugins/auth.ts";
import { entitlementsFor } from "./entitlements.ts";

/** Plans extraits de la base avant tri fin en mémoire. */
const FETCH_LIMIT = 300;

/** Plans renvoyés dans un fil. */
const FEED_SIZE = 60;

interface Contexte {
  latRounded: number;
  lonRounded: number;
  maxDistanceKm: number;
  minAge: number;
  maxAge: number;
  seeking: string[];
  categories: string[];
  escaleCity: string | null;
  exclus: string[];
}

async function contexteDe(accountId: string): Promise<Contexte | null> {
  const compte = await prisma.account.findUnique({
    where: { id: accountId },
    select: {
      profile: { select: { latRounded: true, lonRounded: true } },
      preference: true,
      blocksMade: { select: { targetId: true } },
      blocksReceived: { select: { authorId: true } },
    },
  });

  if (compte?.profile == null || compte.preference == null) return null;

  const pref = compte.preference;
  const escaleActive = pref.escaleUntil !== null && pref.escaleUntil.getTime() > Date.now();

  return {
    latRounded: compte.profile.latRounded,
    lonRounded: compte.profile.lonRounded,
    maxDistanceKm: pref.maxDistanceKm || DEFAULT_RADIUS_KM,
    minAge: pref.minAge,
    maxAge: pref.maxAge,
    seeking: listeJson(pref.seekingJson),
    categories: listeJson(pref.categoriesJson),
    escaleCity: escaleActive ? pref.escaleCity : null,
    exclus: [
      accountId,
      ...compte.blocksMade.map((b) => b.targetId),
      ...compte.blocksReceived.map((b) => b.authorId),
    ],
  };
}

function listeJson(brut: string): string[] {
  try {
    const valeur = JSON.parse(brut);
    return Array.isArray(valeur) ? valeur.filter((v): v is string => typeof v === "string") : [];
  } catch {
    return [];
  }
}

/**
 * Construit le fil : les plans à venir, dans le rayon, dans les critères.
 *
 * L'ordre est celui de l'imminence — ce qui arrive bientôt d'abord — puis de la
 * proximité à égalité de jour. Aucun autre terme n'intervient : ni ancienneté
 * de compte, ni palier, ni popularité.
 */
export async function buildFeed(account: AuthenticatedAccount): Promise<Feed> {
  const droits = entitlementsFor(account.tier);
  const jour = localDay(account.timezone);
  const restantes = await requestsLeft(account.id, jour, droits.requestsPerDay);

  const cache = await getJson<Omit<Feed, "requestsLeftToday">>(keys.feed(account.id));
  if (cache !== null) {
    return { ...cache, requestsLeftToday: restantes, fromCache: true };
  }

  const contexte = await contexteDe(account.id);
  if (contexte === null) {
    return {
      plans: [],
      requestsLeftToday: restantes,
      fromCache: false,
      generatedAt: new Date().toISOString(),
    };
  }

  const maintenant = new Date();
  const plancher = new Date(maintenant.getTime() - PLAN_GRACE_MINUTES * 60_000);
  const plafond = new Date(maintenant.getTime() + droits.daysAhead * 24 * 60 * 60 * 1000);

  const boite = boundingBox(contexte.latRounded, contexte.lonRounded, contexte.maxDistanceKm);
  const ageMax = new Date(
    maintenant.getFullYear() - contexte.minAge,
    maintenant.getMonth(),
    maintenant.getDate(),
  );
  const ageMin = new Date(
    maintenant.getFullYear() - contexte.maxAge - 1,
    maintenant.getMonth(),
    maintenant.getDate(),
  );

  const lignes = await prisma.plan.findMany({
    where: {
      state: "ouvert",
      startsAt: { gte: plancher, lte: plafond },
      authorId: { notIn: contexte.exclus },
      ...(contexte.categories.length > 0 ? { category: { in: contexte.categories } } : {}),
      ...(contexte.escaleCity !== null
        ? { city: contexte.escaleCity }
        : {
            latRounded: { gte: boite.minLat, lte: boite.maxLat },
            lonRounded: { gte: boite.minLon, lte: boite.maxLon },
          }),
      author: {
        status: "active",
        deletionRequestedAt: null,
        birthDate: { gte: ageMin, lte: ageMax },
        ...(contexte.seeking.length > 0
          ? { profile: { is: { gender: { in: contexte.seeking } } } }
          : {}),
      },
    },
    take: FETCH_LIMIT,
    orderBy: { startsAt: "asc" },
    include: {
      author: {
        select: {
          id: true,
          displayName: true,
          birthDate: true,
          verified: true,
          profile: { select: { photoKey: true } },
        },
      },
      requests: {
        where: { state: { in: ["envoyee", "acceptee"] } },
        select: { authorId: true, state: true },
      },
    },
  });

  const plans: Plan[] = [];

  for (const ligne of lignes) {
    const distance =
      contexte.escaleCity !== null
        ? 0
        : distanceKm(contexte.latRounded, contexte.lonRounded, ligne.latRounded, ligne.lonRounded);
    if (contexte.escaleCity === null && distance > contexte.maxDistanceKm) continue;

    const acceptees = ligne.requests.filter((r) => r.state === "acceptee").length;
    const placesRestantes = Math.max(0, ligne.capacity - acceptees);
    // Un plan complet reste visible mais n'accepte plus : le masquer donnerait
    // l'impression qu'il n'y a rien, alors qu'il s'y passe justement quelque chose.
    if (placesRestantes === 0 && !ligne.requests.some((r) => r.authorId === account.id)) continue;

    plans.push(versPlan(ligne, distance, placesRestantes, account.id));
  }

  // Imminence d'abord, proximité ensuite. Rien d'autre.
  plans.sort((a, b) => {
    const ecart = new Date(a.startsAt).getTime() - new Date(b.startsAt).getTime();
    if (Math.abs(ecart) > 12 * 60 * 60 * 1000) return ecart;
    return a.distanceKm - b.distanceKm;
  });

  const fil = plans.slice(0, FEED_SIZE);
  const genere = new Date().toISOString();

  await setJson(
    keys.feed(account.id),
    { plans: fil, fromCache: true, generatedAt: genere },
    FEED_TTL_SECONDS,
  );

  return { plans: fil, requestsLeftToday: restantes, fromCache: false, generatedAt: genere };
}

type LignePlan = {
  id: string;
  title: string;
  note: string;
  category: string;
  startsAt: Date;
  city: string;
  capacity: number;
  state: string;
  createdAt: Date;
  author: {
    id: string;
    displayName: string;
    birthDate: Date;
    verified: boolean;
    profile: { photoKey: string | null } | null;
  };
  requests: { authorId: string; state: string }[];
};

export function versPlan(
  ligne: LignePlan,
  distance: number,
  placesRestantes: number,
  viewerId: string,
): Plan {
  return {
    id: ligne.id,
    author: {
      id: ligne.author.id,
      displayName: ligne.author.displayName,
      age: ageFrom(ligne.author.birthDate),
      photoUrl: photoSignee(ligne.author.profile?.photoKey ?? null),
      verified: ligne.author.verified,
    },
    title: ligne.title,
    note: ligne.note,
    category: ligne.category as PlanCategory,
    startsAt: ligne.startsAt.toISOString(),
    city: ligne.city,
    distanceKm: distance,
    capacity: ligne.capacity,
    seatsLeft: placesRestantes,
    state: (placesRestantes === 0 ? "complet" : ligne.state) as PlanState,
    requested: ligne.requests.some((r) => r.authorId === viewerId),
    createdAt: ligne.createdAt.toISOString(),
  };
}

export function photoSignee(photoKey: string | null): string | null {
  if (photoKey === null) return null;
  return signMediaUrl(env.media.baseUrl, env.media.signingSecret, photoKey, {
    expiresInSeconds: env.media.signedUrlTtlSeconds,
  });
}
