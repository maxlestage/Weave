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
    priceCents: 149,
    storeKitId: `${BUNDLE}.unit.renfort`,
    grants: 1,
  },
  horizon: {
    sku: "horizon",
    name: "Horizon",
    description: "Publier un plan jusqu'à soixante jours à l'avance, une fois.",
    priceCents: 99,
    storeKitId: `${BUNDLE}.unit.horizon`,
    grants: 1,
  },
  tablee: {
    sku: "tablee",
    name: "Tablée",
    description: "Un plan de groupe, jusqu'à quatre personnes, une fois.",
    priceCents: 149,
    storeKitId: `${BUNDLE}.unit.tablee`,
    grants: 1,
  },
  escale: {
    sku: "escale",
    name: "Escale",
    description: "Publier depuis une autre ville pendant sept jours.",
    priceCents: 399,
    storeKitId: `${BUNDLE}.unit.escale`,
    grants: 1,
  },
  bilan: {
    sku: "bilan",
    name: "Bilan",
    description: "Un retour ponctuel sur vos plans : ce qui attire, ce qui tombe à plat.",
    priceCents: 299,
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
