/**
 * Catalogue commercial de Weave.
 *
 * Freemium : un socle gratuit et QUATRE abonnements, dont chaque avantage est
 * également achetable à l'unité — parce qu'à vingt ans, on ne s'abonne pas à
 * tout, et qu'un produit qui l'exige se prive de la moitié de son public.
 *
 * RÈGLE DE CONCEPTION : aucun palier n'achète de visibilité. Payer ne fait
 * jamais remonter un plan devant celui de quelqu'un d'autre. Ce qui se vend,
 * c'est la finesse des critères, l'horizon de publication et les plans de
 * groupe — jamais une place dans la file.
 */

import { REQUESTS_PER_DAY_FLOOR } from "./invariants.ts";

export const PLAN_TIERS = ["depart", "viree", "escapade", "expedition", "grandtour"] as const;
export type PlanTier = (typeof PLAN_TIERS)[number];

export type FilterDepth = "base" | "etendus" | "precis";

export interface TierEntitlements {
  /** Demandes envoyables par jour. Bornée à tous les paliers, par conception. */
  readonly requestsPerDay: number;
  /** Jusqu'à combien de jours à l'avance un plan peut être publié. */
  readonly daysAhead: number;
  /** Finesse des critères du fil. */
  readonly filters: FilterDepth;
  /** Autorise les plans de groupe (jusqu'à quatre personnes). */
  readonly groupPlans: boolean;
  /** Nombre d'« Escale » incluses par mois. */
  readonly escalesPerMonth: number;
  /** Rapport « Bilan » sur ses propres plans. */
  readonly bilan: boolean;
  /** Vérification de profil accélérée et assistance prioritaire. */
  readonly prioritySupport: boolean;
}

export interface Tier {
  readonly tier: PlanTier;
  readonly name: string;
  readonly tagline: string;
  readonly monthlyPriceCents: number;
  /**
   * Identifiant StoreKit de l'abonnement mensuel. `null` pour le socle
   * gratuit, qui ne s'achète pas.
   *
   * Il n'y a pas d'abonnement annuel : un engagement de douze mois sur un
   * service qu'on peut vouloir quitter du jour au lendemain ne rend service
   * qu'à celui qui l'encaisse.
   */
  readonly storeKit: { readonly monthly: string | null };
  readonly entitlements: TierEntitlements;
  readonly highlights: readonly string[];
}

const BUNDLE = "com.weave.app";

export const TIERS: Readonly<Record<PlanTier, Tier>> = {
  depart: {
    tier: "depart",
    name: "Départ",
    tagline: "De quoi publier ses plans et demander à venir.",
    monthlyPriceCents: 0,
    storeKit: { monthly: null },
    entitlements: {
      requestsPerDay: REQUESTS_PER_DAY_FLOOR,
      daysAhead: 7,
      filters: "base",
      groupPlans: false,
      escalesPerMonth: 0,
      bilan: false,
      prioritySupport: false,
    },
    highlights: [
      "3 plans ouverts à la fois",
      "5 demandes par jour",
      "Conversations sans limite, une fois la demande acceptée",
    ],
  },
  viree: {
    tier: "viree",
    name: "Virée",
    tagline: "Pour ceux qui sortent souvent.",
    monthlyPriceCents: 499,
    storeKit: { monthly: `${BUNDLE}.sub.viree.monthly` },
    entitlements: {
      requestsPerDay: 12,
      daysAhead: 14,
      filters: "etendus",
      groupPlans: true,
      escalesPerMonth: 0,
      bilan: false,
      prioritySupport: false,
    },
    highlights: [
      "12 demandes par jour",
      "Plans de groupe, jusqu'à quatre",
      "Publication jusqu'à deux semaines à l'avance",
    ],
  },
  escapade: {
    tier: "escapade",
    name: "Escapade",
    tagline: "Des critères qui trient vraiment.",
    monthlyPriceCents: 899,
    storeKit: { monthly: `${BUNDLE}.sub.escapade.monthly` },
    entitlements: {
      requestsPerDay: 25,
      daysAhead: 30,
      filters: "precis",
      groupPlans: true,
      escalesPerMonth: 1,
      bilan: false,
      prioritySupport: false,
    },
    highlights: [
      "Critères précis : catégorie, jour, distance fine",
      "Publication jusqu'à un mois à l'avance",
      "1 Escale par mois, pour préparer un départ",
    ],
  },
  expedition: {
    tier: "expedition",
    name: "Expédition",
    tagline: "Organiser loin, et savoir ce qui marche.",
    monthlyPriceCents: 1499,
    storeKit: { monthly: `${BUNDLE}.sub.expedition.monthly` },
    entitlements: {
      requestsPerDay: 40,
      daysAhead: 60,
      filters: "precis",
      groupPlans: true,
      escalesPerMonth: 2,
      bilan: true,
      prioritySupport: false,
    },
    highlights: [
      "Publication jusqu'à deux mois à l'avance",
      "Bilan mensuel : quels plans attirent, et pourquoi",
      "2 Escales par mois",
    ],
  },
  grandtour: {
    tier: "grandtour",
    name: "Grand Tour",
    tagline: "Tout, sans y penser.",
    monthlyPriceCents: 2499,
    storeKit: { monthly: `${BUNDLE}.sub.grandtour.monthly` },
    entitlements: {
      requestsPerDay: 60,
      daysAhead: 90,
      filters: "precis",
      groupPlans: true,
      escalesPerMonth: 4,
      bilan: true,
      prioritySupport: true,
    },
    highlights: [
      "Publication jusqu'à trois mois à l'avance",
      "4 Escales par mois",
      "Vérification de profil accélérée et assistance prioritaire",
    ],
  },
};

