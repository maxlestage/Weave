/**
 * Client Prisma, branché sur l'adaptateur de driver correspondant au moteur.
 *
 *   production / préproduction → PostgreSQL via `@prisma/adapter-pg`
 *   développement              → SQLite via `@weave/prisma-bun-sqlite`
 *
 * Prisma 7 compile les requêtes pour un moteur donné : chaque schéma produit
 * donc son propre client (`generated/prisma` et `generated/prisma-sqlite`), et
 * le bon est chargé dynamiquement au démarrage. Les deux schémas étant
 * identiques à l'exception du bloc `datasource`, les types générés sont
 * interchangeables ; ceux de PostgreSQL font foi pour le reste du code.
 */
import { PrismaPg } from "@prisma/adapter-pg";
import { PrismaBunSqlite } from "@weave/prisma-bun-sqlite";
import { env } from "../env.ts";
import type { PrismaClient } from "../generated/prisma/client.ts";

async function createClient(): Promise<PrismaClient> {
  if (env.db.driver === "postgres") {
    const { PrismaClient: PostgresClient } = await import("../generated/prisma/client.ts");
    return new PostgresClient({
      adapter: new PrismaPg({
        connectionString: env.db.postgresUrl!,
        // Voir `env.db.sslInsecure` : nécessaire chez les hébergeurs dont la
        // base présente un certificat auto-signé sur leur réseau interne.
        ...(env.db.sslInsecure ? { ssl: { rejectUnauthorized: false } } : {}),
      }),
      log: env.isProduction ? ["warn", "error"] : ["query", "warn", "error"],
    });
  }

  // `@prisma/adapter-better-sqlite3` repose sur un module natif Node.js qui ne
  // se charge pas sous Bun : Weave utilise son propre adaptateur `bun:sqlite`.
  const { PrismaClient: SqliteClient } = await import("../generated/prisma-sqlite/client.ts");
  return new SqliteClient({
    adapter: new PrismaBunSqlite({ url: env.db.sqliteUrl }),
    log: ["warn", "error"],
  }) as unknown as PrismaClient;
}

export const prisma: PrismaClient = await createClient();

export async function disconnectPrisma(): Promise<void> {
  await prisma.$disconnect();
}
