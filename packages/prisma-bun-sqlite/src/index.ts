/**
 * Adaptateur de driver Prisma 7 pour `bun:sqlite`.
 *
 * Pourquoi il existe : l'adaptateur officiel `@prisma/adapter-better-sqlite3`
 * repose sur un module natif Node.js qui ne se charge pas sous Bun
 * (`ERR_DLOPEN_FAILED`). Weave ayant fait le choix d'un runtime Bun de bout en
 * bout, cet adaptateur branche Prisma directement sur le pilote SQLite intégré
 * à Bun. Il n'est utilisé qu'en développement : la production tourne sur
 * PostgreSQL via `@prisma/adapter-pg`.
 *
 * Il implémente le contrat public `SqlMigrationAwareDriverAdapterFactory` de
 * `@prisma/driver-adapter-utils` et reproduit la sémantique de conversion de
 * l'adaptateur officiel (types de colonnes, dates ISO 8601, entiers 64 bits).
 */
import { Database } from "bun:sqlite";
import type { Statement } from "bun:sqlite";
import type {
  ConnectionInfo,
  IsolationLevel,
  SqlDriverAdapter,
  SqlMigrationAwareDriverAdapterFactory,
  SqlQuery,
  SqlQueryable,
  SqlResultSet,
  Transaction,
  TransactionOptions,
} from "@prisma/driver-adapter-utils";
import { DriverAdapterError } from "@prisma/driver-adapter-utils";
import { mapArg, mapRow, resolveColumnTypes, type TimestampFormat } from "./conversion.ts";
import { throwAsDriverError } from "./errors.ts";
import { Mutex } from "./mutex.ts";

export interface PrismaBunSqliteConfig {
  /** `:memory:`, un chemin de fichier, ou une URL `file:`. */
  url: string;
  /** Ouvre la base en lecture seule. */
  readonly?: boolean;
  /** Crée le fichier s'il n'existe pas (vrai par défaut). */
  create?: boolean;
}

export interface PrismaBunSqliteOptions {
  /** Base fantôme utilisée par Prisma Migrate. `:memory:` par défaut. */
  shadowDatabaseUrl?: string;
  /** Format d'écriture des dates. `iso8601` par défaut, comme l'adaptateur officiel. */
  timestampFormat?: TimestampFormat;
  /** Active `PRAGMA foreign_keys` (vrai par défaut). */
  foreignKeys?: boolean;
  /** Active le journal WAL, recommandé en développement (vrai par défaut). */
  wal?: boolean;
}

const ADAPTER_NAME = "@weave/prisma-bun-sqlite";

/** SQLite moderne accepte 32 766 paramètres liés par requête. */
const MAX_BIND_VALUES = 32_766;

function openDatabase(config: PrismaBunSqliteConfig, options: PrismaBunSqliteOptions): Database {
  const path = config.url === ":memory:" ? ":memory:" : config.url.replace(/^file:/, "");
  const db = new Database(path, {
    readonly: config.readonly ?? false,
    create: config.create ?? true,
    strict: false,
  });
  if (options.foreignKeys !== false) db.run("PRAGMA foreign_keys = ON");
  if (options.wal !== false && path !== ":memory:") db.run("PRAGMA journal_mode = WAL");
  db.run("PRAGMA busy_timeout = 5000");
  return db;
}

abstract class BunSqliteQueryable implements SqlQueryable {
  readonly provider = "sqlite" as const;
  readonly adapterName = ADAPTER_NAME;

  constructor(
    protected readonly db: Database,
    protected readonly timestampFormat: TimestampFormat,
  ) {}

  async queryRaw(query: SqlQuery): Promise<SqlResultSet> {
    const stmt = this.#prepare(query);
    try {
      const rows = stmt.values(...this.#bind(query)) as unknown[][];
      const declaredTypes = stmt.declaredTypes as (string | null)[];
      const columnTypes = resolveColumnTypes(declaredTypes, rows);
      return {
        columnNames: stmt.columnNames,
        columnTypes,
        rows: rows.map((row) => mapRow(row, columnTypes)),
      };
    } catch (error) {
      throwAsDriverError(error);
    }
  }

  async executeRaw(query: SqlQuery): Promise<number> {
    const stmt = this.#prepare(query);
    try {
      const result = stmt.run(...this.#bind(query));
      return Number(result.changes);
    } catch (error) {
      throwAsDriverError(error);
    }
  }

  /**
   * `db.query` conserve les instructions préparées en cache, ce qui évite de
   * recompiler le même SQL à chaque requête du moteur Prisma.
   */
  #prepare(query: SqlQuery): Statement {
    try {
      const stmt = this.db.query(query.sql);
      // Les entiers reviennent en BigInt pour préserver la précision 64 bits ;
      // `mapRow` les ramène en `number` lorsque c'est sûr. `safeIntegers` est
      // exposé par le prototype de `Statement` mais absent de ses types.
      (stmt as unknown as { safeIntegers(enabled: boolean): void }).safeIntegers(true);
      return stmt;
    } catch (error) {
      throwAsDriverError(error);
    }
  }

  #bind(query: SqlQuery): [] | [unknown[]] {
    if (query.args.length === 0) return [];
    return [query.args.map((arg, i) => mapArg(arg, query.argTypes[i], this.timestampFormat))];
  }
}

