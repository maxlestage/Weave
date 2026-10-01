/*
 * Engendre `src/contrat.rs` à partir de `@weave/contracts`.
 *
 * ## Pourquoi engendrer plutôt que recopier
 *
 * Le site lit du contrat partagé six nombres, les libellés de catégorie en
 * trois langues, et TOUT le catalogue de prix — cinq paliers et cinq produits
 * à l'unité. Le recopier à la main en Rust en ferait une quatrième source de
 * vérité pour les tarifs, après le contrat, l'API et l'application iOS.
 *
 * Celles-là sont tenues par des tests de contrat qui rapprochent les valeurs
 * une à une. Cela marche, mais chaque nouvelle valeur demande une ligne de
 * plus dans une table d'accords — et une valeur oubliée dans cette table n'est
 * rapprochée de rien.
 *
 * Une source ENGENDRÉE n'a pas ce défaut : elle ne peut pas diverger, puisqu'elle
 * n'est pas écrite. Le test se réduit à « le fichier engendré est-il à jour ».
 *
 * ## Pourquoi en TypeScript et non dans un `build.rs`
 *
 * Le catalogue n'est pas une liste de constantes : c'est du TypeScript avec
 * des objets imbriqués, des interfaces et des renvois entre valeurs
 * (`requestsPerDay: REQUESTS_PER_DAY_FLOOR`). L'analyser depuis Rust
 * reviendrait à écrire un petit évaluateur TypeScript, et à se tromper.
 *
 * Ici, c'est le vrai module qui est importé. Aucune analyse, donc aucune
 * façon de mal lire.
 *
 *     bun run apps/site/outils/engendrer-contrat.ts
 */

import {
  ACCOUNT_PURGE_DAYS,
  MAX_OPEN_PLANS,
  MESSAGE_RETENTION_DAYS,
  MIN_AGE,
  PLAN_CATEGORIES,
  PLAN_CATEGORY_LABELS_PAR_LANGUE,
  PLAN_MIN_LEAD_MINUTES,
  PLAN_TIERS,
  POLICY_UPDATED_LABEL,
  REQUESTS_PER_DAY_FLOOR,
  REQUEST_MIN_CHARS,
  TIERS,
  TIER_COPY_PAR_LANGUE,
  UNIT_DESCRIPTIONS_PAR_LANGUE,
  UNIT_PRODUCTS,
  UNIT_SKUS,
  formatPrice,
  type FilterDepth,
  // Le chemin relatif, et non « @weave/contracts » : cette crate est du Rust,
  // elle n'est pas membre de l'espace de travail Bun, et le lui ajouter pour un
  // seul script de génération ferait porter un `package.json` à un paquet qui
  // n'en a pas besoin.
} from "../../../packages/contracts/src/index.ts";
// La liste des pages juridiques vit encore du côté React, et c'est la seule
// source de leurs adresses, de leurs libellés et de leurs dates de version.
// L'engendrer évite d'en écrire une seconde : les dates, en particulier, ne
// supportent pas d'être recopiées — celle de la politique de confidentialité
// EST la version que portent les consentements enregistrés.
import { DOCUMENTS } from "../../web/src/pages/documents.ts";

const LANGUES = ["fr", "en", "es"] as const;

/**
 * La correspondance entre les valeurs du contrat et les variantes Rust.
 *
 * Elle est exhaustive par le typage : `Record<FilterDepth, …>` refuse de
 * compiler s'il manque une finesse, et une finesse ajoutée au contrat casse
 * donc la génération au lieu de produire du Rust qui ne compile pas.
 */
const CRITERES: Record<FilterDepth, "Base" | "Etendus" | "Precis"> = {
  base: "Base",
  etendus: "Etendus",
  precis: "Precis",
};
const LOCALE: Record<(typeof LANGUES)[number], string> = {
  fr: "fr-FR",
  en: "en-IE",
  es: "es-ES",
};

