/**
 * Client APNs — notifications distantes et Live Activities.
 *
 * Trois types d'envoi sont utilisés par Weave :
 *
 *  • `liveactivity` avec `event: "start"`  → démarre à distance la Live Activity
 *    « Métier » sur l'iPhone, à l'heure de tissage, grâce au jeton
 *    « push-to-start » d'ActivityKit ;
 *  • `liveactivity` avec `event: "update"` → met à jour le compte à rebours d'un
 *    fil, sans réveiller l'application ;
 *  • `alert`                               → notification classique.
 *
 * Aucune dépendance : le jeton d'autorisation ES256 est signé avec WebCrypto et
 * la requête part par `fetch`, qui négocie HTTP/2 (exigé par Apple) via ALPN.
 */
import { env } from "../env.ts";
import { log } from "./log.ts";

const HOSTS = {
  production: "https://api.push.apple.com",
  sandbox: "https://api.sandbox.push.apple.com",
} as const;

/** Apple accepte un jeton d'autorisation pendant une heure ; on le renouvelle avant. */
const TOKEN_TTL_MS = 45 * 60 * 1000;

let cachedToken: { value: string; issuedAt: number } | null = null;
let cachedKey: CryptoKey | null = null;

function base64url(input: ArrayBuffer | Uint8Array | string): string {
  const bytes =
    typeof input === "string"
      ? new TextEncoder().encode(input)
      : input instanceof Uint8Array
        ? input
        : new Uint8Array(input);
  return Buffer.from(bytes).toString("base64url");
}

/** Charge la clé .p8 (PKCS#8) émise par Apple et l'importe pour ES256. */
async function loadSigningKey(): Promise<CryptoKey> {
  if (cachedKey !== null) return cachedKey;

  const pem = await Bun.file(env.apns.keyPath!).text();
  const body = pem
    .replace(/-----BEGIN PRIVATE KEY-----/, "")
    .replace(/-----END PRIVATE KEY-----/, "")
    .replace(/\s+/g, "");
  const der = Buffer.from(body, "base64");

  cachedKey = await crypto.subtle.importKey(
    "pkcs8",
    der,
    { name: "ECDSA", namedCurve: "P-256" },
    false,
    ["sign"],
  );
  return cachedKey;
}

/** Jeton d'autorisation APNs (JWT ES256), mis en cache jusqu'à son renouvellement. */
async function authorizationToken(): Promise<string> {
  if (cachedToken !== null && Date.now() - cachedToken.issuedAt < TOKEN_TTL_MS) {
    return cachedToken.value;
  }

  const issuedAt = Date.now();
  const header = base64url(JSON.stringify({ alg: "ES256", kid: env.apns.keyId }));
  const payload = base64url(
    JSON.stringify({ iss: env.apns.teamId, iat: Math.floor(issuedAt / 1000) }),
  );
  const signature = await crypto.subtle.sign(
    { name: "ECDSA", hash: "SHA-256" },
    await loadSigningKey(),
    new TextEncoder().encode(`${header}.${payload}`),
  );

  const value = `${header}.${payload}.${base64url(signature)}`;
  cachedToken = { value, issuedAt };
  return value;
}

export type ApnsPushType = "alert" | "background" | "liveactivity";

export interface ApnsRequest {
  /** Jeton de l'appareil, ou jeton d'activité pour une Live Activity. */
  deviceToken: string;
  pushType: ApnsPushType;
  /** 10 = immédiat, 5 = économe en énergie. Apple impose 5 pour `background`. */
  priority?: 5 | 10;
  /** Sujet APNs. Les Live Activities exigent le suffixe `.push-type.liveactivity`. */
  topicSuffix?: string;
  /** Identifiant de dédoublonnage, pour ne pas envoyer deux fois le même état. */
  collapseId?: string;
  expiration?: number;
  payload: Record<string, unknown>;
}

export interface ApnsResult {
  ok: boolean;
  status: number;
  /** `Unregistered`, `BadDeviceToken`, `ExpiredToken`… quand Apple refuse l'envoi. */
  reason?: string;
  apnsId?: string;
}

/** Codes signalant un jeton devenu inutilisable : l'enregistrement doit être purgé. */
const DEAD_TOKEN_REASONS = new Set([
  "BadDeviceToken",
  "Unregistered",
  "ExpiredToken",
  "DeviceTokenNotForTopic",
]);

export function isDeadToken(result: ApnsResult): boolean {
  return result.reason !== undefined && DEAD_TOKEN_REASONS.has(result.reason);
}

/**
 * Envoie une notification. En l'absence de configuration APNs (développement
 * local, tests), l'envoi est journalisé puis ignoré : le reste du produit doit
 * pouvoir tourner sans certificat Apple.
 */
