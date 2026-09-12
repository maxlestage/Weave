/**
 * Configuration d'exécution, lue une seule fois au démarrage et validée
 * strictement : une variable manquante en production arrête le processus plutôt
 * que de laisser tourner un service à moitié configuré.
 *
 * Les problèmes sont TOUS rassemblés avant d'arrêter, et non levés au premier
 * rencontré. Sur un hébergeur, chaque démarrage raté coûte un déploiement et
 * une lecture de journal : apprendre les variables manquantes une par une fait
 * autant d'allers-retours qu'il en manque.
 */

type Mode = "development" | "test" | "production";

/*
 * Sur un hébergeur, l'absence de NODE_ENV ne doit surtout pas valoir
 * « développement ». Heroku définit `DYNO` sur chaque dyno : s'y fier écarte le
 * pire des cas — un service qui démarre en ligne avec les réglages du
 * développement, se rabat silencieusement sur SQLite, puis échoue sur un module
 * absent du slug au lieu de dire ce qui manque. Le diagnostic coûtait alors un
 * déploiement et une lecture de journal pour une variable oubliée.
 */
const surUnHebergeur = (process.env.DYNO ?? "") !== "";
const mode = (process.env.NODE_ENV ?? (surUnHebergeur ? "production" : "development")) as Mode;

/** Ce qui empêche de démarrer, accumulé puis rapporté d'un bloc. */
const problemes: { variable: string; raison: string; remede: string }[] = [];

function required(name: string, fallbackInDev?: string): string {
  const value = process.env[name];
  if (value !== undefined && value !== "") return value;
  if (fallbackInDev !== undefined && mode !== "production") return fallbackInDev;
  problemes.push({
    variable: name,
    raison: "absente",
    remede: REMEDES[name] ?? "à définir dans la configuration de l'application",
  });
  return "";
}

/** Comment obtenir chaque valeur, dit une fois plutôt que cherché dans un guide. */
const REMEDES: Readonly<Record<string, string>> = {
  JWT_SECRET: "une chaîne aléatoire d'au moins 32 caractères — `openssl rand -base64 32`",
  MEDIA_SIGNING_SECRET: "une autre chaîne aléatoire — `openssl rand -base64 32`",
  DATABASE_URL: "fournie par l'add-on Heroku Postgres, à attacher à l'application",
};

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

const driver = (
  process.env.WEAVE_DB ?? (mode === "production" ? "postgres" : "sqlite")
).toLowerCase();

if (driver !== "postgres" && driver !== "sqlite") {
  throw new Error(`WEAVE_DB doit valoir "postgres" ou "sqlite" (reçu : ${driver})`);
}
if (driver === "sqlite" && mode === "production") {
  problemes.push({
    variable: "WEAVE_DB",
    raison: "vaut « sqlite », réservé au développement",
    remede: "définir WEAVE_DB=postgres",
  });
}

