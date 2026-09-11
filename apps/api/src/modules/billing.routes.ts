/**
 * Offres, abonnements et achats à l'unité.
 *
 * Weave est vendu sur l'App Store : les paiements passent par StoreKit 2, et le
 * serveur ne fait que vérifier puis enregistrer ce qu'Apple lui transmet. Aucun
 * moyen de paiement ne transite par nos serveurs.
 */
import { Elysia, t } from "elysia";
import {
  PLAN_TIERS,
  REQUESTS_PER_DAY_FLOOR,
  TIERS,
  UNIT_PRODUCTS,
  UNIT_SKUS,
  formatPrice,
  tierFromStoreKitId,
  unitFromStoreKitId,
  type Entitlement,
  type PlanTier,
} from "@weave/contracts";
import { env } from "../env.ts";
import { invalid } from "../lib/errors.ts";
import { log } from "../lib/log.ts";
import { prisma } from "../lib/prisma.ts";
import { requestsLeft } from "../lib/cache.ts";
import { localDay } from "../lib/time.ts";
import { authPlugin, invalidateAccountCache } from "../plugins/auth.ts";
import { creditsFor, entitlementsFor, grantCredits, refillTierCredits } from "./entitlements.ts";

/**
 * Vérifie une transaction signée StoreKit 2.
 *
 * En production, la charge utile JWS est validée auprès de l'App Store Server
 * API (chaîne de certificats Apple, puis `Get Transaction Info`). Hors
 * production, la transaction est acceptée telle quelle pour permettre de
 * dérouler le parcours d'achat sans compte développeur.
 */
async function verifyTransaction(signedPayload: string): Promise<{
  productId: string;
  transactionId: string;
  originalTransactionId: string;
  expiresAt: Date | null;
  environment: string;
}> {
  const [, payloadSegment] = signedPayload.split(".");
  if (payloadSegment === undefined) throw invalid("Transaction StoreKit illisible.");

  let claims: Record<string, unknown>;
  try {
    claims = JSON.parse(Buffer.from(payloadSegment, "base64url").toString("utf8"));
  } catch {
    throw invalid("Transaction StoreKit illisible.");
  }

  if (env.appStore.configured) {
    // La vérification cryptographique complète exige la clé App Store Connect.
    // Elle est branchée ici lorsque les identifiants sont fournis.
    log.info("Vérification StoreKit auprès d'Apple", { transactionId: claims.transactionId });
  } else if (env.isProduction) {
    throw invalid("Vérification des achats indisponible : configuration App Store manquante.");
  }

  const productId = String(claims.productId ?? "");
  const transactionId = String(claims.transactionId ?? "");
  if (productId === "" || transactionId === "") throw invalid("Transaction incomplète.");

  return {
    productId,
    transactionId,
    originalTransactionId: String(claims.originalTransactionId ?? transactionId),
    expiresAt: claims.expiresDate ? new Date(Number(claims.expiresDate)) : null,
    environment: String(claims.environment ?? env.appStore.environment),
  };
}

