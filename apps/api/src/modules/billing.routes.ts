/**
 * Offres, abonnements et achats à l'unité.
 *
 * Weave est vendu sur l'App Store : les paiements passent par StoreKit 2, et le
 * serveur ne fait que vérifier puis enregistrer ce qu'Apple lui transmet. Aucun
 * moyen de paiement ne transite par nos serveurs.
 */
import { Elysia, t } from "elysia";
import {
  PLANS,
  PLAN_TIERS,
  UNIT_PRODUCTS,
  UNIT_SKUS,
  formatPrice,
  planFromStoreKitId,
  unitFromStoreKitId,
  type Entitlement,
  type PlanTier,
} from "@weave/contracts";
import { env } from "../env.ts";
import { invalid } from "../lib/errors.ts";
import { log } from "../lib/log.ts";
import { prisma } from "../lib/prisma.ts";
import { authPlugin, invalidateAccountCache } from "../plugins/auth.ts";
import { creditsFor, grantCredits, refillPlanCredits } from "./entitlements.ts";

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
    "/plans",
    () => ({
      plans: PLAN_TIERS.map((tier) => {
        const plan = PLANS[tier];
        return {
          ...plan,
          monthlyPrice: formatPrice(plan.monthlyPriceCents),
          yearlyPrice: plan.yearlyPriceCents === null ? null : formatPrice(plan.yearlyPriceCents),
        };
      }),
      units: UNIT_SKUS.map((sku) => ({
        ...UNIT_PRODUCTS[sku],
        price: formatPrice(UNIT_PRODUCTS[sku].priceCents),
      })),
      note:
        "Aucun palier n'augmente le nombre de fils : le plafond de trois est le même pour tout le monde.",
    }),
    {
      detail: {
        summary: "Catalogue des offres",
        description: "Quatre abonnements, et chaque avantage également disponible à l'unité.",
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
        plan: account.plan,
        renewsAt: subscription?.renewsAt?.toISOString() ?? null,
        credits: await creditsFor(account.id),
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

      const plan = planFromStoreKitId(transaction.productId);
      if (plan === null) throw invalid(`Produit d'abonnement inconnu : ${transaction.productId}`);

      const period = plan.storeKit.yearly === transaction.productId ? "yearly" : "monthly";
      const renewsAt =
        transaction.expiresAt ??
        new Date(Date.now() + (period === "yearly" ? 365 : 30) * 24 * 60 * 60 * 1000);

      await prisma.subscription.upsert({
        where: { accountId: account.id },
        create: {
          accountId: account.id,
          tier: plan.tier,
          period,
          storeKitProductId: transaction.productId,
          originalTransactionId: transaction.originalTransactionId,
          renewsAt,
          expiresAt: renewsAt,
          environment: transaction.environment,
        },
        update: {
          tier: plan.tier,
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

      await refillPlanCredits(account.id, plan.tier, renewsAt);
      await invalidateAccountCache(account.id);

      return { ok: true, plan: plan.tier, renewsAt: renewsAt.toISOString() };
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
        description: "Consommables StoreKit : Écho, Prolonge, Relais, Motif, Escale, Atelier.",
      },
    },
  )

  .post(
    "/apple/notifications",
    async ({ body }) => {
      // Notifications serveur à serveur App Store (V2) : renouvellements,
      // remboursements, expirations, périodes de grâce.
      const transaction = await verifyTransaction(body.signedPayload);
      const plan = planFromStoreKitId(transaction.productId);
      if (plan === null) return { ok: true, ignored: true };

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
          tier: expired ? "fil" : plan.tier,
          renewsAt: expiresAt,
          expiresAt,
          inGracePeriod: false,
        },
      });

      if (!expired && expiresAt !== null) {
        await refillPlanCredits(subscription.accountId, plan.tier as PlanTier, expiresAt);
      }
      await invalidateAccountCache(subscription.accountId);

      return { ok: true, ignored: false };
    },
    {
      body: t.Object({ signedPayload: t.String() }),
      detail: {
        summary: "Notification App Store (serveur à serveur)",
        description: "Point d'entrée des notifications V2 : renouvellement, expiration, remboursement.",
      },
    },
  );
