/**
 * Catalogue commercial de Weave.
 *
 * Modèle freemium : un socle gratuit (« Fil ») + QUATRE abonnements, dont
 * chaque avantage est également achetable À L'UNITÉ pour les personnes qui ne
 * veulent pas s'abonner.
 *
 * Règle de conception : aucun palier ne modifie l'invariant des trois fils
 * (voir `invariants.ts`). Ce qui se vend, c'est la finesse du tissage et la
 * vitesse de remplacement d'un fil dénoué — jamais le volume de profils.
 */

import { MAX_ACTIVE_THREADS, type MaxActiveThreads } from "./invariants.ts";

export const PLAN_TIERS = ["fil", "trame", "chaine", "navette", "metier"] as const;
export type PlanTier = (typeof PLAN_TIERS)[number];

export type CriteriaDepth = "base" | "etendue" | "precise";

export interface PlanEntitlements {
  /**
   * Plafond de fils actifs. Constant sur tous les paliers, par conception.
   * Le type est celui de l'invariant : un palier ne peut pas annoncer un autre
   * nombre, même par inadvertance.
   */
  readonly activeThreads: MaxActiveThreads;
  /** Délai avant qu'un fil dénoué soit remplacé, en minutes. */
  readonly refillDelayMinutes: number;
  /** Profondeur des critères de composition. */
  readonly criteria: CriteriaDepth;
  /** Autorise l'envoi de fragments vocaux. */
  readonly voiceFragments: boolean;
  /** Nombre d'« Écho » (rappel d'un fil dénoué) inclus par mois. */
  readonly echoesPerMonth: number;
  /** Nombre de « Prolonge » (+24 h sur un fil) inclus par mois. */
  readonly extendsPerMonth: number;
  /** Changements d'heure de tissage inclus par mois. */
  readonly weavingHourChangesPerMonth: number;
  /** Indique si l'auteur voit que son fragment a été lu. */
  readonly fragmentReadState: boolean;
  /** Nombre d'« Escale » (ville temporaire, 7 jours) incluses par mois. */
  readonly escalesPerMonth: number;
  /** Rapport « Atelier » : résonance de ses propres fragments. */
  readonly atelierReport: boolean;
  /** Assistance prioritaire et vérification de profil accélérée. */
  readonly prioritySupport: boolean;
}

export interface Plan {
  readonly tier: PlanTier;
  readonly name: string;
  readonly tagline: string;
  /** Prix mensuel en centimes d'euro (0 pour le socle gratuit). */
  readonly monthlyPriceCents: number;
  /** Prix annuel en centimes d'euro (null si non proposé). */
  readonly yearlyPriceCents: number | null;
  /** Identifiants produits StoreKit (App Store Connect). */
  readonly storeKit: {
    readonly monthly: string | null;
    readonly yearly: string | null;
  };
  readonly entitlements: PlanEntitlements;
  readonly highlights: readonly string[];
}

const BUNDLE = "com.weave.app";

export const PLANS: Readonly<Record<PlanTier, Plan>> = {
  fil: {
    tier: "fil",
    name: "Fil",
    tagline: "Trois fils par jour. Rien de plus, rien de moins.",
    monthlyPriceCents: 0,
    yearlyPriceCents: null,
    storeKit: { monthly: null, yearly: null },
    entitlements: {
      activeThreads: MAX_ACTIVE_THREADS,
      refillDelayMinutes: 24 * 60,
      criteria: "base",
      voiceFragments: false,
      echoesPerMonth: 0,
      extendsPerMonth: 0,
      weavingHourChangesPerMonth: 1,
      fragmentReadState: false,
      escalesPerMonth: 0,
      atelierReport: false,
      prioritySupport: false,
    },
    highlights: [
      "3 fils actifs, renouvelés à votre heure de tissage",
      "Conversations complètes, sans limite de messages",
      "Live Activity et application Apple Watch incluses",
    ],
  },
  trame: {
    tier: "trame",
    name: "Trame",
    tagline: "Le tissage reprend plus vite.",
    monthlyPriceCents: 699,
    yearlyPriceCents: 5990,
    storeKit: { monthly: `${BUNDLE}.sub.trame.monthly`, yearly: `${BUNDLE}.sub.trame.yearly` },
    entitlements: {
      activeThreads: MAX_ACTIVE_THREADS,
      refillDelayMinutes: 6 * 60,
      criteria: "etendue",
      voiceFragments: true,
      echoesPerMonth: 1,
      extendsPerMonth: 2,
      weavingHourChangesPerMonth: 4,
      fragmentReadState: false,
      escalesPerMonth: 0,
      atelierReport: false,
      prioritySupport: false,
    },
    highlights: [
      "Un fil dénoué est remplacé sous 6 h",
      "Fragments vocaux de 8 secondes",
      "1 Écho et 2 Prolonge inclus chaque mois",
    ],
  },
  chaine: {
    tier: "chaine",
    name: "Chaîne",
    tagline: "Des critères qui tiennent la trame.",
    monthlyPriceCents: 1299,
    yearlyPriceCents: 10990,
    storeKit: { monthly: `${BUNDLE}.sub.chaine.monthly`, yearly: `${BUNDLE}.sub.chaine.yearly` },
    entitlements: {
      activeThreads: MAX_ACTIVE_THREADS,
      refillDelayMinutes: 3 * 60,
      criteria: "precise",
      voiceFragments: true,
      echoesPerMonth: 3,
      extendsPerMonth: 5,
      weavingHourChangesPerMonth: 12,
      fragmentReadState: true,
      escalesPerMonth: 1,
      atelierReport: false,
      prioritySupport: false,
    },
    highlights: [
      "Remplacement sous 3 h",
      "Critères précis : intentions, rythme de vie, distance fine",
      "1 Escale par mois et accusé de lecture des fragments",
    ],
  },
  navette: {
    tier: "navette",
    name: "Navette",
    tagline: "Le fil ne reste jamais vide.",
    monthlyPriceCents: 1999,
    yearlyPriceCents: 16990,
    storeKit: { monthly: `${BUNDLE}.sub.navette.monthly`, yearly: `${BUNDLE}.sub.navette.yearly` },
    entitlements: {
      activeThreads: MAX_ACTIVE_THREADS,
      refillDelayMinutes: 60,
      criteria: "precise",
      voiceFragments: true,
      echoesPerMonth: 6,
      extendsPerMonth: 10,
      weavingHourChangesPerMonth: 31,
      fragmentReadState: true,
      escalesPerMonth: 2,
      atelierReport: true,
      prioritySupport: false,
    },
    highlights: [
      "Remplacement sous 1 h",
      "Rapport Atelier mensuel sur la résonance de vos fragments",
      "2 Escales par mois",
    ],
  },
  metier: {
    tier: "metier",
    name: "Métier",
    tagline: "L'atelier complet.",
    monthlyPriceCents: 3499,
    yearlyPriceCents: 29990,
    storeKit: { monthly: `${BUNDLE}.sub.metier.monthly`, yearly: `${BUNDLE}.sub.metier.yearly` },
    entitlements: {
      activeThreads: MAX_ACTIVE_THREADS,
      refillDelayMinutes: 15,
      criteria: "precise",
      voiceFragments: true,
      echoesPerMonth: 15,
      extendsPerMonth: 30,
      weavingHourChangesPerMonth: 31,
      fragmentReadState: true,
      escalesPerMonth: 4,
      atelierReport: true,
      prioritySupport: true,
    },
    highlights: [
      "Remplacement sous 15 min",
      "Vérification de profil accélérée et assistance prioritaire",
      "4 Escales par mois, Écho et Prolonge en abondance",
    ],
  },
};