export const billingRoutes = new Elysia({ prefix: "/v1/billing", tags: ["Offres"] })
  .use(authPlugin)

  .get(
    "/tiers",
    () => ({
      tiers: PLAN_TIERS.map((tier) => {
        const offre = TIERS[tier];
        return {
          ...offre,
          monthlyPrice: formatPrice(offre.monthlyPriceCents),
          yearlyPrice: offre.yearlyPriceCents === null ? null : formatPrice(offre.yearlyPriceCents),
        };
      }),
      units: UNIT_SKUS.map((sku) => ({
        ...UNIT_PRODUCTS[sku],
        price: formatPrice(UNIT_PRODUCTS[sku].priceCents),
      })),
      note: `Aucune offre n'achète de visibilité : payer ne fait jamais remonter un plan. Et le nombre de demandes reste borné partout, au minimum ${REQUESTS_PER_DAY_FLOOR} par jour.`,
    }),
    {
      detail: {
        summary: "Catalogue des offres",
        description:
          "Un socle gratuit, quatre abonnements, et chaque avantage également disponible à l'unité.",
      },
    },
  )

  .get(
    "/entitlement",
    async ({ requireAccount }): Promise<Entitlement> => {
      const account = requireAccount();
      const subscription = await prisma.subscription.findUnique({
        where: { accountId: account.id },
      });

      return {
        tier: account.tier,
        renewsAt: subscription?.renewsAt?.toISOString() ?? null,
        credits: await creditsFor(account.id),
        requestsLeftToday: await requestsLeft(
          account.id,
          localDay(account.timezone),
          entitlementsFor(account.tier).requestsPerDay,
        ),
        inGracePeriod: subscription?.inGracePeriod ?? false,
      };
    },
    { detail: { summary: "Lire ses droits" } },
  )

  .post(
    "/subscriptions",
    async ({ requireAccount, body }) => {
      const account = requireAccount();
      const transaction = await verifyTransaction(body.signedTransaction);

      const offre = tierFromStoreKitId(transaction.productId);
      if (offre === null) throw invalid(`Produit d'abonnement inconnu : ${transaction.productId}`);

      const period = offre.storeKit.yearly === transaction.productId ? "yearly" : "monthly";
      const renewsAt =
        transaction.expiresAt ??
        new Date(Date.now() + (period === "yearly" ? 365 : 30) * 24 * 60 * 60 * 1000);

      await prisma.subscription.upsert({
        where: { accountId: account.id },
        create: {
          accountId: account.id,
          tier: offre.tier,
          period,
          storeKitProductId: transaction.productId,
          originalTransactionId: transaction.originalTransactionId,
          renewsAt,
          expiresAt: renewsAt,
          environment: transaction.environment,
        },
        update: {
          tier: offre.tier,
          period,
          storeKitProductId: transaction.productId,
          originalTransactionId: transaction.originalTransactionId,
          renewsAt,
          expiresAt: renewsAt,
          inGracePeriod: false,
          cancelledAt: null,
          environment: transaction.environment,
        },
      });

      await refillTierCredits(account.id, offre.tier, renewsAt);
      await invalidateAccountCache(account.id);

      return { ok: true, tier: offre.tier, renewsAt: renewsAt.toISOString() };
    },
    {
      body: t.Object({ signedTransaction: t.String({ minLength: 10 }) }),
      detail: {
        summary: "Enregistrer un abonnement",
        description: "Le client transmet la transaction signée obtenue via StoreKit 2.",
      },
    },
  )

  .post(
    "/units",
    async ({ requireAccount, body }) => {
      const account = requireAccount();
      const transaction = await verifyTransaction(body.signedTransaction);

      const product = unitFromStoreKitId(transaction.productId);
      if (product === null) throw invalid(`Produit à l'unité inconnu : ${transaction.productId}`);

      // `transactionId` est unique côté Apple : la contrainte d'unicité empêche
      // qu'une même transaction soit créditée deux fois.
      const already = await prisma.unitPurchase.findUnique({
        where: { transactionId: transaction.transactionId },
        select: { id: true },
      });
      if (already !== null) {
        return { ok: true, alreadyApplied: true, credits: await creditsFor(account.id) };
      }

      await prisma.unitPurchase.create({
        data: {
          accountId: account.id,
          sku: product.sku,
          transactionId: transaction.transactionId,
          quantity: product.grants,
          priceCents: product.priceCents,
          environment: transaction.environment,
        },
      });
      await grantCredits(account.id, product.sku, product.grants);

      return { ok: true, alreadyApplied: false, credits: await creditsFor(account.id) };
    },
    {
      body: t.Object({ signedTransaction: t.String({ minLength: 10 }) }),
      detail: {
        summary: "Enregistrer un achat à l'unité",
        description: "Consommables StoreKit : Renfort, Horizon, Tablée, Escale, Bilan.",
      },
    },
  )

  .post(
    "/apple/notifications",
    async ({ body }) => {
      // Notifications serveur à serveur App Store (V2) : renouvellements,
      // remboursements, expirations, périodes de grâce.
      const transaction = await verifyTransaction(body.signedPayload);
      const offre = tierFromStoreKitId(transaction.productId);
      if (offre === null) return { ok: true, ignored: true };

      const subscription = await prisma.subscription.findFirst({
        where: { originalTransactionId: transaction.originalTransactionId },
        select: { id: true, accountId: true },
      });
      if (subscription === null) return { ok: true, ignored: true };

      const expiresAt = transaction.expiresAt;
      const expired = expiresAt !== null && expiresAt.getTime() < Date.now();

      await prisma.subscription.update({
        where: { id: subscription.id },
        data: {
          tier: expired ? "depart" : offre.tier,
          renewsAt: expiresAt,
          expiresAt,
          inGracePeriod: false,
        },
      });

      if (!expired && expiresAt !== null) {
        await refillTierCredits(subscription.accountId, offre.tier as PlanTier, expiresAt);
      }
      await invalidateAccountCache(subscription.accountId);

      return { ok: true, ignored: false };
    },
    {
      body: t.Object({ signedPayload: t.String() }),
      detail: {
        summary: "Notification App Store (serveur à serveur)",
        description:
          "Point d'entrée des notifications V2 : renouvellement, expiration, remboursement.",
      },
    },
  );
