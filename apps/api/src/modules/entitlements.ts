/**
 * Droits d'usage : ce que le palier autorise, et ce que les crédits achetés à
 * l'unité permettent en plus.
 *
 * Le catalogue est partagé avec le site et l'application iOS
 * (`@weave/contracts/catalog`) : il n'existe qu'une seule définition des offres.
 */
import {
  TIERS,
  UNIT_PRODUCTS,
  UNIT_SKUS,
  type PlanTier,
  type TierEntitlements,
  type UnitSku,
} from "@weave/contracts";
import { entitlementRequired } from "../lib/errors.ts";
import { prisma } from "../lib/prisma.ts";

export function entitlementsFor(tier: PlanTier): TierEntitlements {
  return TIERS[tier].entitlements;
}

export type CreditMap = Record<UnitSku, number>;

const EMPTY_CREDITS = Object.fromEntries(UNIT_SKUS.map((sku) => [sku, 0])) as CreditMap;

export async function creditsFor(accountId: string): Promise<CreditMap> {
  const lignes = await prisma.creditBalance.findMany({
    where: { accountId },
    select: { sku: true, balance: true },
  });

  const credits: CreditMap = { ...EMPTY_CREDITS };
  for (const ligne of lignes) {
    if ((UNIT_SKUS as readonly string[]).includes(ligne.sku)) {
      credits[ligne.sku as UnitSku] = ligne.balance;
    }
  }
  return credits;
}

export async function grantCredits(
  accountId: string,
  sku: UnitSku,
  amount: number,
  resetsAt?: Date,
): Promise<number> {
  const ligne = await prisma.creditBalance.upsert({
    where: { accountId_sku: { accountId, sku } },
    create: { accountId, sku, balance: amount, resetsAt: resetsAt ?? null },
    update: { balance: { increment: amount }, ...(resetsAt ? { resetsAt } : {}) },
    select: { balance: true },
  });
  return ligne.balance;
}

/**
 * Consomme un crédit. La décrémentation est conditionnelle : le filtre exige
 * `balance >= 1`, ce qui empêche deux requêtes simultanées de dépenser le même
 * crédit.
 */
export async function spendCredit(accountId: string, sku: UnitSku): Promise<boolean> {
  const resultat = await prisma.creditBalance.updateMany({
    where: { accountId, sku, balance: { gte: 1 } },
    data: { balance: { decrement: 1 } },
  });
  return resultat.count === 1;
}

/**
 * Exige un crédit, en expliquant précisément comment l'obtenir : par un palier
 * ou à l'unité. Le client n'a rien à deviner ni à coder en dur.
 */
export async function requireCredit(accountId: string, sku: UnitSku): Promise<void> {
  if (await spendCredit(accountId, sku)) return;

  const produit = UNIT_PRODUCTS[sku];
  throw entitlementRequired(`« ${produit.name} » n'est pas compris dans votre offre.`, {
    sku,
    unitProductId: produit.storeKitId,
    unitPriceCents: produit.priceCents,
    includedIn: includedInTiers(sku),
  });
}

/** Paliers qui donnent accès à ce SKU sans achat. */
function includedInTiers(sku: UnitSku): PlanTier[] {
  const tiers = Object.keys(TIERS) as PlanTier[];
  switch (sku) {
    case "escale":
      return tiers.filter((t) => TIERS[t].entitlements.escalesPerMonth > 0);
    case "bilan":
      return tiers.filter((t) => TIERS[t].entitlements.bilan);
    case "tablee":
      return tiers.filter((t) => TIERS[t].entitlements.groupPlans);
    default:
      return [];
  }
}

/**
 * Dotation mensuelle du palier, appelée au renouvellement. Les crédits achetés
 * à l'unité ne sont jamais remis à zéro.
 */
export async function refillTierCredits(
  accountId: string,
  tier: PlanTier,
  resetsAt: Date,
): Promise<void> {
  const droits = TIERS[tier].entitlements;
  if (droits.escalesPerMonth <= 0) return;

  await prisma.creditBalance.upsert({
    where: { accountId_sku: { accountId, sku: "escale" } },
    create: { accountId, sku: "escale", balance: droits.escalesPerMonth, resetsAt },
    update: { balance: { increment: droits.escalesPerMonth }, resetsAt },
  });
}