/* ------------------------------------------------------------------ */
/* Achats à l'unité (consommables StoreKit)                            */
/* ------------------------------------------------------------------ */

export const UNIT_SKUS = ["echo", "prolonge", "relais", "motif", "escale", "atelier"] as const;
export type UnitSku = (typeof UNIT_SKUS)[number];

export interface UnitProduct {
  readonly sku: UnitSku;
  readonly name: string;
  readonly description: string;
  readonly priceCents: number;
  readonly storeKitId: string;
  /** Nombre d'exemplaires accordés par achat. */
  readonly grants: number;
}

export const UNIT_PRODUCTS: Readonly<Record<UnitSku, UnitProduct>> = {
  echo: {
    sku: "echo",
    name: "Écho",
    description: "Rappeler une seule fois un fil que vous avez laissé se dénouer.",
    priceCents: 249,
    storeKitId: `${BUNDLE}.unit.echo`,
    grants: 1,
  },
  prolonge: {
    sku: "prolonge",
    name: "Prolonge",
    description: "Ajouter 24 h de vie à un fil en cours, une seule fois par fil.",
    priceCents: 149,
    storeKitId: `${BUNDLE}.unit.prolonge`,
    grants: 1,
  },
  relais: {
    sku: "relais",
    name: "Relais",
    description: "Remplacer immédiatement un fil dénoué, sans attendre le délai de votre palier.",
    priceCents: 199,
    storeKitId: `${BUNDLE}.unit.relais`,
    grants: 1,
  },
  motif: {
    sku: "motif",
    name: "Motif",
    description: "Retisser les cinq mots-clés qui vous décrivent à partir de nouvelles réponses.",
    priceCents: 99,
    storeKitId: `${BUNDLE}.unit.motif`,
    grants: 1,
  },
  escale: {
    sku: "escale",
    name: "Escale",
    description: "Tisser depuis une autre ville pendant sept jours.",
    priceCents: 499,
    storeKitId: `${BUNDLE}.unit.escale`,
    grants: 1,
  },
  atelier: {
    sku: "atelier",
    name: "Atelier",
    description: "Un rapport ponctuel sur la résonance de vos fragments.",
    priceCents: 349,
    storeKitId: `${BUNDLE}.unit.atelier`,
    grants: 1,
  },
};

/** Retourne le palier correspondant à un identifiant StoreKit d'abonnement. */
export function planFromStoreKitId(productId: string): Plan | null {
  for (const tier of PLAN_TIERS) {
    const plan = PLANS[tier];
    if (plan.storeKit.monthly === productId || plan.storeKit.yearly === productId) return plan;
  }
  return null;
}

/** Retourne le produit à l'unité correspondant à un identifiant StoreKit. */
export function unitFromStoreKitId(productId: string): UnitProduct | null {
  for (const sku of UNIT_SKUS) {
    if (UNIT_PRODUCTS[sku].storeKitId === productId) return UNIT_PRODUCTS[sku];
  }
  return null;
}

/** Formate un prix en centimes vers une chaîne lisible (fr-FR). */
export function formatPrice(cents: number, locale = "fr-FR", currency = "EUR"): string {
  return new Intl.NumberFormat(locale, { style: "currency", currency }).format(cents / 100);
}
