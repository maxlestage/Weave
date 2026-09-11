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
  : process.env.DATABASE_URL;

// `datasource` n'est renseigné que si une URL est connue. `prisma generate`
// n'a pas besoin de joindre une base : l'exiger ici casserait l'intégration
// continue et la construction de l'image, où aucune base n'existe encore.
// Les commandes qui en ont réellement besoin (`migrate`, `db pull`) échouent
// d'elles-mêmes, avec le message de Prisma.
// Base fantôme, utilisée par `migrate diff --from-migrations` et `migrate dev`
// pour rejouer les migrations à blanc. SQLite en crée une en mémoire tout seul ;
// PostgreSQL demande une base réelle.
const shadowDatabaseUrl = process.env.SHADOW_DATABASE_URL;

export default defineConfig({
  schema: useSqlite ? "prisma/schema.sqlite.prisma" : "prisma/schema.prisma",
  ...(url ? { datasource: { url, ...(shadowDatabaseUrl ? { shadowDatabaseUrl } : {}) } } : {}),
  migrations: {
    path: useSqlite ? "prisma/migrations-sqlite" : "prisma/migrations",
    seed: "node src/seed.ts",
  },
});
