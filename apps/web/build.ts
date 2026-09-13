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
await poserLesBalisesDeLAccueil();
await ecrirePagesJuridiques(resultat);
await ecrireFichiersDeReferencement();
await verifierQu_AucuneAdresseN_EstEcriteEnDur();
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
  // Chacune est demandée par une adresse fixe, écrite ailleurs que dans nos
  // pages : un réseau social récupère `partage.png` depuis l'extérieur, iOS
  // demande `apple-touch-icon.png` à la racine, et un navigateur ou un robot
  // demande `favicon.svg` sans avoir lu la moindre balise. Aucune ne doit donc
  // porter d'empreinte — l'adresse partagée serait périmée à la construction
  // suivante.
  //
  // L'accueil référence par ailleurs l'icône empreinte que produit le
  // bundler : les deux coexistent, l'une pour qui lit la page, l'autre pour
  // qui devine l'adresse.
  for (const fichier of [
    "partage.png",
    "apple-touch-icon.png",
    "favicon.svg",
    "site.webmanifest",
  ]) {
    await Bun.write(`${sortie}/${fichier}`, Bun.file(`${racine}public/${fichier}`));
  }
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
    <link rel="canonical" href="${ORIGINE ? `${ORIGINE}/${doc.slug}` : `/${doc.slug}`}" />
    <meta property="og:site_name" content="Weave" />
    <meta property="og:title" content="${doc.titre} — Weave" />
    <meta property="og:description" content="${doc.description}" />
    <meta property="og:type" content="article" />
    <meta property="og:locale" content="fr_FR" />${balisesAbsolues(doc.slug)}
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
 * Pose sur la page d'accueil les balises qui demandent une adresse absolue.
 *
 * L'accueil est produit par le bundler à partir d'`index.html`, qui ne peut
 * rien calculer : ses `canonical`, `og:url` et `og:image` valaient donc
 * « https://weave.app » ÉCRITS EN DUR — un domaine qui n'est pas le nôtre.
 *
 * C'est la page que l'on partage. Le lien aurait montré l'image d'un autre
 * site, ou aucune, et `canonical` aurait désigné ce domaine comme la véritable
 * adresse de nos pages — de quoi remettre à quelqu'un d'autre le référencement
 * du site. Les pages juridiques, elles, lisaient déjà l'origine : seule
 * l'accueil ne le faisait pas, et c'était la seule qui comptait pour cela.
 */
async function poserLesBalisesDeLAccueil() {
  const chemin = `${sortie}/index.html`;
  const html = await Bun.file(chemin).text();

  const canonique = ORIGINE
    ? `    <link rel="canonical" href="${ORIGINE}/" />`
    : // Sans origine, pas de canonique : une adresse relative « / » sur la
      // page d'accueil ne dit rien de plus que la page elle-même, et une
      // adresse inventée dirait quelque chose de faux.
      "";

  // L'icône de l'écran d'accueil et le manifeste sont posés ici, et non dans
  // `index.html`, parce que le bundler suit les `<link>` qu'il y trouve : il
  // tenterait de résoudre ces deux fichiers comme des entrées de construction,
  // et échouerait. Ils sont copiés tels quels, à une adresse fixe.
  //
  // Sans `apple-touch-icon`, « Ajouter à l'écran d'accueil » pose une capture
  // de la page en guise d'icône. Sans manifeste, le raccourci porte le titre
  // complet de la page plutôt que « Weave ».
  const icones = [
    `    <link rel="apple-touch-icon" href="/apple-touch-icon.png" />`,
    `    <link rel="manifest" href="/site.webmanifest" />`,
  ];

  const pose = [canonique, balisesAbsolues("").trimStart(), ...icones].filter(Boolean).join("\n");

  const marque = '<meta property="og:site_name" content="Weave" />';
  if (!html.includes(marque)) {
    throw new Error("Accueil : ancrage des balises d'adresse introuvable.");
  }
  await Bun.write(chemin, html.replace(marque, `${pose}\n    ${marque}`));
}

/*
 * Les balises qui n'ont de sens qu'absolues : `og:url` et `og:image`.
 *
 * Le protocole Open Graph impose des URL absolues — une adresse relative n'y
 * est pas seulement mal vue, elle n'est pas résolue. Sans origine connue, ces
 * balises sont donc omises : un partage sans vignette vaut mieux qu'un
 * partage attribué à un domaine qui ne répond pas.
 *
 * L'URL canonique, elle, reste déclarée : le format relatif y est valide et se
 * résout contre l'adresse de la page.
 */
