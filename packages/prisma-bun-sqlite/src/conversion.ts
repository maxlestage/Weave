/**
 * Conversion des valeurs entre `bun:sqlite` et le moteur de requêtes Prisma.
 *
 * Les règles reproduisent la sémantique attendue par Prisma 7 : types de
 * colonnes déduits du type déclaré (puis inférés à partir des valeurs quand la
 * colonne est calculée), dates au format ISO 8601, entiers 64 bits préservés.
 */
import { ColumnTypeEnum, type ArgType, type ColumnType } from "@prisma/driver-adapter-utils";

export type TimestampFormat = "iso8601" | "unixepoch-ms";

/** Type déclaré d'une colonne SQLite → type de colonne Prisma. */
export function mapDeclaredType(declared: string | null): ColumnType | null {
  if (declared === null) return null;
  switch (declared.toUpperCase()) {
    case "":
      return null;
    case "DECIMAL":
      return ColumnTypeEnum.Numeric;
    case "FLOAT":
      return ColumnTypeEnum.Float;
    case "DOUBLE":
    case "DOUBLE PRECISION":
    case "NUMERIC":
    case "REAL":
      return ColumnTypeEnum.Double;
    case "TINYINT":
    case "SMALLINT":
    case "MEDIUMINT":
    case "INT":
    case "INTEGER":
    case "SERIAL":
    case "INT2":
      return ColumnTypeEnum.Int32;
    case "BIGINT":
    case "UNSIGNED BIG INT":
    case "INT8":
      return ColumnTypeEnum.Int64;
    case "DATETIME":
    case "TIMESTAMP":
      return ColumnTypeEnum.DateTime;
    case "TIME":
      return ColumnTypeEnum.Time;
    case "DATE":
      return ColumnTypeEnum.Date;
    case "TEXT":
    case "CLOB":
    case "CHARACTER":
    case "VARCHAR":
    case "VARYING CHARACTER":
    case "NCHAR":
    case "NATIVE CHARACTER":
    case "NVARCHAR":
      return ColumnTypeEnum.Text;
    case "BLOB":
      return ColumnTypeEnum.Bytes;
    case "BOOLEAN":
      return ColumnTypeEnum.Boolean;
    case "JSONB":
      return ColumnTypeEnum.Json;
    default:
      return null;
  }
}

function inferFromValue(value: unknown): ColumnType {
  switch (typeof value) {
    case "string":
      return ColumnTypeEnum.Text;
    case "bigint":
      return ColumnTypeEnum.Int64;
    case "boolean":
      return ColumnTypeEnum.Boolean;
    case "number":
      return ColumnTypeEnum.UnknownNumber;
    case "object":
      if (value instanceof Uint8Array || value instanceof ArrayBuffer) return ColumnTypeEnum.Bytes;
      break;
  }
  throw new TypeError(`Valeur de type inattendu retournée par bun:sqlite : ${typeof value}`);
}

/**
 * Types de colonnes de la réponse. Les colonnes sans type déclaré (expressions,
 * agrégats) sont inférées à partir de la première valeur non nulle rencontrée.
 */
export function resolveColumnTypes(
  declaredTypes: readonly (string | null)[],
  rows: readonly unknown[][],
): ColumnType[] {
  const resolved: ColumnType[] = [];
  const pending = new Set<number>();

  declaredTypes.forEach((declared, index) => {
    const mapped = mapDeclaredType(declared);
    if (mapped === null) {
      pending.add(index);
      resolved[index] = ColumnTypeEnum.Int32; // valeur de repli, écrasée ci-dessous
    } else {
      resolved[index] = mapped;
    }
  });

  for (const index of pending) {
    for (const row of rows) {
      const candidate = row[index];
      if (candidate !== null && candidate !== undefined) {
        resolved[index] = inferFromValue(candidate);
        break;
      }
    }
  }

  return resolved;
}

/** Normalise une ligne brute vers les valeurs attendues par le moteur Prisma. */
export function mapRow(row: readonly unknown[], columnTypes: readonly ColumnType[]): unknown[] {
  const out: unknown[] = new Array(row.length);
  for (let i = 0; i < row.length; i++) {
    const value = row[i];
    const type = columnTypes[i];

    if (
      typeof value === "number" &&
      (type === ColumnTypeEnum.Int32 || type === ColumnTypeEnum.Int64) &&
      !Number.isInteger(value)
    ) {
      out[i] = Math.trunc(value);
      continue;
    }
    if ((typeof value === "number" || typeof value === "bigint") && type === ColumnTypeEnum.DateTime) {
      out[i] = new Date(Number(value)).toISOString();
      continue;
    }
    if (typeof value === "bigint") {
      const asNumber = Number(value);
      out[i] = Number.isSafeInteger(asNumber) ? asNumber : value.toString();
      continue;
    }
    out[i] = value;
  }
  return out;
}

/** Normalise un paramètre Prisma vers une valeur liable par bun:sqlite. */
export function mapArg(
  arg: unknown,
  argType: ArgType | undefined,
  timestampFormat: TimestampFormat,
): unknown {
  if (arg === null || arg === undefined) return null;

  const scalar = argType?.scalarType;

  if (typeof arg === "string") {
    if (scalar === "int") return Number.parseInt(arg, 10);
    if (scalar === "float" || scalar === "decimal") return Number.parseFloat(arg);
    if (scalar === "bigint") return BigInt(arg);
    if (scalar === "bytes") return Buffer.from(arg, "base64");
    if (scalar === "datetime") arg = new Date(arg);
  }

  if (typeof arg === "boolean") return arg ? 1 : 0;

  if (arg instanceof Date) {
    return timestampFormat === "unixepoch-ms"
      ? arg.getTime()
      : arg.toISOString().replace("Z", "+00:00");
  }

  if (arg instanceof ArrayBuffer) return new Uint8Array(arg);

  return arg;
}
