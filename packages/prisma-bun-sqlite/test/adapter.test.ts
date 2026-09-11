import { describe, expect, test } from "bun:test";
import { ColumnTypeEnum } from "@prisma/driver-adapter-utils";
import { PrismaBunSqlite } from "../src/index.ts";
import { mapArg, mapDeclaredType, mapRow, resolveColumnTypes } from "../src/conversion.ts";
import { Mutex } from "../src/mutex.ts";

const SCHEMA = `
  CREATE TABLE accounts (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    age INTEGER NOT NULL,
    ratio REAL,
    created_at DATETIME NOT NULL
  );
  CREATE TABLE tokens (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE
  );
`;

async function connect() {
  const adapter = await new PrismaBunSqlite({ url: ":memory:" }).connect();
  await adapter.executeScript(SCHEMA);
  return adapter;
}

const noArgs = { args: [], argTypes: [] } as const;

describe("conversion des types déclarés", () => {
  test("mappe les types SQLite usuels", () => {
    expect(mapDeclaredType("INTEGER")).toBe(ColumnTypeEnum.Int32);
    expect(mapDeclaredType("bigint")).toBe(ColumnTypeEnum.Int64);
    expect(mapDeclaredType("DATETIME")).toBe(ColumnTypeEnum.DateTime);
    expect(mapDeclaredType("VARCHAR")).toBe(ColumnTypeEnum.Text);
    expect(mapDeclaredType("BLOB")).toBe(ColumnTypeEnum.Bytes);
    expect(mapDeclaredType("BOOLEAN")).toBe(ColumnTypeEnum.Boolean);
  });

  test("laisse les types inconnus à inférer", () => {
    expect(mapDeclaredType("QUELQUECHOSE")).toBeNull();
    expect(mapDeclaredType(null)).toBeNull();
  });

  test("infère le type d'une colonne calculée depuis la première valeur non nulle", () => {
    const types = resolveColumnTypes([null], [[null], ["texte"]]);
    expect(types[0]).toBe(ColumnTypeEnum.Text);
  });
});

describe("normalisation des lignes", () => {
  test("ramène un BigInt sûr vers un number", () => {
    expect(mapRow([42n], [ColumnTypeEnum.Int64])).toEqual([42]);
  });

  test("conserve un BigInt hors plage sous forme de chaîne", () => {
    expect(mapRow([9007199254740993n], [ColumnTypeEnum.Int64])).toEqual(["9007199254740993"]);
  });

  test("convertit un timestamp numérique en ISO 8601", () => {
    expect(mapRow([0], [ColumnTypeEnum.DateTime])).toEqual(["1970-01-01T00:00:00.000Z"]);
  });
});

describe("normalisation des paramètres", () => {
  test("écrit les dates en ISO 8601 avec décalage explicite", () => {
    const out = mapArg(new Date("2026-09-10T12:00:00Z"), undefined, "iso8601");
    expect(out).toBe("2026-09-10T12:00:00.000+00:00");
  });

  test("écrit les dates en millisecondes si demandé", () => {
    expect(mapArg(new Date(0), undefined, "unixepoch-ms")).toBe(0);
  });

  test("convertit les booléens en 0/1", () => {
    expect(mapArg(true, undefined, "iso8601")).toBe(1);
    expect(mapArg(false, undefined, "iso8601")).toBe(0);
  });

  test("réinterprète une chaîne selon le type scalaire annoncé", () => {
    expect(mapArg("17", { scalarType: "int", arity: "scalar" }, "iso8601")).toBe(17);
    expect(mapArg("1.5", { scalarType: "float", arity: "scalar" }, "iso8601")).toBe(1.5);
    expect(mapArg("9", { scalarType: "bigint", arity: "scalar" }, "iso8601")).toBe(9n);
  });
});

