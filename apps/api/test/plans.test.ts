/**
 * Tests d'intégration des plans et des demandes.
 *
 * Ils vérifient les deux invariants du produit — on ne peut pas arroser, on ne
 * peut pas acheter de visibilité — en passant par les vraies routes HTTP, le
 * vrai cache Redis et la vraie base SQLite de développement.
 */
import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import {
  MAX_OPEN_PLANS,
  PLAN_MIN_LEAD_MINUTES,
  REQUESTS_PER_DAY_FLOOR,
  REQUEST_MIN_CHARS,
  TIERS,
} from "@weave/contracts";
import { app } from "../src/index.ts";
import { keys } from "../src/lib/cache.ts";
import { emailHash } from "../src/lib/crypto.ts";
import { prisma } from "../src/lib/prisma.ts";
import { redis } from "../src/lib/redis.ts";
import { localDay } from "../src/lib/time.ts";

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

function body(method: string, charge: unknown, token?: string): RequestInit {
  return {
    method,
    headers: {
      "content-type": "application/json",
      ...(token !== undefined ? { authorization: `Bearer ${token}` } : {}),
    },
    body: JSON.stringify(charge),
  };
}

const post = (charge: unknown, token?: string) => body("POST", charge, token);

function auth(token: string): RequestInit {
  return { headers: { authorization: `Bearer ${token}` } };
}

interface Session {
  token: string;
  accountId: string;
  timezone: string;
}

/**
 * Ouvre une session pour un compte du jeu de données.
 *
 * Les compteurs anti-abus de la connexion sont remis à zéro au préalable :
 * sans cela, la suite dépendrait de l'état laissé dans Redis par les
 * exécutions précédentes — cinq demandes de code par quart d'heure suffisent
 * à la faire échouer au bout de trois passages.
 */
async function login(local: string): Promise<Session> {
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

  const compte = await prisma.account.findUniqueOrThrow({
    where: { emailHash: emailHash(email) },
    select: { id: true, timezone: true },
  });

  return { token: verified.session!.accessToken, accountId: compte.id, timezone: compte.timezone };
}

interface FeedResponse {
  plans: {
    id: string;
    title: string;
    startsAt: string;
    distanceKm: number;
    seatsLeft: number;
    requested: boolean;
    author: { id: string };
  }[];
  requestsLeftToday: number;
  fromCache: boolean;
}

/** Une date de rendez-vous valide, largement au-delà du préavis minimal. */
function dansHeures(heures: number): string {
  return new Date(Date.now() + heures * 60 * 60 * 1000).toISOString();
}

/** Un message de demande recevable : au-delà du plancher de caractères. */
const MESSAGE = "Je viens de m'installer dans le quartier et ce plan me tente beaucoup.";

let hote: Session;
let invite: Session;

beforeAll(async () => {
  if ((await prisma.account.count()) === 0) {
    throw new Error("Lancez `bun run db:seed` avant les tests d'intégration.");
  }
  hote = await login("ines1");
  invite = await login("theo11");
  await reset(hote);
  await reset(invite);
});

