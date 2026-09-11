/**
 * Live Activity « Prochain plan » : le rendez-vous le plus proche sur l'écran
 * verrouillé et dans l'île dynamique, avec ce qui attend une réponse.
 *
 * L'état poussé est délibérément minuscule — un titre, une date, deux
 * compteurs. Aucun nom, aucune photo, aucun message ne transite par APNs : ce
 * qui s'affiche sur un écran verrouillé doit pouvoir être lu par quelqu'un
 * d'autre sans rien révéler de qui vous voyez.
 */
import type { LiveActivityState, WatchSummary } from "@weave/contracts";
import { keys } from "../lib/cache.ts";
import {
  endLiveActivity,
  isDeadToken,
  startLiveActivity,
  updateLiveActivity,
} from "../lib/apns.ts";
import { log } from "../lib/log.ts";
import { prisma } from "../lib/prisma.ts";
import { getJson, setJson } from "../lib/redis.ts";
import { PLAN_GRACE_MINUTES } from "@weave/contracts";

/** Type d'attributs ActivityKit, doit correspondre au nom Swift exact. */
const ATTRIBUTES_TYPE = "WeaveActivityAttributes";

/** Durée de validité d'un état poussé avant que le système le marque périmé. */
const STALE_AFTER_SECONDS = 60 * 60;

/** TTL du dernier état connu en cache. */
const STATE_TTL_SECONDS = 24 * 60 * 60;

/** Ce qu'il faut savoir pour composer un état : lu en une passe. */
async function situationDe(accountId: string) {
  const maintenant = new Date();
  const plancher = new Date(maintenant.getTime() - PLAN_GRACE_MINUTES * 60_000);

  const [publies, rejoints, aTraiter, sansReponse] = await Promise.all([
    // Mes propres plans à venir.
    prisma.plan.findMany({
      where: { authorId: accountId, state: "ouvert", startsAt: { gte: plancher } },
      orderBy: { startsAt: "asc" },
      take: 1,
      select: { title: true, startsAt: true, city: true },
    }),
    // Les plans où ma demande a été acceptée.
    prisma.joinRequest.findMany({
      where: {
        authorId: accountId,
        state: "acceptee",
        plan: { state: "ouvert", startsAt: { gte: plancher } },
      },
      orderBy: { plan: { startsAt: "asc" } },
      take: 1,
      select: { plan: { select: { title: true, startsAt: true, city: true } } },
    }),
    // Demandes reçues sur mes plans, encore sans décision.
    prisma.joinRequest.count({
      where: { state: "envoyee", plan: { authorId: accountId, state: "ouvert" } },
    }),
    // Mes demandes envoyées, encore sans réponse.
    prisma.joinRequest.count({ where: { authorId: accountId, state: "envoyee" } }),
  ]);

  const candidats = [...publies, ...rejoints.map((r) => r.plan)].sort(
    (a, b) => a.startsAt.getTime() - b.startsAt.getTime(),
  );

  return { prochain: candidats[0] ?? null, aTraiter, sansReponse };
}

type Situation = Awaited<ReturnType<typeof situationDe>>;

export function stateFromSituation(situation: Situation): LiveActivityState {
  return {
    planTitle: situation.prochain?.title ?? null,
    planStartsAt: situation.prochain?.startsAt.toISOString() ?? null,
    pendingRequests: situation.aTraiter,
    awaitingReply: situation.sansReponse,
    updatedAt: new Date().toISOString(),
  };
}

/** Résumé compact pour watchOS : quelques centaines d'octets, pas plus. */
export function watchSummaryFromSituation(situation: Situation): WatchSummary {
  return {
    pendingRequests: situation.aTraiter,
    awaitingReply: situation.sansReponse,
    nextPlan:
      situation.prochain === null
        ? null
        : {
            title: situation.prochain.title,
            startsAt: situation.prochain.startsAt.toISOString(),
            city: situation.prochain.city,
          },
    generatedAt: new Date().toISOString(),
  };
}

/** Deux états sont équivalents si rien d'affichable n'a changé. */
function sameDisplay(a: LiveActivityState | null, b: LiveActivityState): boolean {
  if (a === null) return false;
  return (
    a.planTitle === b.planTitle &&
    a.planStartsAt === b.planStartsAt &&
    a.pendingRequests === b.pendingRequests &&
    a.awaitingReply === b.awaitingReply
  );
}