function balisesAbsolues(slug: string): string {
  if (!ORIGINE) return "";
  return [
    ``,
    `    <meta property="og:url" content="${ORIGINE}/${slug}" />`,
    `    <meta property="og:image" content="${ORIGINE}/partage.png" />`,
    `    <meta name="twitter:card" content="summary_large_image" />`,
  ].join("\n");
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

  // `robots.txt` est toujours écrit : il autorise l'exploration, et c'est le
  // premier fichier qu'un moteur demande. Mais la ligne `Sitemap:` exige une
  // adresse absolue — sans origine, elle est omise plutôt que de renvoyer le
  // moteur vers un hôte qui ne répond pas. Le plan reste trouvable à la
  // racine, où les moteurs le cherchent d'eux-mêmes.
  await Bun.write(
    `${sortie}/robots.txt`,
    [
      `User-agent: *`,
      `Allow: /`,
      ...(ORIGINE ? [``, `Sitemap: ${ORIGINE}/sitemap.xml`] : []),
      ``,
    ].join("\n"),
  );

  // Le plan du site, lui, n'est écrit que si l'origine est connue : la balise
  // `<loc>` n'accepte que des URL absolues. Un plan qui n'énumère que des
  // adresses mortes ne fait pas indexer les pages, il les fait retirer.
  if (!ORIGINE) {
    console.warn(
      "  ↳ pas de sitemap.xml : SITE.origine n'est pas renseignée, et un plan\n" +
        "    du site n'accepte que des adresses absolues.",
    );
    return;
  }

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
/*
 * Refuse la construction si une page nomme notre propre adresse en dur.
 *
 * L'accueil portait « https://weave.app » dans `canonical`, `og:url` et
 * `og:image` — un domaine qui n'est pas le nôtre. Un lien partagé aurait
 * montré l'image d'un autre site, et `canonical` aurait désigné ce domaine
 * comme l'adresse véritable de nos pages : de quoi remettre à quelqu'un
 * d'autre le référencement du site.
 *
 * C'était invisible. Les pages juridiques lisaient l'origine depuis le début,
 * et seule l'accueil ne le faisait pas — c'est-à-dire la seule page que l'on
 * partage. Rien ne l'aurait signalé avant qu'on colle le lien à quelqu'un.
 *
 * Le contrôle ne porte QUE sur les balises dont le rôle est de nommer notre
 * adresse. Les liens sortants d'une page — la CNIL, l'assistance d'Apple —
 * sont du contenu : ils désignent autrui, et c'est bien ce qu'on leur demande.
 */
async function verifierQu_AucuneAdresseN_EstEcriteEnDur() {
  const pages = [
    "index.html",
    ...(await import("./src/pages/documents.ts")).DOCUMENTS.map((d) => `${d.slug}/index.html`),
  ];

  // `rel="canonical"`, puis les propriétés Open Graph et Twitter qui portent
  // une adresse. Chacune répond à la question « où cette page vit-elle ? ».
  const balises =
    /<link[^>]+rel="canonical"[^>]+href="([^"]+)"|<meta[^>]+(?:property|name)="(?:og:url|og:image|twitter:image)"[^>]+content="([^"]+)"/g;

  const fautes: string[] = [];
  for (const page of pages) {
    const html = await Bun.file(`${sortie}/${page}`).text();
    for (const [, href, content] of html.matchAll(balises)) {
      const adresse = href ?? content ?? "";
      if (!adresse.startsWith("http")) continue;
      if (ORIGINE && adresse.startsWith(ORIGINE)) continue;
      fautes.push(`${page} : ${adresse}`);
    }
  }

  if (fautes.length > 0) {
    console.error("");
    console.error("  ✗ Ces balises nomment notre adresse sans passer par SITE.origine :");
    for (const faute of fautes) console.error(`      ${faute}`);
    console.error("    Une adresse partagée se déduit de l'origine, elle ne s'écrit pas.");
    process.exit(1);
  }
}

function avertirDesMentionsIncompletes() {
  const manquantes = valeursManquantes();
  if (manquantes.length === 0) return;
  console.log("");
  console.log(`  ⚠ ${manquantes.length} valeurs restent à renseigner`);
  console.log("    dans apps/web/src/pages/identite.ts :");
  for (const champ of manquantes) console.log(`      • ${champ}`);
  console.log(
    "    Les mentions légales manquantes apparaissent surlignées\n" + "    sur les pages publiées.",
  );

  // `SITE.origine` ne se contente pas de manquer à une mention : sans elle,
  // aucune adresse absolue n'est écrite, et le protocole Open Graph n'en
  // résout aucune de relative. Un lien partagé sort donc sans vignette, sans
  // titre et sans description — une ligne de texte nue. Cela se voit sur la
  // page publiée, mais pas ici, et c'est le genre de chose qu'on découvre en
  // collant le lien à quelqu'un.
  if (!ORIGINE) {
    console.log("");
    console.log("  ⚠ SITE.origine n'est pas posée : les liens partagés sortiront nus.");
    console.log("    Pas de vignette, pas de titre, pas de description — le protocole");
    console.log("    Open Graph ne résout aucune adresse relative. Ni plan du site.");
    console.log("    Posez SITE_ORIGINE à la construction, par exemple :");
    console.log("      SITE_ORIGINE=https://votre-domaine.fr bun run build");
  }
}
