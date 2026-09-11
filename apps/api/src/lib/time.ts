/**
 * Calculs d'horaires : heure de tissage, délais de regarnissage, distances.
 *
 * L'heure de tissage est exprimée dans le fuseau de l'utilisateur : c'est un
 * rendez-vous quotidien, pas une notification opportuniste.
 */
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

/** Secondes restantes avant minuit, dans un fuseau donné. */
export function secondsUntilMidnight(timezone: string, from: Date = new Date()): number {
  const heure = localHour(timezone, from);
  const minutes = Number.parseInt(
    new Intl.DateTimeFormat("fr-FR", { timeZone: timezone, minute: "numeric" }).format(from),
    10,
  );
  return (24 - heure) * 3600 - minutes * 60;
}

/**
 * Boîte englobante autour d'un point, pour préfiltrer en SQL sans extension
 * géospatiale : le schéma doit rester identique sur PostgreSQL et SQLite. La
 * distance exacte est recalculée ensuite en mémoire.
 */
export function boundingBox(lat: number, lon: number, radiusKm: number) {
  const latDelta = radiusKm / 111;
  const lonDelta = radiusKm / Math.max(1, 111 * Math.cos((lat * Math.PI) / 180));
  return {
    minLat: lat - latDelta,
    maxLat: lat + latDelta,
    minLon: lon - lonDelta,
    maxLon: lon + lonDelta,
  };
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