/** Vrai s'il n'y a plus rien à afficher : la Live Activity doit se terminer. */
function vide(state: LiveActivityState): boolean {
  return state.planTitle === null && state.pendingRequests === 0 && state.awaitingReply === 0;
}

/**
 * Recalcule l'état et le pousse vers les Live Activities en cours.
 *
 * L'envoi est ignoré si rien d'affichable n'a bougé : une Live Activity qui
 * clignote pour rien coûte de la batterie et de la confiance.
 */
export async function publishLiveActivityState(accountId: string): Promise<LiveActivityState> {
  const situation = await situationDe(accountId);
  const state = stateFromSituation(situation);

  const previous = await getJson<LiveActivityState>(keys.liveActivity(accountId));
  await setJson(keys.liveActivity(accountId), state, STATE_TTL_SECONDS);
  await setJson(keys.watch(accountId), watchSummaryFromSituation(situation), STATE_TTL_SECONDS);

  if (sameDisplay(previous, state)) return state;

  const sessions = await prisma.liveActivitySession.findMany({
    where: { accountId, endedAt: null, staleAt: { gt: new Date() } },
    select: { id: true, updateToken: true },
  });

  const staleDate = Math.floor(Date.now() / 1000) + STALE_AFTER_SECONDS;
  const termine = vide(state);

  for (const session of sessions) {
    const result = termine
      ? await endLiveActivity({
          updateToken: session.updateToken,
          contentState: state as unknown as Record<string, unknown>,
        })
      : await updateLiveActivity({
          updateToken: session.updateToken,
          contentState: state as unknown as Record<string, unknown>,
          staleDate,
          collapseId: `plan-${accountId}`,
          priority: state.pendingRequests > 0 ? 10 : 5,
        });

    if (isDeadToken(result)) {
      await prisma.liveActivitySession.update({
        where: { id: session.id },
        data: { endedAt: new Date() },
      });
      continue;
    }

    await prisma.liveActivitySession.update({
      where: { id: session.id },
      data: {
        lastStateJson: JSON.stringify(state),
        lastPushAt: new Date(),
        ...(termine ? { endedAt: new Date() } : {}),
      },
    });
  }

  return state;
}

/**
 * Démarre à distance la Live Activity via le jeton « push-to-start »
 * d'ActivityKit. C'est ce qui permet à la bannière d'apparaître d'elle-même
 * quand quelqu'un demande à venir, sans que l'application ait été lancée.
 */
export async function startLiveActivitiesFor(accountId: string): Promise<number> {
  const devices = await prisma.device.findMany({
    where: { accountId, platform: "ios", pushToStartToken: { not: null } },
    select: { id: true, pushToStartToken: true },
  });
  if (devices.length === 0) return 0;

  const state = await publishLiveActivityState(accountId);
  if (vide(state)) return 0;

  const staleDate = Math.floor(Date.now() / 1000) + STALE_AFTER_SECONDS;
  let started = 0;

  for (const device of devices) {
    const result = await startLiveActivity({
      pushToStartToken: device.pushToStartToken!,
      attributesType: ATTRIBUTES_TYPE,
      attributes: { accountHandle: accountId.slice(0, 8) },
      contentState: state as unknown as Record<string, unknown>,
      staleDate,
      alert: alerteDe(state),
    });

    if (result.ok) started += 1;
    else if (isDeadToken(result)) {
      await prisma.device.update({
        where: { id: device.id },
        data: { pushToStartToken: null },
      });
    }
  }

  log.info("Live Activities démarrées", { accountId, started });
  return started;
}

/** Une phrase qui dit ce qui se passe, sans nommer personne. */
function alerteDe(state: LiveActivityState): { title: string; body: string } {
  if (state.pendingRequests > 0) {
    const n = state.pendingRequests;
    return {
      title: n > 1 ? `${n} personnes veulent venir` : "Quelqu'un veut venir",
      body: state.planTitle ?? "Ouvrez Weave pour lire les messages.",
    };
  }
  return {
    title: "Votre prochain plan",
    body: state.planTitle ?? "Ouvrez Weave.",
  };
}
