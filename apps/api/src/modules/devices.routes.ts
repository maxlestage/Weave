/**
 * Appareils, Live Activities et Apple Watch.
 *
 * L'application iOS enregistre ici trois choses : son jeton APNs classique, son
 * jeton « push-to-start » ActivityKit (qui autorise le serveur à démarrer la
 * Live Activity à distance), puis le jeton propre à chaque activité en cours.
 */
import { Elysia, t } from "elysia";
import type { WatchSummary } from "@weave/contracts";
import { keys } from "../lib/cache.ts";
import { notFound } from "../lib/errors.ts";
import { prisma } from "../lib/prisma.ts";
import { getJson } from "../lib/redis.ts";
import { authPlugin } from "../plugins/auth.ts";
import { publishLiveActivityState, startLiveActivitiesFor } from "./live-activity.service.ts";

/** Une Live Activity vit au plus huit heures ; au-delà le système la fige. */
const ACTIVITY_MAX_HOURS = 8;

export const deviceRoutes = new Elysia({ prefix: "/v1", tags: ["Appareils"] })
  .use(authPlugin)

  .put(
    "/devices",
    async ({ requireAccount, body }) => {
      const account = requireAccount();

      const device = await prisma.device.upsert({
        where: { accountId_vendorId: { accountId: account.id, vendorId: body.vendorId } },
        create: {
          accountId: account.id,
          vendorId: body.vendorId,
          platform: body.platform,
          model: body.model ?? null,
          osVersion: body.osVersion ?? null,
          appVersion: body.appVersion ?? null,
          apnsToken: body.apnsToken ?? null,
          pushToStartToken: body.pushToStartToken ?? null,
          apnsEnvironment: body.apnsEnvironment ?? "sandbox",
        },
        update: {
          platform: body.platform,
          ...(body.model !== undefined ? { model: body.model } : {}),
          ...(body.osVersion !== undefined ? { osVersion: body.osVersion } : {}),
          ...(body.appVersion !== undefined ? { appVersion: body.appVersion } : {}),
          ...(body.apnsToken !== undefined ? { apnsToken: body.apnsToken } : {}),
          ...(body.pushToStartToken !== undefined
            ? { pushToStartToken: body.pushToStartToken }
            : {}),
          ...(body.apnsEnvironment !== undefined ? { apnsEnvironment: body.apnsEnvironment } : {}),
          lastSeenAt: new Date(),
        },
        select: { id: true },
      });

      return { ok: true, deviceId: device.id };
    },
    {
      body: t.Object({
        vendorId: t.String({ minLength: 4, maxLength: 128 }),
        platform: t.Union([t.Literal("ios"), t.Literal("watchos"), t.Literal("web")]),
        model: t.Optional(t.String({ maxLength: 60 })),
        osVersion: t.Optional(t.String({ maxLength: 30 })),
        appVersion: t.Optional(t.String({ maxLength: 30 })),
        apnsToken: t.Optional(t.String({ maxLength: 200 })),
        pushToStartToken: t.Optional(t.String({ maxLength: 400 })),
        apnsEnvironment: t.Optional(t.Union([t.Literal("sandbox"), t.Literal("production")])),
      }),
      detail: {
        summary: "Enregistrer un appareil",
        description:
          "Le `pushToStartToken` autorise le serveur à démarrer la Live Activity quand quelqu'un demande à venir, application fermée.",
      },
    },
  )

  .post(
    "/live-activity/sessions",
    async ({ requireAccount, body }) => {
      const account = requireAccount();

      const device = await prisma.device.findUnique({
        where: { accountId_vendorId: { accountId: account.id, vendorId: body.vendorId } },
        select: { id: true },
      });
      if (device === null) throw notFound("Appareil inconnu. Enregistrez-le d'abord.");

      const staleAt = new Date(Date.now() + ACTIVITY_MAX_HOURS * 60 * 60 * 1000);

      await prisma.liveActivitySession.upsert({
        where: { updateToken: body.updateToken },
        create: {
          accountId: account.id,
          deviceId: device.id,
          updateToken: body.updateToken,
          staleAt,
        },
        update: { staleAt, endedAt: null },
      });

      const state = await publishLiveActivityState(account.id);
      return { ok: true, state };
    },
    {
      body: t.Object({
        vendorId: t.String({ minLength: 4, maxLength: 128 }),
        updateToken: t.String({ minLength: 10, maxLength: 400 }),
      }),
      detail: {
        summary: "Déclarer une Live Activity en cours",
        description: "L'application transmet le jeton de mise à jour fourni par ActivityKit.",
      },
    },
  )

  .delete(
    "/live-activity/sessions/:token",
    async ({ requireAccount, params }) => {
      const account = requireAccount();
      await prisma.liveActivitySession.updateMany({
        where: { accountId: account.id, updateToken: params.token, endedAt: null },
        data: { endedAt: new Date() },
      });
      return { ok: true };
    },
    {
      params: t.Object({ token: t.String() }),
      detail: { summary: "Signaler la fin d'une Live Activity" },
    },
  )

  .get(
    "/live-activity/state",
    async ({ requireAccount }) => {
      const account = requireAccount();
      return publishLiveActivityState(account.id);
    },
    {
      detail: {
        summary: "Lire l'état courant de la Live Activity",
        description: "Utilisé au premier lancement pour initialiser l'activité côté application.",
      },
    },
  )

  .post(
    "/live-activity/start",
    async ({ requireAccount }) => {
      const account = requireAccount();
      const started = await startLiveActivitiesFor(account.id);
      return { ok: true, started };
    },
    {
      detail: {
        summary: "Démarrer la Live Activity à distance",
        description:
          "Déclenche un envoi « push-to-start ». Normalement déclenché par l'arrivée d'une demande.",
      },
    },
  )

  .get(
    "/watch/summary",
    async ({ requireAccount }): Promise<WatchSummary> => {
      const account = requireAccount();

      // La montre lit d'abord le résumé en cache : c'est une requête fréquente,
      // depuis un appareil dont la connexion et la batterie sont contraintes.
      const cached = await getJson<WatchSummary>(keys.watch(account.id));
      if (cached !== null) return cached;

      // Absent du cache : le recalcul repeuple aussi la clé au passage.
      await publishLiveActivityState(account.id);
      return (await getJson<WatchSummary>(keys.watch(account.id)))!;
    },
    {
      detail: {
        summary: "Résumé pour Apple Watch",
        description:
          "Charge utile compacte : le prochain plan et deux compteurs. Aucun nom, aucune photo, aucun message.",
      },
    },
  );
