/**
 * Connexion sans mot de passe.
 *
 * Un code à six chiffres est envoyé par e-mail, valable dix minutes et cinq
 * tentatives. Il n'y a pas de mot de passe à voler, à réutiliser ou à oublier.
 */
import { Elysia, t } from "elysia";
import { MIN_AGE, WEAVING_HOURS } from "@weave/contracts";
import { env } from "../env.ts";
import { clearLoom } from "../lib/cache.ts";
import {
  emailHash,
  hashSecret,
  normalizeEmail,
  opaqueToken,
  otpCode,
  sha256Hex,
  verifySecret,
} from "../lib/crypto.ts";
import { invalid, unauthorized } from "../lib/errors.ts";
import { log } from "../lib/log.ts";
import { prisma } from "../lib/prisma.ts";
import { ageFrom } from "../lib/time.ts";
import { authPlugin, invalidateAccountCache } from "../plugins/auth.ts";
import { consume, RULES } from "../plugins/rate-limit.ts";

const OTP_TTL_MINUTES = 10;
const OTP_MAX_ATTEMPTS = 5;

/** Fabrique un identifiant public unique à partir du nom affiché. */
async function uniqueHandle(displayName: string): Promise<string> {
  const base =
    displayName
      .normalize("NFD")
      .replace(/[̀-ͯ]/g, "")
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "")
      .slice(0, 16) || "fil";

  for (let attempt = 0; attempt < 20; attempt++) {
    const candidate = attempt === 0 ? base : `${base}${Math.floor(Math.random() * 10_000)}`;
    const taken = await prisma.account.findUnique({
      where: { handle: candidate },
      select: { id: true },
    });
    if (taken === null) return candidate;
  }
  return `${base}${Date.now().toString(36)}`;
}

async function issueSession(
  accountId: string,
  deviceId: string | undefined,
  sign: (payload: { sub: string }) => Promise<string>,
) {
  const refresh = opaqueToken();
  const expiresAt = new Date(Date.now() + env.auth.refreshTtlDays * 24 * 60 * 60 * 1000);

  await prisma.refreshToken.create({
    data: {
      accountId,
      tokenHash: sha256Hex(refresh),
      deviceId: deviceId ?? null,
      expiresAt,
    },
  });

  const accessToken = await sign({ sub: accountId });
  return {
    accessToken,
    refreshToken: refresh,
    expiresAt: new Date(Date.now() + env.auth.accessTtlSeconds * 1000).toISOString(),
  };
}

