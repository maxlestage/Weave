/**
 * Configuration d'exécution, lue une seule fois au démarrage et validée
 * strictement : une variable manquante en production arrête le processus plutôt
 * que de laisser tourner un service à moitié configuré.
 */

type Mode = "development" | "test" | "production";

function required(name: string, fallbackInDev?: string): string {
  const value = process.env[name];
  if (value !== undefined && value !== "") return value;
  if (fallbackInDev !== undefined && process.env.NODE_ENV !== "production") return fallbackInDev;
  throw new Error(`Variable d'environnement manquante : ${name}`);
}

function optional(name: string): string | undefined {
  const value = process.env[name];
  return value === undefined || value === "" ? undefined : value;
}

function int(name: string, fallback: number): number {
  const raw = process.env[name];
  if (raw === undefined || raw === "") return fallback;
  const parsed = Number.parseInt(raw, 10);
  if (Number.isNaN(parsed)) throw new Error(`${name} doit être un entier`);
  return parsed;
}

/**
 * Résout un chemin SQLite relatif par rapport au paquet `apps/api`, et non par
 * rapport au répertoire courant : les tests et les scripts peuvent ainsi être
 * lancés depuis la racine du dépôt comme depuis le paquet lui-même.
 */
function resolveSqliteUrl(raw: string): string {
  if (!raw.startsWith("file:")) return raw;
  const path = raw.slice("file:".length);
  if (path.startsWith("/") || path === ":memory:") return raw;
  const packageRoot = new URL("..", import.meta.url).pathname.replace(/\/$/, "");
  return `file:${packageRoot}/${path.replace(/^\.\//, "")}`;
}

const mode = (process.env.NODE_ENV ?? "development") as Mode;
const driver = (process.env.WEAVE_DB ?? (mode === "production" ? "postgres" : "sqlite")).toLowerCase();

if (driver !== "postgres" && driver !== "sqlite") {
  throw new Error(`WEAVE_DB doit valoir "postgres" ou "sqlite" (reçu : ${driver})`);
}
if (driver === "sqlite" && mode === "production") {
  throw new Error(
    "SQLite est réservé au développement. Définissez WEAVE_DB=postgres en production.",
  );
}

export const env = {
  mode,
  isProduction: mode === "production",
  port: int("PORT", 3000),
  webOrigin: process.env.PUBLIC_WEB_ORIGIN ?? "http://localhost:5173",

  db: {
    driver: driver as "postgres" | "sqlite",
    postgresUrl: driver === "postgres" ? required("DATABASE_URL") : optional("DATABASE_URL"),
    sqliteUrl: resolveSqliteUrl(process.env.DATABASE_URL_SQLITE ?? "file:./prisma/dev.db"),
  },

  redis: {
    url: process.env.REDIS_URL ?? "redis://localhost:6379",
  },

  auth: {
    jwtSecret: required("JWT_SECRET", "secret-de-developpement-non-utilisable-en-production"),
    accessTtlSeconds: int("ACCESS_TOKEN_TTL_SECONDS", 900),
    refreshTtlDays: int("REFRESH_TOKEN_TTL_DAYS", 60),
  },

  media: {
    baseUrl: process.env.MEDIA_BASE_URL ?? "http://localhost:3000/media",
    signingSecret: required("MEDIA_SIGNING_SECRET", "secret-media-developpement"),
    signedUrlTtlSeconds: int("MEDIA_URL_TTL_SECONDS", 600),
  },

  apns: {
    keyId: optional("APNS_KEY_ID"),
    teamId: optional("APNS_TEAM_ID"),
    bundleId: process.env.APNS_BUNDLE_ID ?? "com.weave.app",
    keyPath: optional("APNS_KEY_PATH"),
    environment: (process.env.APNS_ENVIRONMENT ?? "sandbox") as "sandbox" | "production",
    get configured(): boolean {
      return Boolean(
        process.env.APNS_KEY_ID && process.env.APNS_TEAM_ID && process.env.APNS_KEY_PATH,
      );
    },
  },

  appStore: {
    issuerId: optional("APPSTORE_ISSUER_ID"),
    keyId: optional("APPSTORE_KEY_ID"),
    keyPath: optional("APPSTORE_KEY_PATH"),
    environment: (process.env.APPSTORE_ENVIRONMENT ?? "sandbox") as "sandbox" | "production",
    get configured(): boolean {
      return Boolean(
        process.env.APPSTORE_ISSUER_ID &&
          process.env.APPSTORE_KEY_ID &&
          process.env.APPSTORE_KEY_PATH,
      );
    },
  },
} as const;

if (env.isProduction && env.auth.jwtSecret.length < 32) {
  throw new Error("JWT_SECRET doit faire au moins 32 caractères en production.");
}
