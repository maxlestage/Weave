#!/usr/bin/env node
/**
 * Passe-plat vers la CLI Prisma qui choisit automatiquement le bon schéma.
 *
 *   WEAVE_DB=sqlite   → apps/api/prisma/schema.sqlite.prisma  (développement)
 *   WEAVE_DB=postgres → apps/api/prisma/schema.prisma         (défaut, production)
 *
 * Usage : node scripts/prisma.ts <sous-commande prisma...>
 */

import { spawn } from "node:child_process";

const driver = (process.env.WEAVE_DB ?? "postgres").toLowerCase();
const schema = driver === "sqlite" ? "prisma/schema.sqlite.prisma" : "prisma/schema.prisma";

const args = process.argv.slice(2);
if (args.length === 0) {
  console.error("Usage : node scripts/prisma.ts <sous-commande prisma...>");
  process.exit(1);
}

const child = spawn("prisma", [...args, "--schema", schema], {
  stdio: "inherit",
  shell: false,
  env: process.env,
  cwd: process.cwd(),
});

child.on("error", (err) => {
  console.error(`Impossible de lancer la CLI Prisma : ${err.message}`);
  process.exit(1);
});
child.on("exit", (code) => process.exit(code ?? 1));
