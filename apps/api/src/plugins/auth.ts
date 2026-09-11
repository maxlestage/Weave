/**
 * Authentification par jeton porteur.
 *
 * Weave n'utilise pas de mot de passe : la connexion se fait par code à usage
 * unique envoyé par e-mail, puis par un couple access/refresh. L'access token
 * est court (15 min) ; le refresh est rotatif et stocké haché.
 */
import { Elysia } from "elysia";
import jwt from "@elysiajs/jwt";
import type { PlanTier } from "@weave/contracts";
import { env } from "../env.ts";
import { keys } from "../lib/cache.ts";
import { unauthorized } from "../lib/errors.ts";
import { prisma } from "../lib/prisma.ts";
import { getJson, setJson } from "../lib/redis.ts";
import { SESSION_CACHE_TTL_SECONDS } from "@weave/contracts";

export interface AuthenticatedAccount {
  id: string;
  handle: string;
  displayName: string;
  status: string;
  timezone: string;
  locale: string;
  tier: PlanTier;
  verified: boolean;
}

/**
 * Résumé d'identité mis en cache : évite un aller-retour base à chaque requête
 * authentifiée, tout en restant court pour que les changements de palier ou de
 * statut se propagent vite.
 */
async function loadAccount(accountId: string): Promise<AuthenticatedAccount | null> {
  const cached = await getJson<AuthenticatedAccount>(keys.identity(accountId));
  if (cached !== null) return cached;

  const row = await prisma.account.findUnique({
    where: { id: accountId },
    select: {
      id: true,
      handle: true,
      displayName: true,
      status: true,
      timezone: true,
      locale: true,
      verified: true,
      subscription: { select: { tier: true, expiresAt: true } },
    },
  });
  if (row === null) return null;

  const subscription = row.subscription;
  const active =
    subscription !== null &&
    (subscription.expiresAt === null || subscription.expiresAt.getTime() > Date.now());

  const account: AuthenticatedAccount = {
    id: row.id,
    handle: row.handle,
    displayName: row.displayName,
    status: row.status,
    timezone: row.timezone,
    locale: row.locale,
    verified: row.verified,
    tier: (active ? (subscription!.tier as PlanTier) : "depart") satisfies PlanTier,
  };

  await setJson(keys.identity(accountId), account, SESSION_CACHE_TTL_SECONDS);
  return account;
}

/** Invalide le résumé d'identité (changement de palier, de statut, de fuseau). */
export async function invalidateAccountCache(accountId: string): Promise<void> {
  const { redis } = await import("../lib/redis.ts");
  await redis.del(keys.identity(accountId));
}

export const authPlugin = new Elysia({ name: "weave/auth" })
  .use(
    jwt({
      name: "jwt",
      secret: env.auth.jwtSecret,
      exp: `${env.auth.accessTtlSeconds}s`,
      iss: "weave",
      aud: "weave-app",
    }),
  )
  .derive({ as: "scoped" }, async ({ jwt, headers }) => {
    const header = headers.authorization;
    let account: AuthenticatedAccount | null = null;

    if (header !== undefined && header.toLowerCase().startsWith("bearer ")) {
      const payload = await jwt.verify(header.slice(7).trim());
      if (payload !== false && typeof payload.sub === "string") {
        account = await loadAccount(payload.sub);
      }
    }

    return {
      account,
      /** Renvoie le compte authentifié, ou refuse la requête. */
      requireAccount(): AuthenticatedAccount {
        if (account === null) throw unauthorized();
        if (account.status === "suspended") {
          throw unauthorized("Ce compte est suspendu.");
        }
        return account;
      },
    };
  });
