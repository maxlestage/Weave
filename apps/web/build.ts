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
await ecrireLesAccueils(resultat);
await ecrirePagesJuridiques(resultat);
await ecrireFichiersDeReferencement();
await verifierQu_AucuneAdresseN_EstEcriteEnDur();
await verifierQueLesFilsSeLisent();
verifierQueLOrigineEstHabitable();
avertirDesMentionsIncompletes();

/*
 * Le poids compressé est ce que le visiteur télécharge réellement. Le site est
 * consulté surtout en 4G : l'afficher à chaque construction rend visible toute
 * dérive, au lieu de la découvrir en production.
 */
const { LANGUES: LANGUES_LIVREES, chemin: cheminDeLangue } = await import("./src/langues.ts");

// Les fichiers sont MESURÉS SUR LE DISQUE, et non repris de `resultat.outputs`.
// La construction réécrit `index.html` après le bundler — et en produit deux
// autres, un par langue, que le bundler n'a jamais vus. La taille annoncée
// aurait été celle du gabarit d'entrée, pour une page qui ne part plus.
const livres = [
  ...resultat.outputs
    .map((fichier) => fichier.path.slice(sortie.length + 1))
    .filter((nom) => !nom.endsWith(".html")),
  ...LANGUES_LIVREES.map((langue) => `${cheminDeLangue(langue).slice(1)}index.html`),
];

const tailles = await Promise.all(
  livres.map(async (nom) => {
    const contenu = await Bun.file(`${sortie}/${nom}`).bytes();
    return { nom, brut: contenu.length, gzip: gzipSync(contenu).length };
  }),
);
tailles.sort((a, b) => b.brut - a.brut);

const ko = (octets: number) => `${(octets / 1024).toFixed(2)} ko`;

