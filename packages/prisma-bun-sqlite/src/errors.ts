/** Traduction des erreurs `bun:sqlite` vers les erreurs typées de Prisma. */
import { DriverAdapterError, type Error as PrismaDriverError } from "@prisma/driver-adapter-utils";

const SQLITE_BUSY = 5;
const PRIMARY_ERROR_CODE_MASK = 0xff;

interface BunSqliteError extends Error {
  code?: string;
  errno?: number;
}

function isSqliteError(error: unknown): error is BunSqliteError {
  return (
    error instanceof Error &&
    typeof (error as BunSqliteError).code === "string" &&
    (error as BunSqliteError).code!.startsWith("SQLITE_")
  );
}

/** Extrait « table.colonne, table.colonne » du message d'une violation de contrainte. */
function constraintFields(message: string): { fields?: string[]; table?: string } {
  const columns = message.split("constraint failed: ").at(1)?.split(", ");
  if (columns === undefined) return {};
  const fields = columns.map((c) => c.split(".").pop()!).filter(Boolean);
  const table = columns.at(0)?.split(".").slice(0, -1).join(".") || undefined;
  return { fields: fields.length > 0 ? fields : undefined, table };
}

function map(error: BunSqliteError): PrismaDriverError {
  const message = error.message;
  switch (error.code) {
    case "SQLITE_CONSTRAINT_UNIQUE":
    case "SQLITE_CONSTRAINT_PRIMARYKEY": {
      const { fields, table } = constraintFields(message);
      return {
        kind: "UniqueConstraintViolation",
        ...(fields ? { constraint: { fields } } : {}),
        ...(table ? { table } : {}),
      };
    }
    case "SQLITE_CONSTRAINT_NOTNULL": {
      const { fields } = constraintFields(message);
      return {
        kind: "NullConstraintViolation",
        ...(fields ? { constraint: { fields } } : {}),
      };
    }
    case "SQLITE_CONSTRAINT_FOREIGNKEY":
    case "SQLITE_CONSTRAINT_TRIGGER":
      return { kind: "ForeignKeyConstraintViolation", constraint: { foreignKey: {} } };
    default: {
      const extended = error.errno;
      if (
        error.code?.startsWith("SQLITE_BUSY") ||
        (extended !== undefined && (extended & PRIMARY_ERROR_CODE_MASK) === SQLITE_BUSY)
      ) {
        return { kind: "SocketTimeout" };
      }
      return { kind: "sqlite", extendedCode: extended ?? 1, message };
    }
  }
}

/** Convertit et relance une erreur bun:sqlite sous la forme attendue par Prisma. */
export function throwAsDriverError(error: unknown): never {
  if (isSqliteError(error)) {
    throw new DriverAdapterError({
      originalCode: error.code,
      originalMessage: error.message,
      ...map(error),
    });
  }
  throw error;
}