export async function sendApns(request: ApnsRequest): Promise<ApnsResult> {
  if (!env.apns.configured) {
    log.debug("APNs non configuré, envoi simulé", {
      pushType: request.pushType,
      topicSuffix: request.topicSuffix,
    });
    return { ok: true, status: 200, reason: "simulated" };
  }

  const url = `${HOSTS[env.apns.environment]}/3/device/${request.deviceToken}`;
  const topic =
    request.topicSuffix === undefined
      ? env.apns.bundleId
      : `${env.apns.bundleId}${request.topicSuffix}`;

  const headers: Record<string, string> = {
    authorization: `bearer ${await authorizationToken()}`,
    "apns-topic": topic,
    "apns-push-type": request.pushType,
    "apns-priority": String(request.priority ?? 10),
    "content-type": "application/json",
  };
  if (request.collapseId !== undefined) headers["apns-collapse-id"] = request.collapseId;
  if (request.expiration !== undefined) headers["apns-expiration"] = String(request.expiration);

  try {
    const response = await fetch(url, {
      method: "POST",
      headers,
      body: JSON.stringify(request.payload),
    });

    if (response.ok) {
      return {
        ok: true,
        status: response.status,
        apnsId: response.headers.get("apns-id") ?? undefined,
      };
    }

    const body = (await response.json().catch(() => ({}))) as { reason?: string };
    log.warn("APNs a refusé l'envoi", { status: response.status, reason: body.reason });
    return { ok: false, status: response.status, reason: body.reason };
  } catch (error) {
    log.error("APNs injoignable", { error: String(error) });
    return { ok: false, status: 0, reason: "network" };
  }
}

/* ------------------------------------------------------------------ */
/* Live Activity                                                       */
/* ------------------------------------------------------------------ */

const LIVE_ACTIVITY_TOPIC_SUFFIX = ".push-type.liveactivity";

/**
 * Démarre à distance la Live Activity « Métier », à partir du jeton
 * « push-to-start » remis par ActivityKit lors du premier lancement.
 */
export async function startLiveActivity(options: {
  pushToStartToken: string;
  contentState: Record<string, unknown>;
  attributes: Record<string, unknown>;
  attributesType: string;
  /** Fin de fenêtre de fraîcheur, en secondes epoch. */
  staleDate: number;
  /** Fin automatique de l'activité, en secondes epoch. */
  dismissalDate?: number;
  alert?: { title: string; body: string };
}): Promise<ApnsResult> {
  return sendApns({
    deviceToken: options.pushToStartToken,
    pushType: "liveactivity",
    priority: 10,
    topicSuffix: LIVE_ACTIVITY_TOPIC_SUFFIX,
    payload: {
      aps: {
        timestamp: Math.floor(Date.now() / 1000),
        event: "start",
        "content-state": options.contentState,
        "attributes-type": options.attributesType,
        attributes: options.attributes,
        "stale-date": options.staleDate,
        ...(options.dismissalDate !== undefined ? { "dismissal-date": options.dismissalDate } : {}),
        ...(options.alert !== undefined
          ? { alert: { title: options.alert.title, body: options.alert.body } }
          : {}),
      },
    },
  });
}

/** Met à jour une Live Activity en cours. */
export async function updateLiveActivity(options: {
  updateToken: string;
  contentState: Record<string, unknown>;
  staleDate: number;
  /** Sert de clé de dédoublonnage : un seul état en vol par fil. */
  collapseId?: string;
  /** 5 pour une mise à jour discrète, 10 pour un changement que l'on veut immédiat. */
  priority?: 5 | 10;
  alert?: { title: string; body: string };
}): Promise<ApnsResult> {
  return sendApns({
    deviceToken: options.updateToken,
    pushType: "liveactivity",
    priority: options.priority ?? 5,
    topicSuffix: LIVE_ACTIVITY_TOPIC_SUFFIX,
    collapseId: options.collapseId,
    payload: {
      aps: {
        timestamp: Math.floor(Date.now() / 1000),
        event: "update",
        "content-state": options.contentState,
        "stale-date": options.staleDate,
        ...(options.alert !== undefined
          ? { alert: { title: options.alert.title, body: options.alert.body } }
          : {}),
      },
    },
  });
}

/** Termine une Live Activity (tous les fils dénoués, session close). */
export async function endLiveActivity(options: {
  updateToken: string;
  contentState: Record<string, unknown>;
  /** Instant de disparition de la bannière ; immédiat si omis. */
  dismissalDate?: number;
}): Promise<ApnsResult> {
  return sendApns({
    deviceToken: options.updateToken,
    pushType: "liveactivity",
    priority: 10,
    topicSuffix: LIVE_ACTIVITY_TOPIC_SUFFIX,
    payload: {
      aps: {
        timestamp: Math.floor(Date.now() / 1000),
        event: "end",
        "content-state": options.contentState,
        ...(options.dismissalDate !== undefined
          ? { "dismissal-date": options.dismissalDate }
          : { "dismissal-date": Math.floor(Date.now() / 1000) }),
      },
    },
  });
}
