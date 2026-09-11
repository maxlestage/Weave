/**
 * Tests d'intégration du métier.
 *
 * Ils vérifient l'invariant central — jamais plus de trois fils, et rien du
 * contenu d'un fil en base — en passant par les vraies routes HTTP, le vrai
 * cache Redis et la vraie base SQLite de développement.
 */
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { MAX_ACTIVE_THREADS } from "@weave/contracts";
import { app } from "../src/index.ts";
import { clearLoom, keys } from "../src/lib/cache.ts";
import { emailHash } from "../src/lib/crypto.ts";
import { prisma } from "../src/lib/prisma.ts";
import { redis } from "../src/lib/redis.ts";

const BASE = "http://weave.test";

async function call(path: string, init: RequestInit = {}): Promise<Response> {
  return app.handle(new Request(`${BASE}${path}`, init));
}

async function json<T>(response: Response): Promise<T> {
  return (await response.json()) as T;
}

/** Lit une réponse attendue en succès, en remontant le corps en cas d'échec. */
async function ok<T>(response: Response): Promise<T> {
  if (!response.ok) {
    throw new Error(`Réponse ${response.status} : ${await response.text()}`);
  }
  return json<T>(response);
}

function post(body: unknown, token?: string): RequestInit {
  return {
    method: "POST",
    headers: {
      "content-type": "application/json",
      ...(token !== undefined ? { authorization: `Bearer ${token}` } : {}),
    },
    body: JSON.stringify(body),
  };
}

function auth(token: string): RequestInit {
  return { headers: { authorization: `Bearer ${token}` } };
}

/**
 * Ouvre une session pour un compte du jeu de données.
 *
 * Les compteurs anti-abus de la connexion sont remis à zéro au préalable :
 * sans cela, la suite dépendrait de l'état laissé dans Redis par les
 * exécutions précédentes — cinq demandes de code par quart d'heure suffisent
 * à la faire échouer au bout de trois passages, ou après quelques essais
 * manuels sur le même compte.
 */
async function login(local: string): Promise<{ token: string; accountId: string }> {
  const email = `${local}@weave.test`;
  await resetLoginThrottle(email);

  const requested = await ok<{ devCode?: string }>(
    await call("/v1/auth/otp/request", post({ email })),
  );
  expect(requested.devCode).toBeString();

  const verified = await json<{ session: { accessToken: string } | null }>(
    await call("/v1/auth/otp/verify", post({ email, code: requested.devCode })),
  );
  expect(verified.session).not.toBeNull();

  const account = await prisma.account.findUniqueOrThrow({
    where: { emailHash: emailHash(email) },
    select: { id: true },
  });

  return { token: verified.session!.accessToken, accountId: account.id };
}

interface LoomResponse {
  threads: {
    id: string;
    displayName: string;
    revealPercent: number;
    photoUrl: string | null;
    exchanges: number;
    awaitingYou: boolean;
    state: string;
    fragments: { id: string; kind: string }[];
  }[];
  freeSlots: number;
  nextWeavingAt: string;
  nextRefillAt: string | null;
  fromCache: boolean;
}

let session: { token: string; accountId: string };

beforeAll(async () => {
  const seeded = await prisma.account.count();
  if (seeded === 0) {
    throw new Error("Lancez `bun run db:seed` avant les tests d'intégration.");
  }
  session = await login("ines");
  await resetAccount(session.accountId);
});

afterAll(async () => {
  await resetAccount(session.accountId);
});

/**
 * Efface les compteurs de débit de la connexion.
 *
 * Le seau est indexé par empreinte d'e-mail et par adresse d'appel ; les tests
 * passant par `app.handle`, il n'y a pas de socket et l'adresse vaut
 * « inconnu » — voir `auth.routes.ts`.
 */
async function resetLoginThrottle(email: string): Promise<void> {
  const subjects = [emailHash(email), "inconnu"];
  for (const bucket of ["otp-request", "otp-verify"]) {
    for (const subject of subjects) {
      await redis.del(keys.rateLimit(bucket, subject));
    }
  }
}