/* ------------------------------------------------------------------ */
/* Achats à l'unité (consommables StoreKit)                            */
/* ------------------------------------------------------------------ */

/**
 * Le prix d'une unité, et pourquoi il est ce qu'il est.
 *
 * L'achat à l'unité existe parce qu'à vingt ans on ne s'abonne pas à tout. Il
 * ne doit pas pour autant revenir moins cher que l'abonnement : un catalogue
 * où l'on s'en sort mieux au détail vend des unités à des gens qui auraient
 * pris un abonnement, et laisse le revenu récurrent sur la table.
 *
 * LA RÈGLE, tenue par un test : la somme des cinq unités dépasse l'Expédition,
 * qui est le palier réunissant à peu près tout ce qu'elles accordent. Acheter
 * une fois chacune coûte donc plus cher qu'un mois d'abonnement qui les donne
 * toutes — et les redonne le mois suivant.
 *
 * Les conséquences, palier par palier :
 *
 * - deux « Renfort », « Horizon » ou « Tablée » dans le mois coûtent plus que
 *   la Virée, qui les donne sans compter ;
 * - une « Escale » achetée par-dessus la Virée coûte plus que l'Escapade, qui
 *   en comprend une ;
 * - deux « Escale » par-dessus l'Expédition coûtent plus que le Grand Tour,
 *   qui en comprend quatre. C'était l'inverse : 14,99 + 2 × 3,99 = 22,97,
 *   contre 24,99. On s'abonnait moins pour en avoir plus.
 *
 * Ce qu'elle ne cherche pas à empêcher : acheter UNE unité pour un besoin
 * ponctuel reste moins cher que de s'abonner. C'est le propre de l'achat à
 * l'unité, et le lui retirer reviendrait à ne plus en vendre.
 */
export const UNIT_SKUS = ["renfort", "horizon", "tablee", "escale", "bilan"] as const;
export type UnitSku = (typeof UNIT_SKUS)[number];

export interface UnitProduct {
  readonly sku: UnitSku;
  readonly name: string;
  readonly description: string;
  readonly priceCents: number;
  readonly storeKitId: string;
  readonly grants: number;
}

export const UNIT_PRODUCTS: Readonly<Record<UnitSku, UnitProduct>> = {
  renfort: {
    sku: "renfort",
    name: "Renfort",
    description: "Cinq demandes de plus aujourd'hui.",
    priceCents: 299,
    storeKitId: `${BUNDLE}.unit.renfort`,
    grants: 1,
  },
  horizon: {
    sku: "horizon",
    name: "Horizon",
    description: "Publier un plan jusqu'à soixante jours à l'avance, une fois.",
    priceCents: 299,
    storeKitId: `${BUNDLE}.unit.horizon`,
    grants: 1,
  },
  tablee: {
    sku: "tablee",
    name: "Tablée",
    description: "Un plan de groupe, jusqu'à quatre personnes, une fois.",
    priceCents: 299,
    storeKitId: `${BUNDLE}.unit.tablee`,
    grants: 1,
  },
  escale: {
    sku: "escale",
    name: "Escale",
    description: "Publier depuis une autre ville pendant sept jours.",
    priceCents: 599,
    storeKitId: `${BUNDLE}.unit.escale`,
    grants: 1,
  },
  bilan: {
    sku: "bilan",
    name: "Bilan",
    description: "Un retour ponctuel sur vos plans : ce qui attire, ce qui tombe à plat.",
    priceCents: 499,
    storeKitId: `${BUNDLE}.unit.bilan`,
    grants: 1,
  },
};

export function tierFromStoreKitId(productId: string): Tier | null {
  for (const tier of PLAN_TIERS) {
    const offre = TIERS[tier];
    if (offre.storeKit.monthly === productId) return offre;
  }
  return null;
}

export function unitFromStoreKitId(productId: string): UnitProduct | null {
  for (const sku of UNIT_SKUS) {
    if (UNIT_PRODUCTS[sku].storeKitId === productId) return UNIT_PRODUCTS[sku];
  }
  return null;
}

export function formatPrice(cents: number, locale = "fr-FR", currency = "EUR"): string {
  return new Intl.NumberFormat(locale, { style: "currency", currency }).format(cents / 100);
}

/* ------------------------------------------------------------------ */
/* Traductions du catalogue                                            */
/* ------------------------------------------------------------------ */

