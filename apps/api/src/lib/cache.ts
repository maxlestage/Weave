/**
 * Cache Redis : le fil composé, le quota de demandes, les résumés d'appareils.
 *
 * Deux choses comptent ici.
 *
 * Le **fil** est recomposé et mis en cache quelques minutes : il dépend de la
 * position, des critères et de l'heure, et se périme donc vite. Rien de ce
 * qu'il contient n'est une source de vérité — les plans vivent en base, c'est
 * leur auteur qui les a écrits.
 *
 * Le **quota de demandes** vit en revanche uniquement ici, avec une expiration
 * à minuit. C'est l'invariant central du produit : on ne peut pas arroser.
 * Il est décrémenté de façon atomique, sans quoi deux requêtes simultanées
 * pourraient dépenser la même demande.
 */
import { redis } from "./redis.ts";

const NS = "weave:v2";

export const keys = {
  /** Fil composé pour un compte. */
  feed: (accountId: string) => `${NS}:feed:${accountId}`,
  /** Demandes déjà envoyées aujourd'hui. Expire à minuit, heure locale. */
  requestsUsed: (accountId: string, day: string) => `${NS}:req:${accountId}:${day}`,
  /** « Renforts » déjà appliqués aujourd'hui. Expire à minuit, heure locale. */
  renforts: (accountId: string, day: string) => `${NS}:renfort:${accountId}:${day}`,
  /** Identité résumée, pour éviter un aller-retour base à chaque requête. */
  identity: (accountId: string) => `${NS}:me:${accountId}`,
  /** Dernier état poussé vers la Live Activity. */
  liveActivity: (accountId: string) => `${NS}:la:${accountId}`,
  /** Résumé compact destiné à watchOS. */
  watch: (accountId: string) => `${NS}:watch:${accountId}`,
  /** Compteur de limitation de débit. */
  rateLimit: (bucket: string, subject: string) => `${NS}:rl:${bucket}:${subject}`,
} as const;

/**
 * Consomme une demande du quota du jour, de façon atomique.
 *
 * Le script incrémente puis compare : si le plafond est dépassé, il annule son
 * propre incrément et renvoie -1. Une vérification suivie d'une écriture en
 * deux temps laisserait passer deux demandes concurrentes sur la dernière
 * place.
 */
const CONSUME_REQUEST_LUA = `
local plafond = tonumber(ARGV[1])
local expiration = tonumber(ARGV[2])

local utilisees = redis.call('INCR', KEYS[1])
if utilisees == 1 then
  redis.call('EXPIRE', KEYS[1], expiration)
end

if utilisees > plafond then
  redis.call('DECR', KEYS[1])
  return -1
end

return plafond - utilisees
`;

/**
 * Renvoie le nombre de demandes restantes après consommation, ou `null` si le
 * quota du jour est épuisé.
 */
export async function consumeRequest(
  accountId: string,
  day: string,
  quota: number,
  secondsUntilMidnight: number,
): Promise<number | null> {
  const reponse = await redis.send("EVAL", [
    CONSUME_REQUEST_LUA,
    "1",
    keys.requestsUsed(accountId, day),
    String(quota),
    String(Math.max(60, secondsUntilMidnight)),
  ]);
  const restantes = Number(reponse);
  return restantes < 0 ? null : restantes;
}

/**
 * Applique un « Renfort », de façon atomique et bornée.
 *
 * Le même script que le quota, employé à l'envers : il incrémente, et annule
 * son incrément si le plafond journalier de renforts est franchi. Sans ce
 * second plafond, l'argent lèverait l'invariant.
 *
 * Renvoie le nombre de renforts appliqués aujourd'hui, ou `null` si le plafond
 * est atteint.
 */
export async function applyRenfort(
  accountId: string,
  day: string,
  maximum: number,
  secondsUntilMidnight: number,
): Promise<number | null> {
  const reponse = await redis.send("EVAL", [
    CONSUME_REQUEST_LUA,
    "1",
    keys.renforts(accountId, day),
    String(maximum),
    String(Math.max(60, secondsUntilMidnight)),
  ]);
  const restants = Number(reponse);
  return restants < 0 ? null : maximum - restants;
}

/**
 * Libère une place de renfort réservée mais non payée. Ne descend jamais sous
 * zéro, comme le remboursement d'une demande.
 */
export async function releaseRenfort(accountId: string, day: string): Promise<void> {
  const cle = keys.renforts(accountId, day);
  const brut = await redis.get(cle);
  if (brut !== null && Number(brut) > 0) await redis.decr(cle);
}

/** Renforts appliqués aujourd'hui. */
export async function renfortsToday(accountId: string, day: string): Promise<number> {
  const brut = await redis.get(keys.renforts(accountId, day));
  return brut === null ? 0 : Number(brut);
}

/** Demandes restantes aujourd'hui, sans rien consommer. */
export async function requestsLeft(accountId: string, day: string, quota: number): Promise<number> {
  const brut = await redis.get(keys.requestsUsed(accountId, day));
  const utilisees = brut === null ? 0 : Number(brut);
  return Math.max(0, quota - utilisees);
}

/**
 * Rend une demande au quota — lorsqu'elle est retirée avant d'avoir été lue.
 * Ne descend jamais sous zéro.
 */
export async function refundRequest(accountId: string, day: string): Promise<void> {
  const cle = keys.requestsUsed(accountId, day);
  const brut = await redis.get(cle);
  if (brut !== null && Number(brut) > 0) await redis.decr(cle);
}

/** Invalide le fil d'un compte : critères modifiés, blocage, nouveau plan. */
export async function invalidateFeed(accountId: string): Promise<void> {
  await redis.del(keys.feed(accountId));
}

/**
 * Invalide le fil de tout le monde. Appelé à la publication d'un plan : il
 * doit apparaître sans attendre l'expiration des fils déjà composés.
 *
 * `SCAN` plutôt que `KEYS` : cette dernière bloque le serveur le temps du
 * parcours, ce qui est acceptable sur un jeu de développement et ne l'est plus
 * en production.
 */
export async function invalidateAllFeeds(): Promise<number> {
  let curseur = "0";
  let supprimees = 0;

  do {
    const reponse = (await redis.send("SCAN", [
      curseur,
      "MATCH",
      `${NS}:feed:*`,
      "COUNT",
      "200",
    ])) as [string, string[]];

    curseur = reponse[0];
    const lot = reponse[1];
    if (lot.length > 0) {
      await redis.send("DEL", lot);
      supprimees += lot.length;
    }
  } while (curseur !== "0");

  return supprimees;
}
