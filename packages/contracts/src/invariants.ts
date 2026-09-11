/**
 * Invariants produit de Weave.
 *
 * Ces constantes ne sont pas des « réglages » : elles définissent l'identité du
 * produit. Toute modification doit passer par une décision produit explicite,
 * car plusieurs d'entre elles sont ce qui distingue juridiquement et
 * fonctionnellement Weave des applications de rencontre existantes.
 */

/**
 * INVARIANT CENTRAL — « 3 profils en cache uniquement ».
 *
 * Un utilisateur ne détient jamais plus de trois fils actifs, et le contenu de
 * ces fils (photos, fragments, motifs) n'existe QUE dans Redis, avec un TTL.
 * Aucune table de la base relationnelle ne contient de copie d'un profil
 * proposé : la base ne garde qu'un registre minimal (identifiants + horodatage
 * + issue) permettant de ne pas re-proposer deux fois la même personne.
 *
 * Aucun palier d'abonnement ne relève ce plafond. Les offres payantes agissent
 * sur la VITESSE de remplacement d'un fil dénoué, jamais sur le nombre de fils
 * détenus simultanément.
 */
export const MAX_ACTIVE_THREADS = 3 as const;

/** Durée de vie d'un fil non engagé, en secondes (24 h). */
export const THREAD_TTL_SECONDS = 24 * 60 * 60;

/** Durée de vie maximale d'un fil après achat de « Prolonge » (48 h au total). */
export const THREAD_TTL_MAX_SECONDS = 48 * 60 * 60;

/** TTL du cache de session/identité (15 min). */
export const SESSION_CACHE_TTL_SECONDS = 15 * 60;

/** TTL du cache de composition (candidats pré-calculés), en secondes. */
export const CANDIDATE_POOL_TTL_SECONDS = 30 * 60;

/** Nombre de fragments composant la trame d'un fil. */
export const FRAGMENTS_PER_THREAD = 3 as const;

/** Nombre de mots-clés du « motif » d'un profil. */
export const MOTIF_TAGS = 5 as const;

/** Durée maximale d'un fragment vocal, en secondes. */
export const VOICE_FRAGMENT_MAX_SECONDS = 8;

/** Longueur minimale d'une réponse à un fragment (pas de réaction binaire). */
export const RESPONSE_MIN_CHARS = 12;

/** Longueur maximale d'une réponse à un fragment. */
export const RESPONSE_MAX_CHARS = 480;

/**
 * Paliers de révélation progressive de la photo, en pourcentage de netteté.
 * Index = nombre d'échanges mutuels aboutis sur le fil.
 */
export const REVEAL_STEPS = [0, 33, 66, 100] as const;

/** Âge minimum requis. */
export const MIN_AGE = 18;

/** Nombre d'heures de tissage proposées (créneaux d'ouverture quotidiens). */
export const WEAVING_HOURS = [8, 12, 18, 21] as const;

/** Délai de rétention des messages après dénouage d'un fil (RGPD), en jours. */
export const MESSAGE_RETENTION_DAYS = 90;

/** Délai de purge d'un compte supprimé, en jours. */
export const ACCOUNT_PURGE_DAYS = 30;