/**
 * Les *noms* des offres et des achats à l'unité ne se traduisent pas :
 * « Escapade », « Renfort », « Bilan » sont des noms de produits, déclarés
 * tels quels dans App Store Connect. Ce qui se traduit, c'est la phrase qui
 * les accompagne et ce qu'ils donnent.
 */
export type LangueCatalogue = "fr" | "en" | "es";

export interface TierCopy {
  readonly tagline: string;
  readonly highlights: readonly string[];
}

export const TIER_COPY_PAR_LANGUE: Readonly<
  Record<LangueCatalogue, Readonly<Record<PlanTier, TierCopy>>>
> = {
  fr: {
    depart: { tagline: TIERS.depart.tagline, highlights: TIERS.depart.highlights },
    viree: { tagline: TIERS.viree.tagline, highlights: TIERS.viree.highlights },
    escapade: { tagline: TIERS.escapade.tagline, highlights: TIERS.escapade.highlights },
    expedition: { tagline: TIERS.expedition.tagline, highlights: TIERS.expedition.highlights },
    grandtour: { tagline: TIERS.grandtour.tagline, highlights: TIERS.grandtour.highlights },
  },
  en: {
    depart: {
      tagline: "Enough to post your plans and ask to come along.",
      highlights: [
        "3 plans open at a time",
        "5 requests a day",
        "Conversations with no limit, once a request is accepted",
      ],
    },
    viree: {
      tagline: "For people who go out often.",
      highlights: [
        "12 requests a day",
        "Group plans, up to four people",
        "Post up to two weeks ahead",
      ],
    },
    escapade: {
      tagline: "Filters that actually narrow things down.",
      highlights: [
        "Precise filters: category, day, fine-grained distance",
        "Post up to a month ahead",
        "1 Escale a month, to plan a trip",
      ],
    },
    expedition: {
      tagline: "Organise far ahead, and learn what works.",
      highlights: [
        "Post up to two months ahead",
        "Monthly Bilan: which plans draw people, and why",
        "2 Escales a month",
      ],
    },
    grandtour: {
      tagline: "Everything, without thinking about it.",
      highlights: [
        "Post up to three months ahead",
        "4 Escales a month",
        "Faster profile verification and priority support",
      ],
    },
  },
  es: {
    depart: {
      tagline: "Lo justo para publicar tus planes y pedir venir.",
      highlights: [
        "3 planes abiertos a la vez",
        "5 peticiones al día",
        "Conversaciones sin límite, una vez aceptada la petición",
      ],
    },
    viree: {
      tagline: "Para quien sale a menudo.",
      highlights: [
        "12 peticiones al día",
        "Planes de grupo, hasta cuatro personas",
        "Publicar hasta dos semanas antes",
      ],
    },
    escapade: {
      tagline: "Criterios que filtran de verdad.",
      highlights: [
        "Criterios precisos: categoría, día, distancia afinada",
        "Publicar hasta un mes antes",
        "1 Escale al mes, para preparar una salida",
      ],
    },
    expedition: {
      tagline: "Organizar con tiempo, y saber qué funciona.",
      highlights: [
        "Publicar hasta dos meses antes",
        "Bilan mensual: qué planes atraen, y por qué",
        "2 Escales al mes",
      ],
    },
    grandtour: {
      tagline: "Todo, sin pensarlo.",
      highlights: [
        "Publicar hasta tres meses antes",
        "4 Escales al mes",
        "Verificación de perfil acelerada y asistencia prioritaria",
      ],
    },
  },
};

export const UNIT_DESCRIPTIONS_PAR_LANGUE: Readonly<
  Record<LangueCatalogue, Readonly<Record<UnitSku, string>>>
> = {
  fr: {
    renfort: UNIT_PRODUCTS.renfort.description,
    horizon: UNIT_PRODUCTS.horizon.description,
    tablee: UNIT_PRODUCTS.tablee.description,
    escale: UNIT_PRODUCTS.escale.description,
    bilan: UNIT_PRODUCTS.bilan.description,
  },
  en: {
    renfort: "Five more requests today.",
    horizon: "Post a plan up to sixty days ahead, once.",
    tablee: "One group plan, up to four people, once.",
    escale: "Post from another town for seven days.",
    bilan: "A one-off look at your plans: what draws people, what falls flat.",
  },
  es: {
    renfort: "Cinco peticiones más hoy.",
    horizon: "Publicar un plan hasta sesenta días antes, una vez.",
    tablee: "Un plan de grupo, hasta cuatro personas, una vez.",
    escale: "Publicar desde otra ciudad durante siete días.",
    bilan: "Un repaso puntual de tus planes: qué atrae, qué no cuaja.",
  },
};

/**
 * Les prix restent en euros dans les trois langues : la facturation passe par
 * Apple, en euros, quelle que soit la langue d'affichage. Seule la façon
 * d'écrire le nombre change.
 */
export const LOCALE_DE_PRIX: Readonly<Record<LangueCatalogue, string>> = {
  fr: "fr-FR",
  en: "en-IE",
  es: "es-ES",
};
