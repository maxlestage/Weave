/**
 * Calculs d'horaires : heure de tissage, délais de regarnissage, distances.
 *
 * L'heure de tissage est exprimée dans le fuseau de l'utilisateur : c'est un
 * rendez-vous quotidien, pas une notification opportuniste.
 */
import { WEAVING_HOURS } from "@weave/contracts";

/** Heure locale courante (0-23) dans un fuseau donné. */
export function localHour(timezone: string, at: Date = new Date()): number {
  const formatter = new Intl.DateTimeFormat("fr-FR", {
    timeZone: timezone,
    hour: "numeric",
    hour12: false,
  });
  return Number.parseInt(formatter.format(at), 10);
}

/** Jour local au format AAAA-MM-JJ, pour les clés de cache journalières. */
export function localDay(timezone: string, at: Date = new Date()): string {
  return new Intl.DateTimeFormat("fr-CA", {
    timeZone: timezone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(at);
}

/**
 * Décalage d'un fuseau par rapport à UTC, en minutes, à un instant donné.
 * Passe par `Intl` afin de tenir compte de l'heure d'été sans dépendance.
 */
function offsetMinutes(timezone: string, at: Date): number {
  const parts = new Intl.DateTimeFormat("en-US", {
    timeZone: timezone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  }).formatToParts(at);

  const get = (type: string): number =>
    Number.parseInt(parts.find((p) => p.type === type)?.value ?? "0", 10);

  const asUtc = Date.UTC(
    get("year"),
    get("month") - 1,
    get("day"),
    get("hour") % 24,
    get("minute"),
    get("second"),
  );
  return (asUtc - at.getTime()) / 60_000;
}

/** Prochaine occurrence de l'heure de tissage, en instant absolu. */
export function nextWeavingAt(
  timezone: string,
  weavingHour: number,
  from: Date = new Date(),
): Date {
  const offset = offsetMinutes(timezone, from);
  const local = new Date(from.getTime() + offset * 60_000);

  const target = new Date(local);
  target.setUTCHours(weavingHour, 0, 0, 0);
  if (target.getTime() <= local.getTime()) target.setUTCDate(target.getUTCDate() + 1);

  // Reconversion vers l'instant absolu, en réévaluant le décalage à la date visée
  // pour rester correct lors des changements d'heure.
  const provisional = new Date(target.getTime() - offset * 60_000);
  const correctedOffset = offsetMinutes(timezone, provisional);
  return new Date(target.getTime() - correctedOffset * 60_000);
}

/** Vérifie qu'une heure fait partie des créneaux de tissage proposés. */
export function isValidWeavingHour(hour: number): boolean {
  return (WEAVING_HOURS as readonly number[]).includes(hour);
}

/** Date de regarnissage d'une place libérée, selon le délai du palier. */
export function refillAt(delayMinutes: number, from: Date = new Date()): Date {
  return new Date(from.getTime() + delayMinutes * 60_000);
}

/**
 * Distance orthodromique en kilomètres, arrondie à l'entier.
 * Weave n'expose jamais mieux que le kilomètre : les coordonnées stockées sont
 * elles-mêmes déjà arrondies.
 */
export function distanceKm(aLat: number, aLon: number, bLat: number, bLon: number): number {
  const R = 6371;
  const toRad = (deg: number) => (deg * Math.PI) / 180;
  const dLat = toRad(bLat - aLat);
  const dLon = toRad(bLon - aLon);
  const h =
    Math.sin(dLat / 2) ** 2 +
    Math.cos(toRad(aLat)) * Math.cos(toRad(bLat)) * Math.sin(dLon / 2) ** 2;
  return Math.round(2 * R * Math.asin(Math.min(1, Math.sqrt(h))));
}

/** Âge en années révolues. */
export function ageFrom(birthDate: Date, at: Date = new Date()): number {
  let age = at.getUTCFullYear() - birthDate.getUTCFullYear();
  const monthDiff = at.getUTCMonth() - birthDate.getUTCMonth();
  if (monthDiff < 0 || (monthDiff === 0 && at.getUTCDate() < birthDate.getUTCDate())) age -= 1;
  return age;
}