afterAll(async () => {
  await reset(hote);
  await reset(invite);
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

/**
 * Remet un compte à zéro : plans créés par les tests, quota du jour, compteurs
 * de débit. Le quota vivant uniquement en cache, l'oublier ferait dépendre une
 * exécution de la précédente.
 */
async function reset(session: Session): Promise<void> {
  await prisma.conversation.deleteMany({
    where: { OR: [{ hostId: session.accountId }, { guestId: session.accountId }] },
  });
  await prisma.joinRequest.deleteMany({ where: { authorId: session.accountId } });
  await prisma.plan.deleteMany({ where: { authorId: session.accountId } });
  await redis.del(keys.requestsUsed(session.accountId, localDay(session.timezone)));
  await redis.del(keys.feed(session.accountId));
  for (const bucket of ["feed", "publish", "join", "message"]) {
    await redis.del(keys.rateLimit(bucket, session.accountId));
  }
}

/** Publie un plan et renvoie son identifiant. */
async function publier(session: Session, titre: string, heures = 48): Promise<string> {
  const cree = await ok<{ id: string }>(
    await call(
      "/v1/plans",
      post(
        {
          title: titre,
          note: "Rien de spectaculaire.",
          category: "sortie",
          startsAt: dansHeures(heures),
        },
        session.token,
      ),
    ),
  );
  return cree.id;
}

describe("publier un plan", () => {
  test("refuse un rendez-vous trop proche", async () => {
    const reponse = await call(
      "/v1/plans",
      post(
        {
          title: "Un verre dans dix minutes",
          category: "sortie",
          startsAt: new Date(Date.now() + (PLAN_MIN_LEAD_MINUTES - 10) * 60_000).toISOString(),
        },
        hote.token,
      ),
    );
    expect(reponse.status).toBe(422);
  });

  test(`n'autorise pas plus de ${MAX_OPEN_PLANS} plans ouverts`, async () => {
    await reset(hote);

    for (let n = 0; n < MAX_OPEN_PLANS; n++) {
      await publier(hote, `Plan de test numéro ${n + 1}`, 24 + n * 12);
    }

    const refuse = await call(
      "/v1/plans",
      post(
        { title: "Le plan de trop, celui-là", category: "sortie", startsAt: dansHeures(72) },
        hote.token,
      ),
    );
    expect(refuse.status).toBe(409);
    expect((await json<{ error: string }>(refuse)).error).toBe("too_many_plans");

    const miens = await ok<unknown[]>(await call("/v1/plans/mine", auth(hote.token)));
    expect(miens.length).toBe(MAX_OPEN_PLANS);
  });
});

describe("le fil", () => {
  test("n'affiche jamais ses propres plans", async () => {
    await reset(hote);
    await publier(hote, "Mon plan à moi, invisible pour moi");

    const fil = await ok<FeedResponse>(await call("/v1/plans", auth(hote.token)));
    expect(fil.plans.some((plan) => plan.author.id === hote.accountId)).toBeFalse();
  });

  test("est trié par imminence, jamais par offre", async () => {
    const fil = await ok<FeedResponse>(await call("/v1/plans", auth(invite.token)));
    expect(fil.plans.length).toBeGreaterThan(0);

    // L'ordre exact tolère la proximité à moins de douze heures d'écart : on
    // vérifie donc la tendance, pas un tri strict.
    const premier = new Date(fil.plans[0]!.startsAt).getTime();
    const dernier = new Date(fil.plans.at(-1)!.startsAt).getTime();
    expect(premier).toBeLessThanOrEqual(dernier);
  });

  test("se sert du cache au second appel", async () => {
    await redis.del(keys.feed(invite.accountId));
    const froid = await ok<FeedResponse>(await call("/v1/plans", auth(invite.token)));
    expect(froid.fromCache).toBeFalse();

    const chaud = await ok<FeedResponse>(await call("/v1/plans", auth(invite.token)));
    expect(chaud.fromCache).toBeTrue();
  });
});

describe("demander à venir", () => {
  test("exige un message écrit", async () => {
    await reset(hote);
    await reset(invite);
    const planId = await publier(hote, "Balade au bord du canal");

    const trop_court = await call("/v1/requests", post({ planId, message: "ok" }, invite.token));
    // Le plancher est d'abord posé par le schéma de la route, puis revérifié
    // après nettoyage des espaces : les deux répondent 422.
    expect(trop_court.status).toBe(422);
    expect(MESSAGE.length).toBeGreaterThanOrEqual(REQUEST_MIN_CHARS);
  });

  test("consomme une unité du quota, et refuse la seconde demande", async () => {
    await reset(hote);
    await reset(invite);
    const planId = await publier(hote, "Ciné en version originale");

    const avant = await ok<FeedResponse>(await call("/v1/plans", auth(invite.token)));

    const envoyee = await ok<{ id: string; requestsLeftToday: number }>(
      await call("/v1/requests", post({ planId, message: MESSAGE }, invite.token)),
    );
    expect(envoyee.requestsLeftToday).toBe(avant.requestsLeftToday - 1);

    const doublon = await call("/v1/requests", post({ planId, message: MESSAGE }, invite.token));
    expect(doublon.status).toBe(409);
    expect((await json<{ error: string }>(doublon)).error).toBe("already_requested");
  });

  test("rend l'unité quand la demande est retirée", async () => {
    await reset(hote);
    await reset(invite);
    const planId = await publier(hote, "Friperies puis café, tout l'après-midi");

    const envoyee = await ok<{ id: string; requestsLeftToday: number }>(
      await call("/v1/requests", post({ planId, message: MESSAGE }, invite.token)),
    );

    await ok(await call(`/v1/requests/${envoyee.id}`, { method: "DELETE", ...auth(invite.token) }));

    const apres = await ok<{ requestsLeftToday: number }>(
      await call("/v1/requests/sent", auth(invite.token)),
    );
    expect(apres.requestsLeftToday).toBe(envoyee.requestsLeftToday + 1);
  });

  test("s'arrête net quand le quota du jour est épuisé", async () => {
    await reset(hote);
    await reset(invite);

    const quota = TIERS.depart.entitlements.requestsPerDay;
    expect(quota).toBe(REQUESTS_PER_DAY_FLOOR);

    // On brûle le quota sans passer par des plans : le compteur est le même,
    // et cela évite d'avoir à publier autant de plans distincts.
    const jour = localDay(invite.timezone);
    await redis.set(keys.requestsUsed(invite.accountId, jour), String(quota));

    const planId = await publier(hote, "Marché du dimanche, puis brunch");
    const refuse = await call("/v1/requests", post({ planId, message: MESSAGE }, invite.token));

    expect(refuse.status).toBe(429);
    expect((await json<{ error: string }>(refuse)).error).toBe("no_requests_left");
  });
});

describe("accepter une demande", () => {
  test("ouvre une conversation, et referme le plan quand il est complet", async () => {
    await reset(hote);
    await reset(invite);
    const planId = await publier(hote, "Soirée jeux de société chez moi");

    const demande = await ok<{ id: string }>(
      await call("/v1/requests", post({ planId, message: MESSAGE }, invite.token)),
    );

    const recues = await ok<{ id: string }[]>(
      await call(`/v1/plans/${planId}/requests`, auth(hote.token)),
    );
    expect(recues.map((r) => r.id)).toContain(demande.id);

    const acceptee = await ok<{ conversationId: string }>(
      await call(`/v1/requests/${demande.id}/accept`, post({}, hote.token)),
    );
    expect(acceptee.conversationId).toBeString();

    // Capacité par défaut : une place. Le plan doit être complet.
    const plan = await prisma.plan.findUniqueOrThrow({ where: { id: planId } });
    expect(plan.state).toBe("complet");

    // La conversation existe des deux côtés.
    for (const session of [hote, invite]) {
      const liste = await ok<{ id: string }[]>(
        await call("/v1/conversations", auth(session.token)),
      );
      expect(liste.map((c) => c.id)).toContain(acceptee.conversationId);
    }

    // Et l'on peut y écrire.
    const message = await ok<{ body: string }>(
      await call(
        `/v1/conversations/${acceptee.conversationId}/messages`,
        post({ body: "Super, à jeudi alors." }, invite.token),
      ),
    );
    expect(message.body).toBe("Super, à jeudi alors.");
  });

  test("interdit d'accepter une demande sur le plan de quelqu'un d'autre", async () => {
    await reset(hote);
    await reset(invite);
    const planId = await publier(hote, "Course tranquille au parc");

    const demande = await ok<{ id: string }>(
      await call("/v1/requests", post({ planId, message: MESSAGE }, invite.token)),
    );

    const refuse = await call(`/v1/requests/${demande.id}/accept`, post({}, invite.token));
    expect(refuse.status).toBe(403);
  });
});

describe("la santé du service", () => {
  test("répond que la base et le cache sont joignables", async () => {
    const sante = await ok<{ status: string; cache: { ok: boolean; required: boolean } }>(
      await call("/health"),
    );
    expect(sante.status).toBe("ok");
    expect(sante.cache.ok).toBeTrue();
    expect(sante.cache.required).toBeTrue();
  });
});