/** Une chaîne Rust, échappée. */
const r = (texte: string) => `"${texte.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;

/** Un tableau Rust de trois chaînes, dans l'ordre des langues. */
const traduit = (par: Record<string, string>) =>
  `Traduit([${LANGUES.map((l) => r(par[l] ?? par.fr)).join(", ")}])`;

/**
 * Trois LISTES de chaînes, dans l'ordre des langues.
 *
 * Les atouts d'un palier n'ont pas forcément le même nombre d'entrées d'une
 * langue à l'autre — ils sont rédigés, pas traduits mot à mot — d'où une
 * tranche et non un tableau de taille fixe.
 */
const traduitListes = (par: Record<string, readonly string[]>) =>
  `Traduit([${LANGUES.map((l) => `&[${(par[l] ?? par.fr).map(r).join(", ")}]`).join(", ")}])`;

const lignes: string[] = [];
const ecrire = (...l: string[]) => lignes.push(...l);

ecrire(
  "//! Le contrat partagé, ENGENDRÉ. Ne pas modifier à la main.",
  "//!",
  "//! Produit par `apps/site/outils/engendrer-contrat.ts` à partir de",
  "//! `packages/contracts`. Le contrat reste la seule source de vérité : ce",
  "//! fichier n'en est qu'une projection, et un test vérifie qu'il est à jour.",
  "//!",
  "//! `bun run apps/site/outils/engendrer-contrat.ts` pour le régénérer.",
  "",
  "#![allow(dead_code)]",
  "",
  "use crate::langues::Traduit;",
  "",
  "// — Les nombres —————————————————————————————————————————————————",
  "",
);

for (const [nom, valeur] of [
  ["REQUESTS_PER_DAY_FLOOR", REQUESTS_PER_DAY_FLOOR],
  ["MAX_OPEN_PLANS", MAX_OPEN_PLANS],
  ["PLAN_MIN_LEAD_MINUTES", PLAN_MIN_LEAD_MINUTES],
  ["REQUEST_MIN_CHARS", REQUEST_MIN_CHARS],
  ["MIN_AGE", MIN_AGE],
  ["MESSAGE_RETENTION_DAYS", MESSAGE_RETENTION_DAYS],
  ["ACCOUNT_PURGE_DAYS", ACCOUNT_PURGE_DAYS],
] as const) {
  ecrire(`pub const ${nom}: u32 = ${valeur};`);
}

/** « benevolat » devient « Benevolat » : les clés sont des mots simples. */
const variante = (cle: string) => cle[0]!.toUpperCase() + cle.slice(1);

ecrire(
  "",
  `pub const POLICY_UPDATED_LABEL: &str = ${r(POLICY_UPDATED_LABEL)};`,
  "",
  "// — Les catégories de plan ——————————————————————————————————————",
  "",
  "/// Une catégorie de plan.",
  "///",
  "/// Une énumération, et non une clé à rapprocher d'une table. Les sections du",
  "/// site nomment une catégorie pour afficher son libellé ; avec une chaîne,",
  "/// une faute de frappe compilait et affichait `sport` tel quel au visiteur,",
  "/// ou demandait un cas par défaut qui revenait au même. Ici, elle ne compile",
  "/// pas.",
  "#[derive(Clone, Copy, PartialEq, Eq, Debug)]",
  "pub enum Categorie {",
);
for (const categorie of PLAN_CATEGORIES) ecrire(`    ${variante(categorie)},`);
ecrire(
  "}",
  "",
  "impl Categorie {",
  "    /// La clé telle que l'API et le contrat la portent.",
  "    pub const fn cle(self) -> &'static str {",
  "        match self {",
);
for (const categorie of PLAN_CATEGORIES) {
  ecrire(`            Categorie::${variante(categorie)} => ${r(categorie)},`);
}
ecrire(
  "        }",
  "    }",
  "",
  "    /// Le libellé affiché, dans les trois langues.",
  "    pub const fn libelle(self) -> Traduit<&'static str> {",
  "        match self {",
);
for (const categorie of PLAN_CATEGORIES) {
  const par = Object.fromEntries(
    LANGUES.map((l) => [l, PLAN_CATEGORY_LABELS_PAR_LANGUE[l][categorie]]),
  );
  ecrire(`            Categorie::${variante(categorie)} => ${traduit(par)},`);
}
ecrire("        }", "    }", "}", "");

ecrire(
  "/// Les catégories, dans l'ordre où le contrat les déclare — c'est celui dans",
  "/// lequel le site les montre.",
  `pub const CATEGORIES: [Categorie; ${PLAN_CATEGORIES.length}] = [`,
);
for (const categorie of PLAN_CATEGORIES) ecrire(`    Categorie::${variante(categorie)},`);
ecrire("];", "");

// — Le catalogue ————————————————————————————————————————————————
ecrire(
  "// — Le catalogue ————————————————————————————————————————————————",
  "",
  "/// La finesse des critères de recherche qu'un palier ouvre.",
  "///",
  "/// Une énumération, là où le contrat a une union de chaînes. Le libellé",
  "/// affiché n'est PAS ici : il appartient à la section qui le montre, et il",
  "/// est traduit. Ce qui est ici, c'est le fait — et un `match` dessus ne",
  "/// compile pas s'il oublie un cas, là où une chaîne demanderait un cas par",
  "/// défaut qui afficherait « base » tel quel au visiteur.",
  "#[derive(Clone, Copy, PartialEq, Eq, Debug)]",
  "pub enum Criteres {",
  "    Base,",
  "    Etendus,",
  "    Precis,",
  "}",
  "",
  "pub struct Palier {",
  "    pub cle: &'static str,",
  "    pub nom: &'static str,",
  "    pub accroche: Traduit<&'static str>,",
  "    pub prix_centimes: u32,",
  "    /// Le prix mis en forme, par langue. Mis en forme ICI : `Intl` n'existe",
  "    /// pas en Rust, et deux mises en forme différentes donneraient deux prix",
  "    /// affichés différents pour un même nombre de centimes.",
  "    pub prix: Traduit<&'static str>,",
  "    pub atouts: Traduit<&'static [&'static str]>,",
  "    pub demandes_par_jour: u32,",
  "    pub jours_a_l_avance: u32,",
  "    pub criteres: Criteres,",
  "    pub plans_de_groupe: bool,",
  "}",
  "",
  `pub const PALIERS: [Palier; ${PLAN_TIERS.length}] = [`,
);
for (const cle of PLAN_TIERS) {
  const palier = TIERS[cle];
  const prix = Object.fromEntries(
    LANGUES.map((l) => [l, formatPrice(palier.monthlyPriceCents, LOCALE[l])]),
  );
  const accroche = Object.fromEntries(
    LANGUES.map((l) => [l, TIER_COPY_PAR_LANGUE[l][cle].tagline]),
  );
  const atouts = Object.fromEntries(
    LANGUES.map((l) => [l, TIER_COPY_PAR_LANGUE[l][cle].highlights]),
  );
  const criteres = CRITERES[palier.entitlements.filters];
  ecrire(
    "    Palier {",
    `        cle: ${r(palier.tier)},`,
    `        nom: ${r(palier.name)},`,
    `        accroche: ${traduit(accroche)},`,
    `        prix_centimes: ${palier.monthlyPriceCents},`,
    `        prix: ${traduit(prix)},`,
    `        atouts: ${traduitListes(atouts)},`,
    `        demandes_par_jour: ${palier.entitlements.requestsPerDay},`,
    `        jours_a_l_avance: ${palier.entitlements.daysAhead},`,
    `        criteres: Criteres::${criteres},`,
    `        plans_de_groupe: ${palier.entitlements.groupPlans},`,
    "    },",
  );
}
ecrire("];", "");

ecrire(
  "pub struct ProduitUnite {",
  "    pub sku: &'static str,",
  "    pub nom: &'static str,",
  "    pub description: Traduit<&'static str>,",
  "    pub prix_centimes: u32,",
  "    pub prix: Traduit<&'static str>,",
  "}",
  "",
  `pub const PRODUITS_UNITE: [ProduitUnite; ${UNIT_SKUS.length}] = [`,
);
for (const sku of UNIT_SKUS) {
  const produit = UNIT_PRODUCTS[sku];
  const prix = Object.fromEntries(
    LANGUES.map((l) => [l, formatPrice(produit.priceCents, LOCALE[l])]),
  );
  ecrire(
    "    ProduitUnite {",
    `        sku: ${r(sku)},`,
    `        nom: ${r(produit.name)},`,
    `        description: ${traduit(
      Object.fromEntries(LANGUES.map((l) => [l, UNIT_DESCRIPTIONS_PAR_LANGUE[l][sku]])),
    )},`,
    `        prix_centimes: ${produit.priceCents},`,
    `        prix: ${traduit(prix)},`,
    "    },",
  );
}
ecrire("];", "");

// — Les pages juridiques ————————————————————————————————————————
ecrire(
  "// — Les pages juridiques ————————————————————————————————————————",
  "",
  "/// Une page juridique.",
  "///",
  "/// Elles ne sont publiées QU'EN FRANÇAIS, et leurs adresses ne portent donc",
  "/// pas de préfixe de langue. Traduire des conditions générales n'est pas un",
  "/// travail de langue : une traduction non relue engagerait sur un texte que",
  "/// personne n'a validé.",
  "pub struct PageJuridique {",
  "    /// Adresse de la page, sans barre oblique initiale.",
  "    pub adresse: &'static str,",
  "    /// Titre de l'onglet et du partage.",
  "    pub titre: &'static str,",
  "    /// Description de référencement — celle que les moteurs affichent.",
  "    pub description: &'static str,",
  "    /// Libellé court, pour le pied de page.",
  "    pub lien: &'static str,",
  "    /// Date de la version en vigueur DE CE DOCUMENT, et non des quatre.",
  "    pub mise_a_jour: &'static str,",
  "}",
  "",
  `pub const PAGES_JURIDIQUES: [PageJuridique; ${DOCUMENTS.length}] = [`,
);
for (const doc of DOCUMENTS) {
  ecrire(
    "    PageJuridique {",
    `        adresse: ${r(doc.slug)},`,
    `        titre: ${r(doc.titre)},`,
    `        description: ${r(doc.description)},`,
    `        lien: ${r(doc.lien)},`,
    `        mise_a_jour: ${r(doc.miseAJour)},`,
    "    },",
  );
}
ecrire("];", "");

const sortie = new URL("../src/contrat.rs", import.meta.url).pathname;

/*
 * `--verifier` : engendre, et dit si le fichier du dépôt est celui qu'on
 * vient d'engendrer — sans le remplacer.
 *
 * Ma première version de cette garde comparait avec `git diff --exit-code`, ce
 * qui était faux : elle échouait sur tout fichier modifié mais pas encore
 * validé, donc pendant exactement le travail qu'elle devait accompagner. Elle
 * ne disait pas « le fichier a dérivé du contrat », elle disait « le fichier a
 * changé depuis la dernière validation » — deux choses différentes, dont une
 * seule est un défaut.
 *
 * On compare donc au contenu engendré, et à rien d'autre. L'état de git n'y
 * entre pas.
 */
const verifier = Bun.argv.includes("--verifier");

const temporaire = verifier ? `${sortie}.verification` : sortie;
await Bun.write(temporaire, lignes.join("\n"));

/*
 * On repasse `rustfmt` dessus.
 *
 * Écrire du Rust déjà bien coupé depuis un script TypeScript demanderait de
 * reproduire les règles de coupe de `rustfmt` — la largeur de ligne, le moment
 * où un littéral de structure passe en plusieurs lignes — et de s'en écarter
 * à la première occasion. Autant laisser l'outil qui les connaît faire le
 * travail : sans quoi `cargo fmt` reformate le fichier engendré, le test qui
 * vérifie qu'il est à jour échoue, et on régénère en rond.
 */
const miseEnForme = Bun.spawnSync(["rustfmt", "--edition", "2024", temporaire]);
if (!miseEnForme.success) {
  // Pas une erreur fatale : le Rust engendré est valide, seulement mal coupé.
  // Le taire, en revanche, laisserait croire que le fichier est canonique.
  console.warn(
    `rustfmt n'a pas pu mettre en forme le fichier : ${new TextDecoder().decode(miseEnForme.stderr).trim()}`,
  );
}

const relu = await Bun.file(temporaire).text();

if (verifier) {
  const dansLeDepot = await Bun.file(sortie)
    .text()
    .catch(() => null);
  await Bun.file(temporaire).delete();

  if (dansLeDepot === relu) {
    console.log(`src/contrat.rs est à jour — ${relu.split("\n").length} lignes`);
  } else {
    console.error(
      "src/contrat.rs a DÉRIVÉ du contrat partagé.\n" +
        "Le site annoncerait donc autre chose que ce que le contrat dit.\n" +
        "  bun run apps/site/outils/engendrer-contrat.ts",
    );
    process.exit(1);
  }
} else {
  console.log(`src/contrat.rs engendré — ${relu.split("\n").length} lignes`);
}
