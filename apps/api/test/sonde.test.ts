/**
 * La sonde de santé doit toujours répondre.
 *
 * C'est sa seule obligation absolue : vu de l'extérieur, un service qui ne
 * répond pas est indiscernable d'un service mort, et l'hébergeur finit par le
 * tuer. Un `PING` adressé à un Redis injoignable n'échoue pourtant pas — avec
 * `autoReconnect`, il est mis en file et attend indéfiniment.
 */
import { describe, expect, test } from "bun:test";
import { PROBE_TIMEOUT_MS, withTimeout } from "../src/lib/redis.ts";

describe("la borne des sondes", () => {
  test("laisse passer une réponse rapide", async () => {
    expect(await withTimeout(Promise.resolve(true), 1_000)).toBeTrue();
    expect(await withTimeout(Promise.resolve(false), 1_000)).toBeFalse();
  });

  test("déclare indisponible ce qui ne répond pas à temps", async () => {
    const debut = Date.now();
    // Une promesse qui n'aboutit jamais : exactement le cas d'un cache
    // injoignable dont la commande reste en file.
    const jamais = new Promise<boolean>(() => {});

    expect(await withTimeout(jamais, 50)).toBeFalse();
    expect(Date.now() - debut).toBeLessThan(1_000);
  });

  test("traite un échec comme une indisponibilité, sans rejet non traité", async () => {
    expect(await withTimeout(Promise.reject(new Error("connexion refusée")), 1_000)).toBeFalse();
  });

  test("la borne reste courte : une sonde n'est pas une requête métier", () => {
    expect(PROBE_TIMEOUT_MS).toBeLessThanOrEqual(5_000);
  });
});
