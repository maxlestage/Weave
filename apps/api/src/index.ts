/**
 * Point d'entrée de l'API Weave.
 *
 * Pile : Bun + Elysia + Prisma + Redis. Aucun Node.js dans la chaîne, ni au
 * développement ni en production.
 */
import { Elysia } from "elysia";
import { cors } from "@elysiajs/cors";
import { openapi } from "@elysiajs/openapi";
import { serverTiming } from "@elysiajs/server-timing";
import { staticPlugin } from "@elysiajs/static";
import { MAX_ACTIVE_THREADS } from "@weave/contracts";
import { env } from "./env.ts";
import { AppError } from "./lib/errors.ts";
import { log } from "./lib/log.ts";
import { disconnectPrisma, prisma } from "./lib/prisma.ts";
import { disconnectRedis, pingRedis } from "./lib/redis.ts";
import { authRoutes } from "./modules/auth.routes.ts";
import { billingRoutes } from "./modules/billing.routes.ts";
import { deviceRoutes } from "./modules/devices.routes.ts";
import { loomRoutes } from "./modules/loom.routes.ts";
import { mediaRoutes } from "./modules/media.routes.ts";
import { meRoutes } from "./modules/me.routes.ts";
import { moderationRoutes } from "./modules/moderation.routes.ts";
import { promptRoutes } from "./modules/prompts.routes.ts";
import { threadRoutes } from "./modules/threads.routes.ts";

/**
 * Site vitrine compilé, servi par le même processus que l'API.
 *
 * Le site est statique et peu visité : lui dédier un second processus
 * doublerait le coût d'hébergement sans rien apporter. En développement,
 * `WEB_DIST_PATH` n'est pas défini — Vite s'en charge — et ce greffon
 * n'enregistre alors aucune route.
 */
const vitrine =
  env.webDist === undefined
    ? new Elysia({ name: "weave/vitrine-absente" })
    : new Elysia({ name: "weave/vitrine" })
        .use(
          await staticPlugin({
            assets: env.webDist,
            prefix: "/",
            indexHTML: true,
            maxAge: 3600,
          }),
        )
        // `indexHTML` ne couvre pas la racine elle-même : sans cette route,
        // ouvrir l'adresse du service renvoie une 404.
        .get("/", () => Bun.file(`${env.webDist}/index.html`), {
          detail: { summary: "Site vitrine", tags: ["Service"] },
        });

export const app = new Elysia()
  .use(
    cors({
      origin: env.isProduction ? [env.webOrigin] : true,
      credentials: true,
      methods: ["GET", "POST", "PUT", "PATCH", "DELETE"],
    }),
  )
  .use(serverTiming({ enabled: !env.isProduction }))
  .use(
    openapi({
      path: "/openapi",
      documentation: {
        info: {
          title: "API Weave",
          version: "0.1.0",
          description: [
            "API de Weave, application de rencontre fondée sur un principe simple :",
            `au plus ${MAX_ACTIVE_THREADS} fils à la fois, en cache uniquement, renouvelés à l'heure`,
            "que vous avez choisie.",
            "",
            "Il n'existe volontairement aucune route pour balayer des cartes, consulter",
            "la liste des personnes qui vous ont aimé, ou acheter de la visibilité.",
          ].join("\n"),
        },
        tags: [
          { name: "Authentification", description: "Connexion par code à usage unique." },
          { name: "Profil", description: "Sa fiche, ses fragments, son motif, ses critères." },
          { name: "Métier", description: "Les trois fils : lecture, réponse, dénouage." },
          { name: "Conversations", description: "Fils tissés et messages." },
          { name: "Appareils", description: "APNs, Live Activity, Apple Watch." },
          { name: "Offres", description: "Quatre abonnements, et le même à l'unité." },
          { name: "Sécurité", description: "Blocage, signalement, mise en pause." },
          { name: "Questions", description: "Bibliothèque de questions." },
          { name: "Médias", description: "Photos et voix, sous URL signée." },
        ],
      },
    }),
  )

  .onError(({ error, code, set, request }) => {
    if (error instanceof AppError) {
      set.status = error.status;
      return error.toJSON();
    }

    if (code === "VALIDATION") {
      set.status = 422;
      return { error: "validation", message: "Requête invalide.", details: String(error) };
    }
    if (code === "NOT_FOUND") {
      set.status = 404;
      return { error: "not_found", message: "Route inconnue." };
    }

    log.error("Erreur non gérée", {
      code,
      path: new URL(request.url).pathname,
      error: error instanceof Error ? error.message : String(error),
      stack: error instanceof Error ? error.stack : undefined,
    });
    set.status = 500;
    return { error: "internal", message: "Une erreur interne est survenue." };
  })

  .get(
    "/health",
    async ({ set }) => {
      const [dbOk, cacheOk] = await Promise.all([
        prisma.$queryRaw`SELECT 1`.then(() => true).catch(() => false),
        pingRedis(),
      ]);

      const healthy = dbOk && cacheOk;
      if (!healthy) set.status = 503;

      return {
        status: healthy ? "ok" : "degraded",
        database: { driver: env.db.driver, ok: dbOk },
        // Le cache n'est pas un confort dans Weave : sans lui, il n'y a pas de fils.
        cache: { ok: cacheOk, required: true },
        version: "0.1.0",
      };
    },
    { detail: { summary: "Sonde de santé", tags: ["Service"] } },
  )

  .use(authRoutes)
  .use(meRoutes)
  .use(loomRoutes)
  .use(threadRoutes)
  .use(deviceRoutes)
  .use(billingRoutes)
  .use(moderationRoutes)
  .use(promptRoutes)
  .use(mediaRoutes)
  .use(vitrine);

export type WeaveApp = typeof app;

/* ------------------------------------------------------------------ */
/* Démarrage                                                           */
/* ------------------------------------------------------------------ */

if (import.meta.main) {
  app.listen(env.port);

  log.info("API Weave démarrée", {
    port: env.port,
    mode: env.mode,
    database: env.db.driver,
    maxActiveThreads: MAX_ACTIVE_THREADS,
    openapi: `http://localhost:${env.port}/openapi`,
  });

  const shutdown = async (signal: string): Promise<void> => {
    log.info("Arrêt demandé", { signal });
    await app.stop();
    await disconnectPrisma();
    await disconnectRedis();
    process.exit(0);
  };

  process.on("SIGINT", () => void shutdown("SIGINT"));
  process.on("SIGTERM", () => void shutdown("SIGTERM"));
}