class BunSqliteTransaction extends BunSqliteQueryable implements Transaction {
  constructor(
    db: Database,
    timestampFormat: TimestampFormat,
    readonly options: TransactionOptions,
    private readonly release: () => void,
  ) {
    super(db, timestampFormat);
  }

  /**
   * Avec `usePhantomQuery: false`, c'est le moteur de requêtes Prisma qui émet
   * lui-même le `COMMIT` via `executeRaw`. L'adaptateur n'a donc plus qu'à
   * relâcher le verrou de sérialisation des transactions.
   */
  async commit(): Promise<void> {
    this.release();
  }

  /** Idem pour `ROLLBACK` : émis par le moteur, le verrou est simplement relâché. */
  async rollback(): Promise<void> {
    this.release();
  }

  async createSavepoint(name: string): Promise<void> {
    await this.executeRaw({ sql: `SAVEPOINT ${name}`, args: [], argTypes: [] });
  }

  async rollbackToSavepoint(name: string): Promise<void> {
    await this.executeRaw({ sql: `ROLLBACK TO ${name}`, args: [], argTypes: [] });
  }

  async releaseSavepoint(name: string): Promise<void> {
    await this.executeRaw({ sql: `RELEASE SAVEPOINT ${name}`, args: [], argTypes: [] });
  }
}

class BunSqliteAdapter extends BunSqliteQueryable implements SqlDriverAdapter {
  #mutex = new Mutex();

  async executeScript(script: string): Promise<void> {
    try {
      this.db.exec(script);
    } catch (error) {
      throwAsDriverError(error);
    }
  }

  async startTransaction(isolationLevel?: IsolationLevel): Promise<Transaction> {
    if (isolationLevel !== undefined && isolationLevel !== "SERIALIZABLE") {
      throw new DriverAdapterError({ kind: "InvalidIsolationLevel", level: isolationLevel });
    }
    const release = await this.#mutex.acquire();
    try {
      this.db.run("BEGIN");
    } catch (error) {
      release();
      throwAsDriverError(error);
    }
    return new BunSqliteTransaction(
      this.db,
      this.timestampFormat,
      { usePhantomQuery: false },
      release,
    );
  }

  getConnectionInfo(): ConnectionInfo {
    return { maxBindValues: MAX_BIND_VALUES, supportsRelationJoins: false };
  }

  async dispose(): Promise<void> {
    this.db.close(false);
  }
}

/**
 * Fabrique d'adaptateur, à passer au constructeur `PrismaClient` :
 *
 * ```ts
 * const adapter = new PrismaBunSqlite({ url: "file:./prisma/dev.db" });
 * const prisma = new PrismaClient({ adapter });
 * ```
 */
export class PrismaBunSqlite implements SqlMigrationAwareDriverAdapterFactory {
  readonly provider = "sqlite" as const;
  readonly adapterName = ADAPTER_NAME;

  readonly #config: PrismaBunSqliteConfig;
  readonly #options: PrismaBunSqliteOptions;

  constructor(config: PrismaBunSqliteConfig, options: PrismaBunSqliteOptions = {}) {
    this.#config = config;
    this.#options = options;
  }

  async connect(): Promise<SqlDriverAdapter> {
    return new BunSqliteAdapter(
      openDatabase(this.#config, this.#options),
      this.#options.timestampFormat ?? "iso8601",
    );
  }

  async connectToShadowDb(): Promise<SqlDriverAdapter> {
    const url = this.#options.shadowDatabaseUrl ?? ":memory:";
    return new BunSqliteAdapter(
      openDatabase({ ...this.#config, url }, this.#options),
      this.#options.timestampFormat ?? "iso8601",
    );
  }
}

export type { TimestampFormat };