for (const { nom, brut, gzip } of tailles) {
  console.log(`  ${nom.padEnd(28)} ${ko(brut).padStart(10)}  gzip ${ko(gzip)}`);
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

  const { style, icone } = ressourcesConstruites(construction);

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
    <link rel="icon" href="/${icone}" type="image/svg+xml" />
    <link rel="stylesheet" href="/${style}" />
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
 * Les trois pages d'accueil : « / » en français, « /en/ » et « /es/ ».
 *
 * Une seule application React, rendue trois fois avec une langue différente.
 * Ce sont de vraies pages à de vraies adresses, et non un choix conservé dans
 * le navigateur : une page traduite doit pouvoir se partager, se mettre en
 * signet, et surtout s'indexer — un moteur n'appuie sur aucun bouton.
 *
 * Le rendu en HTML vaut pour les trois, pour la raison qu'il valait déjà pour
 * une :
 *
 * - un robot qui n'exécute pas de JavaScript ne voit RIEN d'une racine vide.
 *   Les aperçus de lien s'en tirent — ils lisent les balises `og:` — mais un
 *   moteur qui n'exécute pas de script indexe une page vide ;
 * - si le paquet ne se charge pas, le visiteur voit une page blanche ;
 * - le premier affichage attend le paquet entier, sur un site consulté surtout
 *   en 4G.
 *
 * Le JavaScript reste servi, et `main.tsx` HYDRATE ce balisage au lieu de le
 * remplacer : le menu mobile et les questions dépliantes fonctionnent comme
 * avant. Il relit la langue dans l'adresse, pour retrouver exactement l'arbre
 * rendu ici.
 */
async function ecrireLesAccueils(construction: Awaited<ReturnType<typeof Bun.build>>) {
  // `renderToString`, et non `renderToStaticMarkup` comme les pages juridiques.
  //
  // Les deux rendent le même HTML à un détail près : `renderToStaticMarkup`
  // omet les séparateurs `<!-- -->` que React pose entre deux nœuds de texte
  // voisins. Les pages juridiques ne sont jamais hydratées, donc ces marques
  // ne leur serviraient à rien — et c'est bien pour cela qu'elles emploient
  // l'autre fonction.
  //
  // L'accueil, lui, EST hydraté. Sans les séparateurs, une phrase mêlant du
  // texte et une valeur — « Réservé aux {MIN_AGE} ans et plus » — arrive en un
  // seul nœud là où le client en attend trois. React abandonne alors
  // l'hydratation de cette branche et la reconstruit : erreur en console, et
  // le travail du serveur perdu là où il servait.
  const { renderToString } = await import("react-dom/server");
  const { createElement } = await import("react");
  const { App } = await import("./src/App.tsx");
  const { LANGUES, LANGUE_PAR_DEFAUT, chemin } = await import("./src/langues.ts");
  const { METADONNEES } = await import("./src/pages/metadonnees.ts");

  const { style, icone, script } = ressourcesConstruites(construction);

  // Le rendu de chaque langue, pour pouvoir les comparer ensuite.
  const rendus = new Map<string, string>();

  for (const langue of LANGUES) {
    const meta = METADONNEES[langue];
    const corps = renderToString(createElement(App, { langue }));

    // Le rendu a-t-il produit quelque chose ?
    //
    // Une erreur dans un composant, une exportation renommée, et
    // `renderToString` rendrait une chaîne vide sans rien dire : la page
    // repartirait en production avec une racine creuse — et personne ne le
    // verrait, puisque le JavaScript la remplirait quand même.
    if (!corps.includes("<h1")) {
      throw new Error(`Accueil « ${langue} » : le rendu serveur ne contient pas de titre.`);
    }

    rendus.set(langue, corps);
    const adresse = chemin(langue);

    /*
     * `hreflang` : ce qui dit à un moteur que ces trois pages sont la même,
     * dans trois langues. Sans ces balises, il les prend pour trois pages
     * distinctes au contenu voisin, et n'en garde souvent qu'une.
     *
     * Elles exigent des adresses absolues : sans origine connue, elles sont
     * omises plutôt qu'écrites contre un hôte qui ne répond pas. `x-default`
     * désigne la version servie à qui ne demande aucune de ces langues.
     */
    const alternatives = ORIGINE
      ? [
          ...LANGUES.map(
            (autre) =>
              `    <link rel="alternate" hreflang="${autre}" href="${ORIGINE}${chemin(autre)}" />`,
          ),
          `    <link rel="alternate" hreflang="x-default" href="${ORIGINE}${chemin(LANGUE_PAR_DEFAUT)}" />`,
        ]
      : [];

    // Le protocole Open Graph nomme les autres langues disponibles ; un
    // réseau social sert alors l'aperçu dans celle de son lecteur.
    const autresLocales = LANGUES.filter((autre) => autre !== langue).map(
      (autre) =>
        `    <meta property="og:locale:alternate" content="${METADONNEES[autre].ogLocale}" />`,
    );

    const canonique = ORIGINE
      ? [`    <link rel="canonical" href="${ORIGINE}${adresse}" />`]
      : // Sans origine, pas de canonique : une adresse relative ne dirait rien
        // de plus que la page elle-même, et une adresse inventée dirait
        // quelque chose de faux.
        [];

    const structurees = {
      "@context": "https://schema.org",
      "@type": "MobileApplication",
      name: "Weave",
      applicationCategory: "SocialNetworkingApplication",
      operatingSystem: "iOS, watchOS",
      inLanguage: meta.bcp47,
      description: meta.applicationDescription,
      offers: {
        "@type": "Offer",
        price: "0",
        priceCurrency: "EUR",
        description: meta.offreDescription,
      },
    };

    const html = `<!doctype html>
<html lang="${langue}">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover" />
    <meta name="theme-color" content="#16121f" media="(prefers-color-scheme: dark)" />
    <meta name="theme-color" content="#fffdf9" media="(prefers-color-scheme: light)" />
    <title>${echappe(meta.titre)}</title>
    <meta name="description" content="${echappe(meta.description)}" />
${[...canonique, ...alternatives].join("\n")}
    <link rel="apple-touch-icon" href="/apple-touch-icon.png" />
    <link rel="manifest" href="/site.webmanifest" />
    <meta property="og:site_name" content="Weave" />
    <meta property="og:title" content="${echappe(meta.partageTitre)}" />
    <meta property="og:description" content="${echappe(meta.partageDescription)}" />
    <meta property="og:type" content="website" />
    <meta property="og:locale" content="${meta.ogLocale}" />
${autresLocales.join("\n")}
    <meta property="og:image:width" content="1200" />
    <meta property="og:image:height" content="630" />
    <meta property="og:image:alt" content="${echappe(meta.partageImageAlt)}" />${balisesAbsolues(adresse.slice(1))}
    <link rel="icon" href="/${icone}" type="image/svg+xml" />
    <link rel="stylesheet" href="/${style}" />
    <script type="application/ld+json">
${JSON.stringify(structurees, null, 2).replace(/^/gm, "      ")}
    </script>
  </head>
  <body>
    <div id="racine">${corps}</div>
    <script type="module" src="/${script}"></script>
  </body>
</html>
`;

    // « / » est `index.html` à la racine ; « /en/ » est `en/index.html`.
    const fichier = langue === LANGUE_PAR_DEFAUT ? "index.html" : `${langue}/index.html`;
    await Bun.write(`${sortie}/${fichier}`, html);
  }

  /*
   * Deux langues qui affichent le même titre, c'est une traduction qui n'est
   * pas arrivée jusqu'à la page.
   *
   * Le cas se produit sans rien casser : un fournisseur de langue oublié, un
   * `useTraduction` remplacé par une constante française, et « /es/ » sort en
   * français — avec le bon `lang`, le bon titre d'onglet et les bonnes balises
   * `hreflang`. La page paraît traduite partout sauf dans son contenu, et
   * c'est précisément l'endroit qu'on ne relit pas à chaque construction.
   *
   * Le contrôle porte sur le `<h1>`, et non sur la page entière : celle-ci
   * diffère toujours un peu d'une langue à l'autre — le choix de langue y
   * marque la langue courante, le pied de page y préfixe ses ancres — même
   * quand plus rien n'est traduit. Le titre, lui, ne vient que de la
   * traduction.
   */
  const titres = new Map<string, string>();
  for (const [langue, corps] of rendus) {
    const titre = corps.match(/<h1[^>]*>([\s\S]*?)<\/h1>/)?.[1]?.replace(/<[^>]*>/g, "");
    if (titre === undefined) throw new Error(`Accueil « ${langue} » : titre introuvable.`);
    const deja = titres.get(titre);
    if (deja !== undefined) {
      throw new Error(
        `Accueil : « ${deja} » et « ${langue} » affichent le même titre. ` +
          "Une des deux traductions n'atteint pas la page.",
      );
    }
    titres.set(titre, langue);
  }
}

/**
 * Les noms empreintés des fichiers produits.
 *
 * On les retrouve dans la sortie plutôt que de les deviner : un changement de
 * nom casserait le style ou le script sans casser la construction.
 */
function ressourcesConstruites(construction: Awaited<ReturnType<typeof Bun.build>>) {
  const nom = (chemin: string) => chemin.slice(sortie.length + 1);
  const trouve = (fin: string, quoi: string) => {
    const fichier = construction.outputs.find((f) => f.path.endsWith(fin));
    if (!fichier) throw new Error(`${quoi} introuvable dans la construction.`);
    return nom(fichier.path);
  };
  return {
    style: trouve(".css", "Feuille de style"),
    script: trouve(".js", "Script"),
    icone: trouve(".svg", "Icône"),
  };
}

/**
 * Échappe ce qui casserait un attribut HTML.
 *
 * Ces textes sont les nôtres, pas ceux d'un visiteur : le risque n'est pas
 * l'injection mais l'apostrophe droite ou le guillemet qui refermerait
 * l'attribut au milieu d'une phrase — et la balise ne dirait plus rien.
 */
function echappe(texte: string): string {
  return texte
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
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
  const { LANGUES, chemin } = await import("./src/langues.ts");

  // Les trois accueils y figurent chacun : ils ont des adresses distinctes,
  // et `hreflang` dit qu'ils sont la même page — il ne les remplace pas dans
  // le plan. Les pages juridiques n'existent qu'en français, une fois.
  const adresses = [
    ...LANGUES.map((langue) => chemin(langue).slice(1)),
    ...DOCUMENTS.map((doc) => doc.slug),
  ];
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
 * Les six fils doivent pouvoir servir de TEXTE.
 *
 * Ils le font partout : titres de cartes, prix à l'unité, libellés d'étapes.
 * Deux d'entre eux ne le pouvaient pas — mandarine donnait 3,50 de contraste
 * sur une carte blanche et safran 3,25, là où un texte de cette taille en
 * réclame 4,5. Le site paraissait normal : une couleur un peu pâle ne
 * ressemble pas à un défaut, et rien ne la distingue des quatre autres tant
 * qu'on ne la mesure pas.
 *
 * Le contrôle porte sur le thème CLAIR seul. Le thème sombre pose les fils sur
 * de l'encre, où ils sont largement au-dessus du seuil — et ses valeurs sont
 * d'ailleurs des variantes distinctes.
 */
async function verifierQueLesFilsSeLisent() {
  const css = await Bun.file(`${racine}src/styles.css`).text();

  const couleur = (nom: string) => {
    const trouve = css.match(new RegExp(`--color-${nom}:\\s*(#[0-9a-fA-F]{6})`));
    if (!trouve) throw new Error(`La couleur « ${nom} » a disparu de la feuille de style.`);
    return trouve[1]!;
  };

  // Le fond le plus clair sur lequel un fil sert de texte : la carte.
  const carte = css.match(/--carte:\s*(#[0-9a-fA-F]{6})/)?.[1];
  if (!carte) throw new Error("La couleur des cartes a disparu de la feuille de style.");

  const canal = (v: number) => {
    const c = v / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  const luminance = (hex: string) => {
    const n = Number.parseInt(hex.slice(1), 16);
    return (
      0.2126 * canal((n >> 16) & 255) + 0.7152 * canal((n >> 8) & 255) + 0.0722 * canal(n & 255)
    );
  };
  const contraste = (a: string, b: string) => {
    const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p) as [number, number];
    return (x + 0.05) / (y + 0.05);
  };

  // 4,5 : le seuil d'un texte ordinaire. Les fils servent aussi à du texte de
  // 14 px, qui ne bénéficie d'aucun assouplissement.
  const SEUIL = 4.5;
  const fils = ["framboise", "mandarine", "safran", "menthe", "ocean", "iris"];

  const trop_pales = fils
    .map((nom) => ({ nom, valeur: couleur(nom), mesure: contraste(couleur(nom), carte) }))
    .filter(({ mesure }) => mesure < SEUIL);

  if (trop_pales.length > 0) {
    console.error("");
    console.error("  ✗ Ces fils ne se lisent pas en texte sur une carte :");
    for (const { nom, valeur, mesure } of trop_pales) {
      console.error(`      ${nom.padEnd(10)} ${valeur}  ${mesure.toFixed(2)} < ${SEUIL}`);
    }
    console.error("    Un fil sert de titre et de prix : il doit se lire, pas seulement se voir.");
    process.exit(1);
  }
}

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
  const { LANGUES, chemin } = await import("./src/langues.ts");
  const pages = [
    ...LANGUES.map((langue) => `${chemin(langue).slice(1)}index.html`),
    ...(await import("./src/pages/documents.ts")).DOCUMENTS.map((d) => `${d.slug}/index.html`),
  ];

  // `rel="canonical"`, puis les propriétés Open Graph et Twitter qui portent
  // une adresse. Chacune répond à la question « où cette page vit-elle ? ».
  const balises =
    /<link[^>]+rel="(?:canonical|alternate)"[^>]+href="([^"]+)"|<meta[^>]+(?:property|name)="(?:og:url|og:image|twitter:image)"[^>]+content="([^"]+)"/g;

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

/**
 * Une origine posée doit mener quelque part.
 *
 * Ne rien poser est permis : les adresses absolues sont alors omises, les
 * liens partagés sortent nus, et un avertissement le dit. Poser une adresse
 * FAUSSE est autre chose — les pages se déclarent canoniques à un endroit qui
 * ne répond pas, et le plan du site n'énumère que des adresses injoignables.
 * Un moteur qui suit ces indications retire les pages de son index plutôt que
 * de les y mettre : c'est pire que de n'avoir rien dit.
 *
 * `weave.app` est le domaine de remplacement de ce dépôt, et il ne répond pas.
 * L'envoi à TestFlight le refuse déjà pour `WEAVE_API_URL` ; la construction du
 * site ne refusait rien, et l'on pouvait remettre la valeur qui avait
 * précisément causé le défaut.
 *
 * Si ce domaine devient un jour le vôtre, c'est cette fonction qu'il faut
 * changer — ainsi que la garde correspondante dans `ios-testflight.yml`.
 */
function verifierQueLOrigineEstHabitable() {
  if (!ORIGINE) return;

  const refuser = (raison: string) => {
    console.error("");
    console.error(`  ✗ SITE.origine vaut « ${ORIGINE} ».`);
    console.error(`    ${raison}`);
    console.error("");
    console.error("    Les pages se déclareraient canoniques à cette adresse, et le plan");
    console.error("    du site n'énumérerait qu'elle. Mieux vaut ne rien poser du tout :");
    console.error("    les adresses absolues sont alors simplement omises.");
    process.exit(1);
  };

  if (!ORIGINE.startsWith("https://")) {
    refuser("Ce n'est pas une adresse https.");
  }
  if (/(^|\.)weave\.app$/.test(new URL(ORIGINE).hostname)) {
    refuser("C'est le domaine de remplacement du dépôt, et il ne répond pas.");
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