export const env = {
  mode,
  isProduction: mode === "production",
  port: int("PORT", 3000),
  webOrigin: process.env.PUBLIC_WEB_ORIGIN ?? "http://localhost:5173",
  /**
   * Répertoire du site vitrine compilé. Quand il est présent, l'API le sert
   * elle-même : un seul processus, donc un seul dyno à payer. En développement
   * le site est servi par Vite et cette variable reste vide.
   */
  webDist: optional("WEB_DIST_PATH"),

  db: {
    driver: driver as "postgres" | "sqlite",
    postgresUrl: driver === "postgres" ? required("DATABASE_URL") : optional("DATABASE_URL"),
    sqliteUrl: resolveSqliteUrl(process.env.DATABASE_URL_SQLITE ?? "file:./prisma/dev.db"),
    /**
     * Même compromis que pour Redis : Heroku Postgres présente un certificat
     * auto-signé. La connexion reste chiffrée, l'identité du serveur n'est pas
     * vérifiée. Réservé aux bases jointes par le réseau interne de l'hébergeur.
     */
    sslInsecure: process.env.DATABASE_SSL_INSECURE === "true",
  },

  redis: {
    // Heroku Key-Value Store expose deux adresses : `REDIS_TLS_URL` (chiffrée,
    // obligatoire sur les petits plans) et `REDIS_URL`. On préfère la première
    // quand elle existe.
    url: process.env.REDIS_TLS_URL ?? process.env.REDIS_URL ?? "redis://localhost:6379",
    /**
     * Accepte un certificat serveur non vérifiable.
     *
     * Nécessaire chez Heroku, dont le magasin clé-valeur présente un certificat
     * auto-signé : sans cela la connexion échoue. Le trafic reste chiffré, mais
     * l'identité du serveur n'est pas vérifiée — à n'activer que sur un réseau
     * interne d'hébergeur, jamais pour joindre un cache à travers l'internet
     * public.
     */
    tlsInsecure: process.env.REDIS_TLS_INSECURE === "true",
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

if (env.isProduction && env.auth.jwtSecret !== "" && env.auth.jwtSecret.length < 32) {
  problemes.push({
    variable: "JWT_SECRET",
    raison: `trop court (${env.auth.jwtSecret.length} caractères, minimum 32)`,
    remede: REMEDES.JWT_SECRET!,
  });
}

/**
 * Rapport de démarrage.
 *
 * Écrit sur la sortie d'erreur, seule voie que l'on soit sûr de retrouver dans
 * le journal d'un hébergeur, et sous une forme qui se lit sur un téléphone :
 * une ligne par variable, le remède à côté du problème.
 */
if (problemes.length > 0) {
  const lignes = [
    "",
    "  Weave ne peut pas démarrer : la configuration est incomplète.",
    "",
    ...problemes.flatMap((probleme) => [
      `  • ${probleme.variable} — ${probleme.raison}`,
      `      ${probleme.remede}`,
    ]),
    "",
    "  Sur Heroku : tableau de bord → Settings → Config Vars,",
    "  ou le workflow « Heroku — configurer les variables » depuis GitHub.",
    "",
  ];
  console.error(lignes.join("\n"));
  process.exit(1);
}

/**
 * Avertissements : ce qui n'empêche pas de démarrer, mais se verra aussitôt.
 *
 * Ils ne sont émis que sur un dyno Heroku (`DYNO` est posé par la plateforme),
 * parce que ce sont ses particularités qu'ils décrivent. Ailleurs, exiger ces
 * réglages serait faux — et dangereux : accepter un certificat non vérifié n'a
 * de sens que sur un réseau interne d'hébergeur.
 */
if (process.env.DYNO !== undefined) {
  const avertissements: { sujet: string; raison: string; remede: string }[] = [];

  if (!env.db.sslInsecure) {
    avertissements.push({
      sujet: "DATABASE_SSL_INSECURE",
      raison: "la base d'Heroku présente un certificat auto-signé sur son réseau interne",
      remede: "définir DATABASE_SSL_INSECURE=true, sans quoi la base restera injoignable",
    });
  }

  // Uniquement si l'adresse est chiffrée : sur une adresse en clair, la
  // question ne se pose pas.
  if (env.redis.url.startsWith("rediss://") && !env.redis.tlsInsecure) {
    avertissements.push({
      sujet: "REDIS_TLS_INSECURE",
      raison: "le magasin clé-valeur d'Heroku présente lui aussi un certificat auto-signé",
      remede: "définir REDIS_TLS_INSECURE=true, sans quoi le cache restera injoignable",
    });
  }

  if (env.webOrigin.includes("localhost")) {
    avertissements.push({
      sujet: "PUBLIC_WEB_ORIGIN",
      raison: "pointe encore sur localhost",
      remede:
        "définir l'adresse publique du site, sinon les appels depuis le navigateur seront refusés",
    });
  }

  if (env.media.baseUrl.includes("localhost")) {
    avertissements.push({
      sujet: "MEDIA_BASE_URL",
      raison: "pointe encore sur localhost",
      remede: "définir l'adresse publique des médias, sinon les photos ne s'afficheront pas",
    });
  }

  if (avertissements.length > 0) {
    console.warn(
      [
        "",
        "  Weave démarre, mais la configuration est incomplète :",
        "",
        ...avertissements.flatMap((a) => [`  • ${a.sujet} — ${a.raison}`, `      ${a.remede}`]),
        "",
        "  Le workflow « Heroku — configurer les variables » pose tout cela.",
        "",
      ].join("\n"),
    );
  }
}