/** Remet un compte à zéro : cache, registre et compteurs de débit. */
async function resetAccount(accountId: string): Promise<void> {
  await clearLoom(accountId);
  await redis.del(keys.refill(accountId));
  await redis.del(keys.rateLimit("weave", accountId));
  await redis.del(keys.rateLimit("message", accountId));
  await prisma.threadLedger.deleteMany({ where: { viewerId: accountId } });
}

describe("le métier", () => {
  test("compose au plus trois fils", async () => {
    const loom = await ok<LoomResponse>(await call("/v1/loom", auth(session.token)));
    expect(loom.threads.length).toBeLessThanOrEqual(MAX_ACTIVE_THREADS);
    expect(loom.threads.length).toBe(MAX_ACTIVE_THREADS);
    expect(loom.freeSlots).toBe(0);
    expect(loom.fromCache).toBe(true);
  });

  test("ne dépasse jamais trois, même sur des appels répétés", async () => {
    for (let i = 0; i < 8; i++) {
      const loom = await ok<LoomResponse>(await call("/v1/loom", auth(session.token)));
      expect(loom.threads.length).toBeLessThanOrEqual(MAX_ACTIVE_THREADS);
    }
    const count = Number(await redis.send("ZCARD", [keys.loom(session.accountId)]));
    expect(count).toBeLessThanOrEqual(MAX_ACTIVE_THREADS);
  });

  test("ne dépasse jamais trois sous appels concurrents", async () => {
    await clearLoom(session.accountId);
    await redis.del(keys.refill(session.accountId));

    await Promise.all(Array.from({ length: 6 }, () => call("/v1/loom", auth(session.token))));

    const count = Number(await redis.send("ZCARD", [keys.loom(session.accountId)]));
    expect(count).toBeLessThanOrEqual(MAX_ACTIVE_THREADS);
  });

  test("propose des personnes distinctes", async () => {
    const loom = await ok<LoomResponse>(await call("/v1/loom", auth(session.token)));
    const names = loom.threads.map((thread) => thread.displayName);
    expect(new Set(names).size).toBe(names.length);
  });

  test("sert la photo entièrement masquée au premier contact", async () => {
    const loom = await ok<LoomResponse>(await call("/v1/loom", auth(session.token)));
    for (const thread of loom.threads) {
      expect(thread.revealPercent).toBe(0);
      expect(thread.photoUrl).toContain("blur=100");
      expect(thread.photoUrl).toContain("sig=");
    }
  });

  test("n'écrit aucun contenu de profil proposé en base", async () => {
    const loom = await ok<LoomResponse>(await call("/v1/loom", auth(session.token)));
    const ledgers = await prisma.threadLedger.findMany({
      where: { viewerId: session.accountId },
    });

    expect(ledgers.length).toBeGreaterThan(0);
    // Le registre ne porte que des identifiants, un score et des dates.
    for (const ledger of ledgers) {
      const fields = Object.keys(ledger);
      expect(fields).toEqual([
        "id",
        "viewerId",
        "candidateId",
        "cacheKey",
        "outcome",
        "score",
        "servedAt",
        "resolvedAt",
        "expiresAt",
      ]);
    }
    // Et chaque fil affiché correspond bien à une ligne de registre.
    const keysInLedger = new Set(ledgers.map((l) => l.cacheKey));
    for (const thread of loom.threads) expect(keysInLedger.has(thread.id)).toBe(true);
  });

  test("le contenu d'un fil vit dans Redis, avec un TTL", async () => {
    const loom = await ok<LoomResponse>(await call("/v1/loom", auth(session.token)));
    const first = loom.threads[0]!;
    const ttl = Number(await redis.send("TTL", [keys.thread(first.id)]));
    expect(ttl).toBeGreaterThan(0);
    expect(ttl).toBeLessThanOrEqual(24 * 60 * 60);
  });

  test("une réponse engage le fil et dévoile un cran de la photo", async () => {
    const loom = await ok<LoomResponse>(await call("/v1/loom", auth(session.token)));
    const thread = loom.threads[0]!;
    const fragment = thread.fragments[0]!;

    const responded = await json<{ thread: LoomResponse["threads"][number] }>(
      await call(
        `/v1/loom/threads/${thread.id}/respond`,
        post({ fragmentId: fragment.id, body: "Votre dimanche ressemble au mien." }, session.token),
      ),
    );

    expect(responded.thread.state).toBe("engage");
    expect(responded.thread.exchanges).toBe(1);
    expect(responded.thread.revealPercent).toBe(33);
    expect(responded.thread.awaitingYou).toBe(false);
  });

  test("refuse une réponse trop courte", async () => {
    const loom = await ok<LoomResponse>(await call("/v1/loom", auth(session.token)));
    const thread = loom.threads.find((t) => t.awaitingYou)!;
    const response = await call(
      `/v1/loom/threads/${thread.id}/respond`,
      post({ fragmentId: thread.fragments[0]!.id, body: "ok" }, session.token),
    );
    expect(response.status).toBe(422);
  });

  test("refuse un fragment qui n'appartient pas au fil", async () => {
    const loom = await ok<LoomResponse>(await call("/v1/loom", auth(session.token)));
    const thread = loom.threads.find((t) => t.awaitingYou)!;
    const response = await call(
      `/v1/loom/threads/${thread.id}/respond`,
      post({ fragmentId: "fragment-inexistant", body: "Une réponse assez longue." }, session.token),
    );
    expect(response.status).toBe(422);
  });

  test("exige un crédit pour prolonger un fil", async () => {
    const loom = await ok<LoomResponse>(await call("/v1/loom", auth(session.token)));
    const response = await call(
      `/v1/loom/threads/${loom.threads[0]!.id}/extend`,
      post({}, session.token),
    );
    expect(response.status).toBe(402);

    const body = await json<{ error: string; details: { unitProductId: string } }>(response);
    expect(body.error).toBe("entitlement_required");
    // L'erreur indique comment obtenir le droit, à l'unité comme par abonnement.
    expect(body.details.unitProductId).toContain("unit.prolonge");
  });

  test("dénouer un fil libère une place et efface la carte du cache", async () => {
    const before = await ok<LoomResponse>(await call("/v1/loom", auth(session.token)));
    const victim = before.threads[before.threads.length - 1]!;

    const released = await call(
      `/v1/loom/threads/${victim.id}/release`,
      post({ reason: "pas la bonne trame" }, session.token),
    );
    expect(released.status).toBe(200);

    expect(await redis.get(keys.thread(victim.id))).toBeNull();

    const ledger = await prisma.threadLedger.findUniqueOrThrow({
      where: { cacheKey: victim.id },
    });
    expect(ledger.outcome).toBe("relache");
    expect(ledger.resolvedAt).not.toBeNull();
  });

  test("un fil dénoué ne peut plus recevoir de réponse", async () => {
    const ledger = await prisma.threadLedger.findFirstOrThrow({
      where: { viewerId: session.accountId, outcome: "relache" },
    });
    const response = await call(
      `/v1/loom/threads/${ledger.cacheKey}/respond`,
      post({ fragmentId: "peu-importe", body: "Une réponse suffisamment longue." }, session.token),
    );
    expect(response.status).toBe(410);
  });

  test("refuse un fil qui appartient à quelqu'un d'autre", async () => {
    const other = await login("jonas");
    await call("/v1/loom", auth(other.token));

    const ledger = await prisma.threadLedger.findFirstOrThrow({
      where: { viewerId: other.accountId },
    });
    const response = await call(
      `/v1/loom/threads/${ledger.cacheKey}/release`,
      post({}, session.token),
    );
    expect(response.status).toBe(410);

    await resetAccount(other.accountId);
  });

  test("refuse un accès sans jeton", async () => {
    expect((await call("/v1/loom")).status).toBe(401);
  });
});

describe("la santé du service", () => {
  test("signale la base et le cache", async () => {
    const health = await json<{ status: string; cache: { required: boolean } }>(
      await call("/health"),
    );
    expect(health.status).toBe("ok");
    // Sans cache, Weave n'a pas de fils : ce n'est pas une dépendance optionnelle.
    expect(health.cache.required).toBe(true);
  });
});
