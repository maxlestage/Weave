/**
 * Le mode d'exécution ne se devine pas : il se déduit, et mal le déduire coûte
 * un déploiement.
 *
 * `NODE_ENV` oubliée sur un hébergeur faisait démarrer Weave avec les réglages
 * du développement : repli silencieux sur SQLite, puis échec sur un client
 * Prisma absent du slug — un message qui ne dit rien de la variable manquante.
 *
 * La configuration est lue une fois, au chargement du module. On l'observe donc
 * depuis un processus fils, seule façon de lui présenter un environnement
 * choisi.
 */
import { describe, expect, test } from "bun:test";

const ENV_TS = new URL("../src/env.ts", import.meta.url).pathname;

/** Démarre un processus qui lit la configuration et rapporte ce qu'il en tire. */
async function lireEnv(
  environnement: Record<string, string>,
): Promise<{ code: number; sortie: string }> {
  const processus = Bun.spawn(
    [
      "bun",
      "-e",
      `const { env } = await import(${JSON.stringify(ENV_TS)}); console.log(JSON.stringify({ mode: env.mode, driver: env.db.driver }));`,
    ],
    {
      // `env` remplace l'environnement en entier : sans cela, le NODE_ENV du
      // lanceur de tests fausserait chaque cas.
      env: { PATH: process.env.PATH ?? "", ...environnement },
      stdout: "pipe",
      stderr: "pipe",
    },
  );
  const [sortie, erreur, code] = await Promise.all([
    new Response(processus.stdout).text(),
    new Response(processus.stderr).text(),
    processus.exited,
  ]);
  return { code, sortie: sortie + erreur };
}

const SECRETS = {
  JWT_SECRET: "un-secret-de-test-assez-long-pour-passer-la-validation",
  MEDIA_SIGNING_SECRET: "un-autre-secret-de-test-assez-long-pour-passer",
  DATABASE_URL: "postgresql://u:p@127.0.0.1:5432/x",
  REDIS_URL: "redis://127.0.0.1:6379",
  PUBLIC_WEB_ORIGIN: "https://exemple.test",
};

describe("le mode d'exécution", () => {
  test("sur un dyno, NODE_ENV oubliée ne vaut pas « développement »", async () => {
    const { sortie } = await lireEnv({ ...SECRETS, DYNO: "web.1" });

    expect(sortie).toContain('"mode":"production"');
    // C'est la conséquence qui comptait : PostgreSQL, et non le repli SQLite
    // dont le client n'est pas dans le slug.
    expect(sortie).toContain('"driver":"postgres"');
  });

  test("hors hébergeur, l'absence de NODE_ENV vaut toujours « développement »", async () => {
    const { sortie } = await lireEnv({});

    expect(sortie).toContain('"mode":"development"');
    expect(sortie).toContain('"driver":"sqlite"');
  });

  test("NODE_ENV explicite l'emporte sur la détection", async () => {
    const { sortie } = await lireEnv({ ...SECRETS, DYNO: "web.1", NODE_ENV: "test" });

    expect(sortie).toContain('"mode":"test"');
  });
});
