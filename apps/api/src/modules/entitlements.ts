/**
 * Droits d'usage : ce que le palier d'abonnement autorise, et ce que les
 * crédits achetés à l'unité permettent en plus.
 *
 * Le catalogue est partagé avec le site et l'application iOS
 * (`@weave/contracts/catalog`) : il n'y a qu'une seule définition des offres.
 */
import {
  PLANS,
  UNIT_PRODUCTS,
  UNIT_SKUS,
  type PlanEntitlements,
  type PlanTier,
  type UnitSku,
} from "@weave/contracts";
import { entitlementRequired } from "../lib/errors.ts";
import { prisma } from "../lib/prisma.ts";

export function entitlementsFor(tier: PlanTier): PlanEntitlements {
  return PLANS[tier].entitlements;
}

export type CreditMap = Record<UnitSku, number>;

const EMPTY_CREDITS = Object.fromEntries(UNIT_SKUS.map((sku) => [sku, 0])) as CreditMap;

/** Solde de crédits par SKU, tous SKU présents (0 par défaut). */
export async function creditsFor(accountId: string): Promise<CreditMap> {
  const rows = await prisma.creditBalance.findMany({
    where: { accountId },
    select: { sku: true, balance: true },
  });

  const credits: CreditMap = { ...EMPTY_CREDITS };
  for (const row of rows) {
    if ((UNIT_SKUS as readonly string[]).includes(row.sku)) {
      credits[row.sku as UnitSku] = row.balance;
    }
  }
  return credits;
}

/** Ajoute des crédits (achat à l'unité, ou dotation mensuelle d'un abonnement). */
export async function grantCredits(
  accountId: string,
  sku: UnitSku,
  amount: number,
  resetsAt?: Date,
): Promise<number> {
  const row = await prisma.creditBalance.upsert({
    where: { accountId_sku: { accountId, sku } },
    create: { accountId, sku, balance: amount, resetsAt: resetsAt ?? null },
    update: { balance: { increment: amount }, ...(resetsAt ? { resetsAt } : {}) },
    select: { balance: true },
  });
  return row.balance;
}

/**
 * Consomme un crédit. La décrémentation est conditionnelle : `updateMany` avec
 * `balance >= 1` dans le filtre, ce qui empêche deux requêtes simultanées de
 * dépenser le même crédit.
 */
export async function spendCredit(accountId: string, sku: UnitSku): Promise<boolean> {
  const result = await prisma.creditBalance.updateMany({
    where: { accountId, sku, balance: { gte: 1 } },
    data: { balance: { decrement: 1 } },
  });
  return result.count === 1;
}

/**
 * Exige un crédit pour l'action demandée, en expliquant précisément comment
 * l'obtenir : par un palier d'abonnement, ou à l'unité.
 */
export async function requireCredit(accountId: string, sku: UnitSku): Promise<void> {
  if (await spendCredit(accountId, sku)) return;

  const product = UNIT_PRODUCTS[sku];
  throw entitlementRequired(
    `« ${product.name} » n'est pas disponible sur votre offre actuelle.`,
    {
      sku,
      unitProductId: product.storeKitId,
      unitPriceCents: product.priceCents,
      includedIn: includedInPlans(sku),
    },
  );
}

/** Paliers qui incluent au moins un exemplaire mensuel de ce SKU. */
function includedInPlans(sku: UnitSku): PlanTier[] {
  const field: Partial<Record<UnitSku, keyof PlanEntitlements>> = {
    echo: "echoesPerMonth",
    prolonge: "extendsPerMonth",
    escale: "escalesPerMonth",
  };
  const key = field[sku];
  if (key === undefined) return [];
  return (Object.keys(PLANS) as PlanTier[]).filter((tier) => {
    const value = PLANS[tier].entitlements[key];
    return typeof value === "number" && value > 0;
  });
}

/**
 * Dotation mensuelle du palier : appelée au renouvellement d'un abonnement.
 * Les crédits inclus sont remis à leur valeur nominale ; les crédits achetés à
 * l'unité, eux, ne sont jamais remis à zéro.
 */
export async function refillPlanCredits(accountId: string, tier: PlanTier, resetsAt: Date): Promise<void> {
  const e = PLANS[tier].entitlements;
  const included: Partial<Record<UnitSku, number>> = {
    echo: e.echoesPerMonth,
    prolonge: e.extendsPerMonth,
    escale: e.escalesPerMonth,
  };

  for (const [sku, amount] of Object.entries(included) as [UnitSku, number][]) {
    if (amount <= 0) continue;
    await prisma.creditBalance.upsert({
      where: { accountId_sku: { accountId, sku } },
      create: { accountId, sku, balance: amount, resetsAt },
      update: { balance: { increment: amount }, resetsAt },
    });
  }
}
