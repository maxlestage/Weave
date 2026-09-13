/**
 * Invariants produit de Weave.
 *
 * Weave ne présente pas des profils : il présente des **plans**. Quelqu'un
 * publie ce qu'il compte faire dans les jours qui viennent, les autres
 * demandent à venir en écrivant pourquoi. La conversation démarre donc sur
 * quelque chose à faire, jamais sur « salut ça va ».
 *
 * Ces constantes ne sont pas des réglages : elles définissent le produit, et
 * plusieurs d'entre elles sont ce qui le distingue des applications de
 * rencontre existantes.
 */

/**
 * INVARIANT CENTRAL — on ne peut pas arroser.
 *
 * Un plan se demande en écrivant, et le nombre de demandes par jour est borné.
 * C'est la contrainte qui change la nature de ce qu'on écrit : quand on ne peut
 * en envoyer que quelques-unes, on les écrit vraiment.
 *
 * Le plafond dépend du palier (voir `catalog.ts`), mais il existe à tous les
 * paliers — y compris au plus cher. Personne ne peut acheter le droit
 * d'envoyer la même phrase à cinquante personnes.
 */
export const REQUESTS_PER_DAY_FLOOR = 5 as const;

/**
 * Un « Renfort » acheté ajoute des demandes à la journée en cours — mais leur
 * nombre est lui-même borné. Sans ce second plafond, l'argent lèverait
 * l'invariant, et « on ne peut pas arroser » deviendrait « on ne peut pas
 * arroser gratuitement », ce qui n'est pas la même règle.
 */
export const RENFORT_GRANT = 5 as const;
export const MAX_RENFORTS_PER_DAY = 2 as const;

/**
 * SECOND INVARIANT — on n'achète pas de visibilité.
 *
 * Aucun palier, aucun achat ne fait remonter un plan devant les autres. Le
 * classement du fil ne dépend que de la proximité, de la date et des critères
 * de la personne qui regarde. C'est aussi ce qui éloigne Weave des mécaniques
 * de mise en avant payante du secteur.
 */
export const PAID_VISIBILITY = false as const;

/** Plans ouverts simultanément par personne. Une intention, pas un catalogue. */
export const MAX_OPEN_PLANS = 3 as const;

/** Longueur minimale d'une demande : on écrit, on ne clique pas. */
export const REQUEST_MIN_CHARS = 20;
export const REQUEST_MAX_CHARS = 600;

/** Intitulé d'un plan. */
export const PLAN_TITLE_MIN_CHARS = 8;
export const PLAN_TITLE_MAX_CHARS = 80;

/** Note qui accompagne un plan : l'esprit de la chose, en une ou deux phrases. */
export const PLAN_NOTE_MAX_CHARS = 280;

/** Un plan se publie au plus tôt dans une heure. */
export const PLAN_MIN_LEAD_MINUTES = 60;

/** Nombre de personnes qu'un plan peut accueillir, en plus de son auteur. */
export const PLAN_CAPACITY_SOLO = 1 as const;
export const PLAN_CAPACITY_GROUP_MAX = 4 as const;

/**
 * Un plan disparaît du fil à son heure de rendez-vous. Personne n'a à le
 * retirer : le temps s'en charge, comme dans la vraie vie.
 */
export const PLAN_GRACE_MINUTES = 30;

/** Durée de vie du fil composé, en cache. */
export const FEED_TTL_SECONDS = 5 * 60;

/** TTL du cache d'identité. */
export const SESSION_CACHE_TTL_SECONDS = 15 * 60;

/** Âge minimum. Weave est une application de rencontre : elle est réservée aux majeurs. */
export const MIN_AGE = 18;

/** Rayon de recherche par défaut, en kilomètres. */
export const DEFAULT_RADIUS_KM = 25;
export const MAX_RADIUS_KM = 100;

/**
 * Genres, pour la fiche et pour les critères du fil.
 *
 * Un vocabulaire fixe, et c'est ce qui compte : le fil retient un plan quand
 * le genre de son auteur figure parmi ceux que le lecteur cherche, comparés
 * caractère par caractère. Deux orthographes d'une même chose — « femme » et
 * « Femme » — ne se rencontreraient jamais.
 *
 * Rien de tout cela n'existait : l'API acceptait n'importe quelle chaîne de
 * quarante caractères, et l'application ne proposait ni de renseigner son
 * genre ni de dire qui l'on cherche. La correspondance par genre était donc
 * écrite, et inatteignable.
 */
export const GENDERS = ["femme", "homme", "non_binaire", "autre"] as const;

/** Longueur de la phrase de présentation. Une phrase, pas une biographie. */
export const BIO_MAX_CHARS = 160;
export type Gender = (typeof GENDERS)[number];

export const GENDER_LABELS: Readonly<Record<Gender, string>> = {
  femme: "Femme",
  homme: "Homme",
  non_binaire: "Non binaire",
  autre: "Autre",
};

/** Catégories de plans. Volontairement peu nombreuses et concrètes. */
export const PLAN_CATEGORIES = [
  "sortie",
  "sport",
  "culture",
  "repas",
  "musique",
  "jeux",
  "balade",
  "benevolat",
] as const;
export type PlanCategory = (typeof PLAN_CATEGORIES)[number];

export const PLAN_CATEGORY_LABELS: Readonly<Record<PlanCategory, string>> = {
  sortie: "Sortie",
  sport: "Sport",
  culture: "Culture",
  repas: "Repas",
  musique: "Musique",
  jeux: "Jeux",
  balade: "Balade",
  benevolat: "Bénévolat",
};

/** Rétention des messages après clôture d'une conversation, en jours. */
export const MESSAGE_RETENTION_DAYS = 90;

/** Délai de purge d'un compte supprimé, en jours. */
export const ACCOUNT_PURGE_DAYS = 30;
