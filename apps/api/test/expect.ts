/**
 * Assertions, posées sur `node:assert`.
 *
 * Le lanceur de tests de Node ne fournit pas de `expect`. Plutôt que de tirer
 * une bibliothèque entière pour une dizaine de formes, en voici exactement
 * autant qu'il en faut — et pas une de plus : chacune est utilisée quelque
 * part, et le jour où l'une ne sert plus, elle se voit.
 *
 * Les messages d'échec passent par `assert`, qui montre déjà la valeur reçue et
 * la valeur attendue.
 */
import { strict as assert } from "node:assert";

export interface Attente<T> {
  toBe(attendu: T): void;
  toBeTrue(): void;
  toBeFalse(): void;
  toBeNull(): void;
  toBeString(): void;
  toBeGreaterThan(borne: number): void;
  toBeGreaterThanOrEqual(borne: number): void;
  toBeLessThan(borne: number): void;
  toBeLessThanOrEqual(borne: number): void;
  toContain(element: unknown): void;
  readonly not: { toBeNull(): void; toBe(attendu: T): void };
}

export function expect<T>(recu: T): Attente<T> {
  return {
    toBe: (attendu) => assert.deepStrictEqual(recu, attendu),
    toBeTrue: () => assert.strictEqual(recu, true),
    toBeFalse: () => assert.strictEqual(recu, false),
    toBeNull: () => assert.strictEqual(recu, null),
    toBeString: () => assert.strictEqual(typeof recu, "string"),
    toBeGreaterThan: (borne) =>
      assert.ok((recu as number) > borne, `${recu} devrait dépasser ${borne}`),
    toBeGreaterThanOrEqual: (borne) =>
      assert.ok((recu as number) >= borne, `${recu} devrait valoir au moins ${borne}`),
    toBeLessThan: (borne) =>
      assert.ok((recu as number) < borne, `${recu} devrait rester sous ${borne}`),
    toBeLessThanOrEqual: (borne) =>
      assert.ok((recu as number) <= borne, `${recu} devrait valoir au plus ${borne}`),
    toContain: (element) =>
      assert.ok(
        (recu as unknown[]).includes(element),
        `${JSON.stringify(recu)} devrait contenir ${JSON.stringify(element)}`,
      ),
    not: {
      toBeNull: () => assert.notStrictEqual(recu, null),
      toBe: (attendu) => assert.notDeepStrictEqual(recu, attendu),
    },
  };
}
