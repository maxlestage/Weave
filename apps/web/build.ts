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

import type { ReactElement } from "react";
import { rm } from "node:fs/promises";
import { ORIGINE, valeursManquantes } from "./src/pages/identite.ts";
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

await copierRessourcesPubliques();
await ecrirePagesJuridiques(resultat);
await ecrireFichiersDeReferencement();
avertirDesMentionsIncompletes();

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

/*
 * L'image de partage, telle quelle.
 *
 * Elle est désignée par une adresse absolue dans les balises `og:image` — un
 * réseau social la récupère depuis l'extérieur, il ne suit pas les chemins
 * relatifs du site. Le bundler ne la voit donc pas, et elle ne doit surtout pas
 * porter d'empreinte : l'adresse partagée serait périmée à chaque construction.
 */
async function copierRessourcesPubliques() {
  await Bun.write(`${sortie}/partage.png`, Bun.file(`${racine}public/partage.png`));
}

/*
 * Les pages juridiques, rendues en HTML une fois pour toutes.
 *
 * Elles sont écrites en React pour partager l'identité du site, mais ce sont
 * des documents : rien n'y bouge. Les livrer comme points d'entrée de Bun
 * embarquerait React dans chacune — six paquets de 220 ko pour afficher du
 * texte. Rendues ici, elles ne coûtent que leur balisage et réutilisent la
 * feuille de style déjà produite pour l'accueil.
 *
 * Conséquence utile : elles s'affichent sans JavaScript, donc aussi pour un
 * robot d'indexation ou un navigateur qui l'a désactivé.
 */
async function ecrirePagesJuridiques(construction: Awaited<ReturnType<typeof Bun.build>>) {
  const { renderToStaticMarkup } = await import("react-dom/server");
  const { DOCUMENTS } = await import("./src/pages/documents.ts");
  const { Cgu } = await import("./src/pages/Cgu.tsx");
  const { Cgv } = await import("./src/pages/Cgv.tsx");
  const { Confidentialite } = await import("./src/pages/Confidentialite.tsx");
  const { MentionsLegales } = await import("./src/pages/MentionsLegales.tsx");
  const { SuppressionCompte } = await import("./src/pages/SuppressionCompte.tsx");

  // La jonction slug → composant vit ici, pas dans `documents.ts` : c'est ce
  // qui garde les pages juridiques hors du paquet de l'accueil.
  const { createElement } = await import("react");
  const COMPOSANTS: Record<string, () => ReactElement> = {
    confidentialite: Confidentialite,
    cgu: Cgu,
    cgv: Cgv,
    "suppression-compte": SuppressionCompte,
    "mentions-legales": MentionsLegales,
  };

  // La feuille de style porte une empreinte : on la retrouve dans la sortie
  // plutôt que de la deviner, sinon un changement de nom casserait le style
  // sans casser la construction.
  const style = construction.outputs.find((f) => f.path.endsWith(".css"));
  const icone = construction.outputs.find((f) => f.path.endsWith(".svg"));
  if (!style) throw new Error("Feuille de style introuvable dans la construction.");

  const nom = (chemin: string) => chemin.slice(sortie.length + 1);

  for (const doc of DOCUMENTS) {
    const Composant = COMPOSANTS[doc.slug];
    if (!Composant) throw new Error(`Aucun composant pour la page « ${doc.slug} ».`);
    const corps = renderToStaticMarkup(createElement(Composant));
    const html = `<!doctype html>
<html lang="fr">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover" />
    <meta name="theme-color" content="#16121f" media="(prefers-color-scheme: dark)" />
    <meta name="theme-color" content="#fffdf9" media="(prefers-color-scheme: light)" />
    <title>${doc.titre} — Weave</title>
    <meta name="description" content="${doc.description}" />
    <link rel="canonical" href="${ORIGINE}/${doc.slug}" />
    <meta property="og:site_name" content="Weave" />
    <meta property="og:title" content="${doc.titre} — Weave" />
    <meta property="og:description" content="${doc.description}" />
    <meta property="og:type" content="article" />
    <meta property="og:locale" content="fr_FR" />
    <meta property="og:url" content="${ORIGINE}/${doc.slug}" />
    <meta property="og:image" content="${ORIGINE}/partage.png" />
    <meta name="twitter:card" content="summary_large_image" />
    <link rel="icon" href="/${icone ? nom(icone.path) : "favicon.svg"}" type="image/svg+xml" />
    <link rel="stylesheet" href="/${nom(style.path)}" />
  </head>
  <body>
    <div id="racine">${corps}</div>
  </body>
</html>
`;
    await Bun.write(`${sortie}/${doc.slug}/index.html`, html);
  }
}

/*
 * `robots.txt` et `sitemap.xml`.
 *
 * Sans eux, rien n'indique aux moteurs quelles pages existent : le site n'a
 * pas de liens entrants et ses pages juridiques ne sont atteignables que
 * depuis le pied de page.
 */
async function ecrireFichiersDeReferencement() {
  const { DOCUMENTS } = await import("./src/pages/documents.ts");
  const adresses = ["", ...DOCUMENTS.map((doc) => doc.slug)];
  const jour = new Date().toISOString().slice(0, 10);

  await Bun.write(
    `${sortie}/robots.txt`,
    [`User-agent: *`, `Allow: /`, ``, `Sitemap: ${ORIGINE}/sitemap.xml`, ``].join("\n"),
  );

  await Bun.write(
    `${sortie}/sitemap.xml`,
    [
      `<?xml version="1.0" encoding="UTF-8"?>`,
      `<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">`,
      ...adresses.map((chemin) =>
        [
          `  <url>`,
          `    <loc>${ORIGINE}/${chemin}</loc>`,
          `    <lastmod>${jour}</lastmod>`,
          `  </url>`,
        ].join("\n"),
      ),
      `</urlset>`,
      ``,
    ].join("\n"),
  );
}

/*
 * Ce qu'il reste à renseigner dans les mentions légales.
 *
 * La construction ne s'interrompt pas : le site doit rester constructible tant
 * que la société n'est pas immatriculée. Mais elle le dit à chaque fois, parce
 * qu'un gabarit oublié se publie tout seul — et qu'une mention légale
 * incomplète est pénalement sanctionnée.
 */
function avertirDesMentionsIncompletes() {
  const manquantes = valeursManquantes();
  if (manquantes.length === 0) return;
  console.log("");
  console.log(`  ⚠ ${manquantes.length} mentions légales restent à renseigner`);
  console.log("    dans apps/web/src/pages/identite.ts :");
  for (const champ of manquantes) console.log(`      • ${champ}`);
  console.log("    Elles apparaissent surlignées sur les pages publiées.");
}
