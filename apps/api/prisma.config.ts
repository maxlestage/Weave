/**
 * Configuration de la CLI Prisma (Prisma 7).
 *
 * Depuis Prisma 7, l'URL de connexion ne figure plus dans le fichier `.prisma`.
 * Ce fichier sélectionne le couple (schéma, URL) selon la variable `WEAVE_DB` :
 *
 *   WEAVE_DB=sqlite   → prisma/schema.sqlite.prisma + DATABASE_URL_SQLITE
 *   WEAVE_DB=postgres → prisma/schema.prisma        + DATABASE_URL   (défaut)
 *
 * SQLite est réservé au développement local ; la production tourne sur PostgreSQL.
 */
import { defineConfig } from "prisma/config";

const useSqlite = (process.env.WEAVE_DB ?? "postgres").toLowerCase() === "sqlite";

const url = useSqlite
  ? (process.env.DATABASE_URL_SQLITE ?? "file:./prisma/dev.db")
  : (process.env.DATABASE_URL ?? "");

if (!useSqlite && url === "") {
  throw new Error(
    "DATABASE_URL est requis pour PostgreSQL. Utilisez WEAVE_DB=sqlite pour le développement local.",
  );
}

export default defineConfig({
  schema: useSqlite ? "prisma/schema.sqlite.prisma" : "prisma/schema.prisma",
  datasource: { url },
  migrations: {
    path: useSqlite ? "prisma/migrations-sqlite" : "prisma/migrations",
    seed: "bun src/seed.ts",
  },
});
