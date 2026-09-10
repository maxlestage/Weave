/**
 * Live Activity « Métier » : les trois fils sur l'écran verrouillé et dans
 * l'île dynamique, avec le compte à rebours du fil le plus proche de se dénouer.
 *
 * L'état poussé est délibérément minuscule — des compteurs et des dates. Aucun
 * contenu de profil ne transite par APNs : ni prénom complet, ni photo, ni
 * fragment. Ce qui s'affiche sur un écran verrouillé doit pouvoir être lu par
 * quelqu'un d'autre sans rien révéler.
 */
import type { Loom, LiveActivityState, WatchSummary } from "@weave/contracts";
import { keys, readLoom } from "../lib/cache.ts";
import { endLiveActivity, isDeadToken, startLiveActivity, updateLiveActivity } from "../lib/apns.ts";
import { log } from "../lib/log.ts";
import { prisma } from "../lib/prisma.ts";
import { getJson, setJson } from "../lib/redis.ts";
import { THREAD_TTL_MAX_SECONDS } from "@weave/contracts";

/** Type d'attributs ActivityKit, doit correspondre au nom Swift exact. */
const ATTRIBUTES_TYPE = "WeaveActivityAttributes";

/** Durée de validité d'un état poussé avant que le système le marque périmé. */
const STALE_AFTER_SECONDS = 60 * 60;

/** TTL du dernier état connu en cache. */
const STATE_TTL_SECONDS = THREAD_TTL_MAX_SECONDS;

/** Compose l'état dynamique à partir des fils en cache. */
export function stateFromLoom(loom: Pick<Loom, "threads" | "nextRefillAt">): LiveActivityState {
  const sorted = [...loom.threads].sort(
    (a, b) => new Date(a.expiresAt).getTime() - new Date(b.expiresAt).getTime(),
  );
  const soonest = sorted[0] ?? null;

  return {
    activeThreads: loom.threads.length,
    awaitingYou: loom.threads.filter((thread) => thread.awaitingYou).length,
    soonestExpiryAt: soonest?.expiresAt ?? null,
    // Le prénom seul, jamais le profil : c'est ce qui apparaît sur l'écran verrouillé.
    soonestName: soonest?.displayName ?? null,
    nextRefillAt: loom.nextRefillAt,
    updatedAt: new Date().toISOString(),
  };
}

/** Résumé compact pour watchOS : quelques centaines d'octets, pas plus. */
export function watchSummaryFromLoom(loom: Pick<Loom, "threads">): WatchSummary {
  return {
    activeThreads: loom.threads.length,
    awaitingYou: loom.threads.filter((thread) => thread.awaitingYou).length,
    soonestExpiryAt:
      loom.threads
        .map((thread) => thread.expiresAt)
        .sort((a, b) => new Date(a).getTime() - new Date(b).getTime())[0] ?? null,
    entries: loom.threads.map((thread) => ({
      id: thread.id,
      name: thread.displayName,
      expiresAt: thread.expiresAt,
      awaitingYou: thread.awaitingYou,
    })),
    generatedAt: new Date().toISOString(),
  };
}

/** Deux états sont équivalents si rien d'affichable n'a changé. */
function sameDisplay(a: LiveActivityState | null, b: LiveActivityState): boolean {
  if (a === null) return false;
  return (
    a.activeThreads === b.activeThreads &&
    a.awaitingYou === b.awaitingYou &&
    a.soonestExpiryAt === b.soonestExpiryAt &&
    a.nextRefillAt === b.nextRefillAt
  );
}

/**
 * Recalcule l'état et le pousse vers les Live Activities en cours.
 *
 * L'envoi est ignoré si rien d'affichable n'a bougé : une Live Activity qui
 * clignote pour rien coûte de la batterie et de la confiance.
 */
export async function publishLiveActivityState(
  accountId: string,
  loom?: Pick<Loom, "threads" | "nextRefillAt">,
): Promise<LiveActivityState> {
  const source = loom ?? { threads: await readLoom(accountId), nextRefillAt: null };
  const state = stateFromLoom(source);

  const previous = await getJson<LiveActivityState>(keys.liveActivity(accountId));
  await setJson(keys.liveActivity(accountId), state, STATE_TTL_SECONDS);
  await setJson(keys.watch(accountId), watchSummaryFromLoom(source), STATE_TTL_SECONDS);

  if (sameDisplay(previous, state)) return state;

  const sessions = await prisma.liveActivitySession.findMany({
    where: { accountId, endedAt: null, staleAt: { gt: new Date() } },
    select: { id: true, updateToken: true },
  });

  const staleDate = Math.floor(Date.now() / 1000) + STALE_AFTER_SECONDS;

  for (const session of sessions) {
    const result =
      state.activeThreads === 0
        ? await endLiveActivity({
            updateToken: session.updateToken,
            contentState: state as unknown as Record<string, unknown>,
          })
        : await updateLiveActivity({
            updateToken: session.updateToken,
            contentState: state as unknown as Record<string, unknown>,
            staleDate,
            collapseId: `loom-${accountId}`,
            priority: state.awaitingYou > 0 ? 10 : 5,
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
        ...(state.activeThreads === 0 ? { endedAt: new Date() } : {}),
      },
    });
  }

  return state;
}

/**
 * Démarre à distance la Live Activity à l'heure de tissage, via le jeton
 * « push-to-start » d'ActivityKit. C'est ce qui permet à la bannière
 * d'apparaître d'elle-même quand les fils du jour sont prêts, sans que
 * l'application ait besoin d'être lancée.
 */
export async function startLiveActivitiesFor(accountId: string): Promise<number> {
  const devices = await prisma.device.findMany({
    where: { accountId, platform: "ios", pushToStartToken: { not: null } },
    select: { id: true, pushToStartToken: true },
  });
  if (devices.length === 0) return 0;

  const state = await publishLiveActivityState(accountId);
  if (state.activeThreads === 0) return 0;

  const staleDate = Math.floor(Date.now() / 1000) + STALE_AFTER_SECONDS;
  let started = 0;

  for (const device of devices) {
    const result = await startLiveActivity({
      pushToStartToken: device.pushToStartToken!,
      attributesType: ATTRIBUTES_TYPE,
      attributes: { accountHandle: accountId.slice(0, 8) },
      contentState: state as unknown as Record<string, unknown>,
      staleDate,
      alert: {
        title: "Votre métier est prêt",
        body: `${state.activeThreads} fil${state.activeThreads > 1 ? "s" : ""} vous attend${state.activeThreads > 1 ? "ent" : ""}.`,
      },
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
