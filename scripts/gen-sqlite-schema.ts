#!/usr/bin/env bun
/**
 * Dérive `prisma/schema.sqlite.prisma` à partir du schéma PostgreSQL.
 *
 * Le schéma de Weave est écrit de façon portable : aucun `enum`, aucune liste
 * scalaire, aucun type natif propre à un moteur. La seule différence entre les
 * deux cibles est donc le bloc `datasource` et le répertoire de sortie du
 * client généré. Ce script fait cette substitution — et échoue bruyamment si
 * une construction non portable est introduite dans le schéma source.
 */

const SOURCE = new URL("../apps/api/prisma/schema.prisma", import.meta.url).pathname;
const TARGET = new URL("../apps/api/prisma/schema.sqlite.prisma", import.meta.url).pathname;

const source = await Bun.file(SOURCE).text();

/* --- Garde-fous de portabilité ------------------------------------ */

const violations: string[] = [];
if (/^\s*enum\s+\w+\s*\{/m.test(source)) {
  violations.push("`enum` détecté : SQLite ne le supporte pas. Utilisez un String documenté.");
}
if (/@db\.\w+/.test(source)) {
  violations.push("Type natif `@db.*` détecté : il n'est pas portable vers SQLite.");
}
if (/^\s*\w+\s+(String|Int|Float|Boolean|DateTime|Json)\[\]/m.test(source)) {
  violations.push("Liste scalaire détectée : non supportée par SQLite. Encodez en JSON (String).");
}
if (/^\s*\w+\s+Json(\s|\?)/m.test(source)) {
  violations.push("Champ `Json` détecté : préférez un String encodé pour rester portable.");
}
if (violations.length > 0) {
  console.error("Schéma non portable :\n" + violations.map((v) => `  • ${v}`).join("\n"));
  process.exit(1);
}

/* --- Substitution du datasource et de la sortie du client ---------- */

const header = `// FICHIER GÉNÉRÉ — NE PAS MODIFIER À LA MAIN.
// Dérivé de prisma/schema.prisma par \`bun run db:sqlite\`.
// Cible : SQLite, développement local uniquement. La production tourne sur PostgreSQL.

`;

let out = source
  .replace(
    /datasource\s+db\s*\{[\s\S]*?\n\}/,
    `datasource db {\n  provider = "sqlite"\n}`,
  )
  .replace(/output\s*=\s*"\.\.\/src\/generated\/prisma"/, `output   = "../src/generated/prisma-sqlite"`);

// Retire l'en-tête de documentation du fichier source (remplacé par le nôtre).
out = out.replace(/^\/\/[^\n]*\n(\/\/[^\n]*\n|\n)*/, "");

await Bun.write(TARGET, header + out);
console.log(`Schéma SQLite dérivé → ${TARGET.replace(process.cwd() + "/", "")}`);
