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
import { MAX_OPEN_PLANS, REQUESTS_PER_DAY_FLOOR } from "@weave/contracts";
import { env } from "./env.ts";
import { AppError } from "./lib/errors.ts";
import { log } from "./lib/log.ts";
import { disconnectPrisma, prisma } from "./lib/prisma.ts";
import { disconnectRedis, pingRedis, withTimeout, PROBE_TIMEOUT_MS } from "./lib/redis.ts";
import { authRoutes } from "./modules/auth.routes.ts";
import { billingRoutes } from "./modules/billing.routes.ts";
import { conversationRoutes } from "./modules/conversations.routes.ts";
import { deviceRoutes } from "./modules/devices.routes.ts";
import { mediaRoutes } from "./modules/media.routes.ts";
import { meRoutes } from "./modules/me.routes.ts";
import { moderationRoutes } from "./modules/moderation.routes.ts";
import { planRoutes } from "./modules/plans.routes.ts";
import { requestRoutes } from "./modules/requests.routes.ts";

/**
 * Site vitrine compilé, servi par le même processus que l'API.
 *
 * Le site est statique et peu visité : lui dédier un second processus
 * doublerait le coût d'hébergement sans rien apporter. En développement,
 * `WEB_DIST_PATH` n'est pas défini — Vite s'en charge — et ce greffon
 * n'enregistre alors aucune route.
 */
const vitrineDisponible =
  env.webDist !== undefined && (await Bun.file(`${env.webDist}/index.html`).exists());

if (env.webDist !== undefined && !vitrineDisponible) {
  // Un site absent ne doit pas empêcher l'API de démarrer : elle rend un
  // service autonome, dont l'application iOS dépend.
  log.warn("Site vitrine introuvable, l'API démarre sans lui", { chemin: env.webDist });
}

const vitrine = !vitrineDisponible
  ? new Elysia({ name: "weave/vitrine-absente" })
  : new Elysia({ name: "weave/vitrine" })
      .use(
        await staticPlugin({
          assets: env.webDist!,
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
            "API de Weave : on ne publie pas un profil, on publie un plan pour les",
            "jours qui viennent, et les autres demandent à venir — en écrivant pourquoi.",
            "",
            `Deux invariants tiennent le produit. On ne peut pas arroser : au plus ${MAX_OPEN_PLANS} plans`,
            `ouverts, et un nombre borné de demandes par jour (au minimum ${REQUESTS_PER_DAY_FLOOR}) à TOUS les`,
            "paliers, socle gratuit compris. Et on ne peut pas acheter de visibilité : le",
            "fil est trié par imminence puis par proximité, et par rien d'autre.",
            "",
            "Il n'existe volontairement aucune route pour balayer des cartes, « aimer »",
            "quelqu'un, consulter qui vous a remarqué, ou faire remonter un plan.",
          ].join("\n"),
        },
        tags: [
          { name: "Authentification", description: "Connexion par code à usage unique." },
          { name: "Profil", description: "Sa fiche — une ville, un genre, une phrase." },
          { name: "Plans", description: "Publier un plan, lire le fil, annuler." },
          { name: "Demandes", description: "Demander à venir, accepter, refuser." },
          { name: "Conversations", description: "Ce qui s'ouvre après un oui." },
          { name: "Appareils", description: "APNs, Live Activity, Apple Watch." },
          { name: "Offres", description: "Quatre abonnements, et les mêmes à l'unité." },
          { name: "Sécurité", description: "Blocage, signalement, mise en pause." },
          { name: "Médias", description: "Photos, sous URL signée." },
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
      // Les deux sondes sont bornées : une dépendance qui ne répond pas doit
      // être rapportée comme telle, jamais faire attendre la réponse.
      const [dbOk, cacheOk] = await Promise.all([
        withTimeout(
          prisma.$queryRaw`SELECT 1`.then(() => true),
          PROBE_TIMEOUT_MS,
        ),
        pingRedis(),
      ]);

      const healthy = dbOk && cacheOk;
      if (!healthy) set.status = 503;

      return {
        status: healthy ? "ok" : "degraded",
        database: { driver: env.db.driver, ok: dbOk },
        // Le cache n'est pas un confort dans Weave : le quota de demandes n'y
        // vit qu'ici, et sans lui l'invariant central ne tient plus.
        cache: { ok: cacheOk, required: true },
        version: "0.1.0",
      };
    },
    { detail: { summary: "Sonde de santé", tags: ["Service"] } },
  )

  .use(authRoutes)
  .use(meRoutes)
  .use(planRoutes)
  .use(requestRoutes)
  .use(conversationRoutes)
  .use(deviceRoutes)
  .use(billingRoutes)
  .use(moderationRoutes)
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
    maxOpenPlans: MAX_OPEN_PLANS,
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