describe("adaptateur", () => {
  test("exécute une requête et renvoie noms et types de colonnes", async () => {
    const adapter = await connect();
    await adapter.executeRaw({
      sql: "INSERT INTO accounts VALUES (?, ?, ?, ?, ?)",
      args: ["a1", "a@weave.test", 30, 1.5, new Date("2026-01-01T00:00:00Z")],
      argTypes: [
        { scalarType: "string", arity: "scalar" },
        { scalarType: "string", arity: "scalar" },
        { scalarType: "int", arity: "scalar" },
        { scalarType: "float", arity: "scalar" },
        { scalarType: "datetime", arity: "scalar" },
      ],
    });

    const result = await adapter.queryRaw({
      sql: "SELECT id, age, ratio FROM accounts",
      ...noArgs,
    });
    expect(result.columnNames).toEqual(["id", "age", "ratio"]);
    expect(result.columnTypes).toEqual([
      ColumnTypeEnum.Text,
      ColumnTypeEnum.Int32,
      ColumnTypeEnum.Double,
    ]);
    expect(result.rows).toEqual([["a1", 30, 1.5]]);
    await adapter.dispose();
  });

  test("renvoie le nombre de lignes affectées", async () => {
    const adapter = await connect();
    const inserted = await adapter.executeRaw({
      sql: "INSERT INTO accounts VALUES ('a1','a@weave.test',30,NULL,'2026-01-01')",
      ...noArgs,
    });
    expect(inserted).toBe(1);
    await adapter.dispose();
  });

  test("traduit une violation d'unicité", async () => {
    const adapter = await connect();
    const insert = {
      sql: "INSERT INTO accounts VALUES ('a1','a@weave.test',30,NULL,'2026-01-01')",
      ...noArgs,
    };
    await adapter.executeRaw(insert);
    await expect(
      adapter.executeRaw({ ...insert, sql: insert.sql.replace("'a1'", "'a2'") }),
    ).rejects.toMatchObject({ cause: { kind: "UniqueConstraintViolation" } });
    await adapter.dispose();
  });

  test("applique les clés étrangères", async () => {
    const adapter = await connect();
    await expect(
      adapter.executeRaw({ sql: "INSERT INTO tokens VALUES ('t1','inconnu')", ...noArgs }),
    ).rejects.toMatchObject({ cause: { kind: "ForeignKeyConstraintViolation" } });
    await adapter.dispose();
  });

  test("valide une transaction quand le moteur émet COMMIT", async () => {
    const adapter = await connect();
    const tx = await adapter.startTransaction();
    await tx.executeRaw({
      sql: "INSERT INTO accounts VALUES ('a1','a@weave.test',30,NULL,'2026-01-01')",
      ...noArgs,
    });
    await tx.executeRaw({ sql: "COMMIT", ...noArgs });
    await tx.commit();
    const rows = await adapter.queryRaw({ sql: "SELECT id FROM accounts", ...noArgs });
    expect(rows.rows).toEqual([["a1"]]);
    await adapter.dispose();
  });

  test("annule une transaction quand le moteur émet ROLLBACK", async () => {
    const adapter = await connect();
    const tx = await adapter.startTransaction();
    await tx.executeRaw({
      sql: "INSERT INTO accounts VALUES ('a1','a@weave.test',30,NULL,'2026-01-01')",
      ...noArgs,
    });
    await tx.executeRaw({ sql: "ROLLBACK", ...noArgs });
    await tx.rollback();
    const rows = await adapter.queryRaw({ sql: "SELECT id FROM accounts", ...noArgs });
    expect(rows.rows).toEqual([]);
    await adapter.dispose();
  });

  test("refuse un niveau d'isolation non supporté par SQLite", async () => {
    const adapter = await connect();
    await expect(adapter.startTransaction("READ COMMITTED")).rejects.toMatchObject({
      cause: { kind: "InvalidIsolationLevel" },
    });
    await adapter.dispose();
  });

  test("sérialise les transactions concurrentes", async () => {
    const adapter = await connect();
    const first = await adapter.startTransaction();
    let secondStarted = false;
    const pending = adapter.startTransaction().then((tx) => {
      secondStarted = true;
      return tx;
    });
    await Promise.resolve();
    expect(secondStarted).toBe(false);
    await first.executeRaw({ sql: "COMMIT", ...noArgs });
    await first.commit();
    const second = await pending;
    expect(secondStarted).toBe(true);
    await second.executeRaw({ sql: "ROLLBACK", ...noArgs });
    await second.rollback();
    await adapter.dispose();
  });

  test("expose les limites de la connexion", async () => {
    const adapter = await connect();
    expect(adapter.getConnectionInfo?.()).toMatchObject({ supportsRelationJoins: false });
    await adapter.dispose();
  });
});

describe("verrou", () => {
  test("sérialise les acquisitions dans l'ordre", async () => {
    const mutex = new Mutex();
    const order: number[] = [];
    const release1 = await mutex.acquire();
    const p2 = mutex.acquire().then((r) => {
      order.push(2);
      r();
    });
    const p3 = mutex.acquire().then((r) => {
      order.push(3);
      r();
    });
    order.push(1);
    release1();
    await Promise.all([p2, p3]);
    expect(order).toEqual([1, 2, 3]);
  });
});
