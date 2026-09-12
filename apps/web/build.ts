/*
 * Construction du site vitrine, par Bun seul.
 *
 * Bun lit `index.html`, suit les balises <script> et <link> qu'il y trouve, et
 * bundle React, TypeScript et le CSS d'un seul tenant. C'est ce qui permet de
 * se passer de Vite : le bundler fait déjà le travail, sans configuration.
 *
 * Tailwind v4 est le seul point qui demande un greffon. Sa syntaxe — `@import
 * "tailwindcss"`, `@theme` — n'est pas du CSS standard : sans lui, le bundler
 * la recopierait telle quelle et le site sortirait sans styles.
 *
 * `bunfig.toml` déclare ce greffon pour le serveur de développement. La
 * construction ne lit pas cette section : elle l'appelle donc ici.
 */

import { rm } from "node:fs/promises";
import { gzipSync } from "node:zlib";
import tailwind from "bun-plugin-tailwind";

const racine = new URL(".", import.meta.url).pathname;
const sortie = `${racine}dist`;

await rm(sortie, { recursive: true, force: true });

const resultat = await Bun.build({
  entrypoints: [`${racine}index.html`],
  outdir: sortie,
  plugins: [tailwind],
  target: "browser",
  minify: true,
  // React s'appuie sur cette variable pour retirer ses avertissements de
  // développement et ses contrôles internes. Sans elle, le site embarque la
  // version de développement de React : plus lourde, et plus lente.
  define: { "process.env.NODE_ENV": JSON.stringify("production") },
});

if (!resultat.success) {
  for (const message of resultat.logs) console.error(message);
  process.exit(1);
}

/*
 * Le poids compressé est ce que le visiteur télécharge réellement. Le site est
 * consulté surtout en 4G : l'afficher à chaque construction rend visible toute
 * dérive, au lieu de la découvrir en production.
 */
const tailles = resultat.outputs
  .map((fichier) => ({
    nom: fichier.path.slice(sortie.length + 1),
    brut: fichier.size,
  }))
  .sort((a, b) => b.brut - a.brut);

const ko = (octets: number) => `${(octets / 1024).toFixed(2)} ko`;

for (const { nom, brut } of tailles) {
  const contenu = await Bun.file(`${sortie}/${nom}`).bytes();
  console.log(`  ${nom.padEnd(28)} ${ko(brut).padStart(10)}  gzip ${ko(gzipSync(contenu).length)}`);
}
