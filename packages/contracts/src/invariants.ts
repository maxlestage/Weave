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
 * Les rayons disponibles sans « critères précis ».
 *
 * Le catalogue vend « Critères précis : catégorie, jour, DISTANCE FINE » à
 * partir de l'Escapade. La catégorie et le jour se limitaient bien par palier ;
 * la distance, elle, se réglait au kilomètre près à tous les paliers — y
 * compris le gratuit. « Distance fine » était donc vendue trois fois sans rien
 * désigner qui n'existât déjà partout.
 *
 * Aux paliers sans critères précis, le rayon se rabat sur l'un de ces quatre
 * crans. Le réglage choisi n'est pas écrasé pour autant : il est conservé tel
 * quel, et redevient exact dès que l'offre le permet — un abonnement qui
 * s'interrompt ne doit pas faire perdre ce qu'on avait réglé.
 */
export const RADIUS_STEPS_KM = [10, 25, 50, 100] as const;

/**
 * Le rayon effectivement appliqué, selon que le palier donne la distance fine.
 *
 * Arrondit au cran le plus proche, et jamais en dessous du plus petit : un
 * rayon rabattu vers le bas viderait le fil de quelqu'un qui n'a rien demandé.
 */
export function effectiveRadiusKm(km: number, fineDistance: boolean): number {
  if (fineDistance) return km;
  return RADIUS_STEPS_KM.reduce((retenu, cran) =>
    Math.abs(cran - km) < Math.abs(retenu - km) ? cran : retenu,
  );
}

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

/**
 * Les mêmes libellés, dans les langues du site.
 *
 * Les VALEURS — `femme`, `sport` — ne se traduisent pas : ce sont elles qui
 * circulent entre l'API, le site et l'application, et un vocabulaire traduit
 * ferait que deux versions d'une même chose ne se rencontreraient jamais.
 * Seuls les libellés affichés changent de langue.
 */
export const GENDER_LABELS_PAR_LANGUE: Readonly<
  Record<"fr" | "en" | "es", Readonly<Record<Gender, string>>>
> = {
  fr: GENDER_LABELS,
  en: { femme: "Woman", homme: "Man", non_binaire: "Non-binary", autre: "Other" },
  es: { femme: "Mujer", homme: "Hombre", non_binaire: "No binario", autre: "Otro" },
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

export const PLAN_CATEGORY_LABELS_PAR_LANGUE: Readonly<
  Record<"fr" | "en" | "es", Readonly<Record<PlanCategory, string>>>
> = {
  fr: PLAN_CATEGORY_LABELS,
  en: {
    sortie: "Going out",
    sport: "Sport",
    culture: "Culture",
    repas: "Food",
    musique: "Music",
    jeux: "Games",
    balade: "Walk",
    benevolat: "Volunteering",
  },
  es: {
    sortie: "Salida",
    sport: "Deporte",
    culture: "Cultura",
    repas: "Comida",
    musique: "Música",
    jeux: "Juegos",
    balade: "Paseo",
    benevolat: "Voluntariado",
  },
};

/** Rétention des messages après clôture d'une conversation, en jours. */
/**
 * Longueur maximale d'un message de conversation.
 *
 * Elle n'était écrite qu'au serveur : ni le contrat, ni l'application ne la
 * connaissaient. Le champ de saisie laissait donc écrire sans fin, et le
 * serveur refusait à l'envoi — la personne perdait ce qu'elle venait
 * d'écrire, pour une règle que rien ne lui avait annoncée.
 */
export const CONVERSATION_MAX_CHARS = 2000;

export const MESSAGE_RETENTION_DAYS = 90;

/** Délai de purge d'un compte supprimé, en jours. */
export const ACCOUNT_PURGE_DAYS = 30;

/* ------------------------------------------------------------------ */
/* Consentements                                                       */
/* ------------------------------------------------------------------ */

/**
 * Les objets sur lesquels un consentement distinct est demandé.
 *
 * Un seul aujourd'hui, et c'est celui qui compte : les personnes que l'on
 * cherche, rapprochées de son propre genre, peuvent révéler l'orientation
 * sexuelle. Le règlement européen range cette information parmi les catégories
 * particulières de l'article 9 — elle ne peut être traitée que sur un
 * consentement **explicite et distinct**.
 *
 * La politique de confidentialité le promet depuis le début. La table
 * `consent_records` existait pour le consigner, et **rien ne l'écrivait** :
 * le critère de genre s'enregistrait sans qu'aucun consentement n'ait jamais
 * été demandé ni conservé.
 */
export const CONSENT_KINDS = ["donnees_sensibles"] as const;
export type ConsentKind = (typeof CONSENT_KINDS)[number];

/**
 * La version des textes en vigueur, au format ISO.
 *
 * Un consentement porte la version du texte accepté. La politique promet qu'en
 * cas de changement substantiel « un nouveau consentement vous est demandé » :
 * un consentement donné sur une version antérieure **cesse donc de valoir**,
 * et le traitement s'arrête jusqu'à ce qu'il soit redonné.
 *
 * Écrite à la main, comme la date affichée qu'elle gouverne : une version
 * calculée à la construction changerait à chaque déploiement et laisserait
 * croire à une révision qui n'a pas eu lieu.
 */
export const POLICY_VERSION = "2026-09-12";

/** Cette version, telle qu'elle s'affiche au bas des pages juridiques. */
export const POLICY_UPDATED_LABEL = "12 septembre 2026";