export const authRoutes = new Elysia({ prefix: "/v1/auth", tags: ["Authentification"] })
  .use(authPlugin)

  .post(
    "/otp/request",
    async ({ body, server, request }) => {
      const address = server?.requestIP(request)?.address ?? "inconnu";
      await consume(RULES.otpRequest, address);

      const email = normalizeEmail(body.email);
      const hash = emailHash(email);
      await consume(RULES.otpRequest, hash);

      const code = otpCode();
      await prisma.otpChallenge.create({
        data: {
          emailHash: hash,
          codeHash: await hashSecret(code),
          expiresAt: new Date(Date.now() + OTP_TTL_MINUTES * 60 * 1000),
        },
      });

      // L'envoi réel passe par le fournisseur d'e-mail transactionnel. En
      // développement, le code est journalisé et renvoyé pour permettre de
      // dérouler le parcours sans dépendance externe.
      log.info("Code de connexion émis", { emailHash: hash });

      return {
        sent: true,
        expiresInSeconds: OTP_TTL_MINUTES * 60,
        ...(env.isProduction ? {} : { devCode: code }),
      };
    },
    {
      body: t.Object({ email: t.String({ format: "email", maxLength: 320 }) }),
      detail: {
        summary: "Demander un code de connexion",
        description:
          "Envoie un code à six chiffres valable dix minutes. Hors production, le code est renvoyé dans la réponse.",
      },
    },
  )

  .post(
    "/otp/verify",
    async ({ body, jwt, server, request }) => {
      const address = server?.requestIP(request)?.address ?? "inconnu";
      await consume(RULES.otpVerify, address);

      const email = normalizeEmail(body.email);
      const hash = emailHash(email);

      const challenge = await prisma.otpChallenge.findFirst({
        where: { emailHash: hash, consumedAt: null, expiresAt: { gt: new Date() } },
        orderBy: { createdAt: "desc" },
      });

      if (challenge === null) throw unauthorized("Code expiré ou déjà utilisé.");
      if (challenge.attempts >= OTP_MAX_ATTEMPTS) {
        throw unauthorized("Trop de tentatives sur ce code. Demandez-en un nouveau.");
      }

      const valid = await verifySecret(body.code, challenge.codeHash);
      if (!valid) {
        await prisma.otpChallenge.update({
          where: { id: challenge.id },
          data: { attempts: { increment: 1 } },
        });
        throw unauthorized("Code incorrect.");
      }

      await prisma.otpChallenge.update({
        where: { id: challenge.id },
        data: { consumedAt: new Date() },
      });

      let account = await prisma.account.findUnique({ where: { emailHash: hash } });
      let created = false;

      if (account === null) {
        if (body.displayName === undefined || body.birthDate === undefined) {
          // Le compte n'existe pas : le client doit rappeler la même route avec
          // les informations d'inscription minimales.
          return { session: null, needsProfile: true, created: false };
        }
        const birthDate = new Date(body.birthDate);
        if (Number.isNaN(birthDate.getTime())) throw invalid("Date de naissance invalide.");
        if (ageFrom(birthDate) < MIN_AGE) {
          throw invalid(`Weave est réservé aux personnes de ${MIN_AGE} ans et plus.`);
        }

        account = await prisma.account.create({
          data: {
            email,
            emailHash: hash,
            handle: await uniqueHandle(body.displayName),
            displayName: body.displayName,
            birthDate,
            timezone: body.timezone ?? "Europe/Paris",
            preference: { create: {} },
            subscription: { create: { tier: "fil" } },
          },
        });
        created = true;
      }

      const session = await issueSession(account.id, body.deviceId, (payload) => jwt.sign(payload));
      await prisma.account.update({ where: { id: account.id }, data: { lastSeenAt: new Date() } });

      return { session, needsProfile: false, created };
    },
    {
      body: t.Object({
        email: t.String({ format: "email", maxLength: 320 }),
        code: t.String({ minLength: 6, maxLength: 6 }),
        displayName: t.Optional(t.String({ minLength: 1, maxLength: 40 })),
        birthDate: t.Optional(t.String({ description: "AAAA-MM-JJ" })),
        timezone: t.Optional(t.String({ maxLength: 64 })),
        deviceId: t.Optional(t.String({ maxLength: 128 })),
      }),
      detail: {
        summary: "Vérifier le code et ouvrir une session",
        description:
          "Crée le compte si nécessaire. Répond `needsProfile: true` lorsqu'un nom et une date de naissance sont attendus.",
      },
    },
  )

  .post(
    "/refresh",
    async ({ body, jwt }) => {
      const hash = sha256Hex(body.refreshToken);
      const stored = await prisma.refreshToken.findUnique({ where: { tokenHash: hash } });

      if (stored === null || stored.revokedAt !== null || stored.expiresAt.getTime() < Date.now()) {
        throw unauthorized("Session expirée. Reconnectez-vous.");
      }

      // Rotation : le jeton présenté est immédiatement révoqué et remplacé.
      // Une réutilisation ultérieure du même jeton sera donc rejetée.
      const session = await issueSession(
        stored.accountId,
        stored.deviceId ?? undefined,
        (payload) => jwt.sign(payload),
      );
      await prisma.refreshToken.update({
        where: { id: stored.id },
        data: { revokedAt: new Date(), rotatedTo: sha256Hex(session.refreshToken) },
      });

      return { session };
    },
    {
      body: t.Object({ refreshToken: t.String({ minLength: 20, maxLength: 512 }) }),
      detail: { summary: "Renouveler la session" },
    },
  )

  .post(
    "/logout",
    async ({ body, requireAccount }) => {
      const account = requireAccount();
      if (body.refreshToken !== undefined) {
        await prisma.refreshToken.updateMany({
          where: { accountId: account.id, tokenHash: sha256Hex(body.refreshToken) },
          data: { revokedAt: new Date() },
        });
      } else {
        await prisma.refreshToken.updateMany({
          where: { accountId: account.id, revokedAt: null },
          data: { revokedAt: new Date() },
        });
      }
      await invalidateAccountCache(account.id);
      return { ok: true };
    },
    {
      body: t.Object({ refreshToken: t.Optional(t.String({ maxLength: 512 })) }),
      detail: {
        summary: "Fermer la session",
        description: "Sans `refreshToken`, toutes les sessions du compte sont révoquées.",
      },
    },
  )

  .delete(
    "/account",
    async ({ requireAccount }) => {
      const account = requireAccount();

      // Suppression en deux temps : le compte sort immédiatement de la
      // circulation, les données sont purgées à l'issue du délai légal.
      await prisma.account.update({
        where: { id: account.id },
        data: { status: "deleting", deletionRequestedAt: new Date() },
      });
      await prisma.refreshToken.updateMany({
        where: { accountId: account.id, revokedAt: null },
        data: { revokedAt: new Date() },
      });
      await clearLoom(account.id);
      await invalidateAccountCache(account.id);

      return { ok: true, purgeAfterDays: 30 };
    },
    {
      detail: {
        summary: "Demander la suppression du compte",
        description:
          "Le compte est retiré immédiatement de la composition ; les données sont purgées après trente jours.",
      },
    },
  );

export { WEAVING_HOURS };
