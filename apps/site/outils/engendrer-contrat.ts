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
  PLAN_CAPACITY_GROUP_MAX,
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
// Ce qu'un moteur, un réseau social et un navigateur lisent de l'accueil. Même
// raison : une seconde copie de ces textes en Rust indexerait un jour la page
// anglaise avec un titre français.
import { METADONNEES } from "../../web/src/pages/metadonnees.ts";
// L'identité de l'éditeur — le seul fichier que l'éditeur doit compléter. Les
// pages juridiques la lisent ; une seconde copie en Rust finirait par annoncer
// un SIREN différent de celui du site React, et une mention légale fausse
// engage pénalement son éditeur.
import { CONTACT, EDITEUR, SITE } from "../../web/src/pages/identite.ts";

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
  ["PLAN_CAPACITY_GROUP_MAX", PLAN_CAPACITY_GROUP_MAX],
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

/*
 * `--verifier` : engendre, et dit si les fichiers du dépôt sont ceux qu'on
 * vient d'engendrer — sans les remplacer.
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
/* — L'identité de l'éditeur ————————————————————————————————————— */

const identite: string[] = [
  "//! L'identité de l'éditeur. ENGENDRÉ. Ne pas modifier à la main.",
  "//!",
  "//! Produit par `apps/site/outils/engendrer-contrat.ts` à partir de",
  "//! `apps/web/src/pages/identite.ts`, qui reste LE SEUL FICHIER À COMPLÉTER.",
  "//!",
  "//! Rien n'est inventé. Une mention légale qui annoncerait une raison sociale",
  "//! ou un numéro d'immatriculation plausibles mais faux serait une fausse",
  "//! déclaration — et personne ne s'apercevrait qu'elle est fausse, puisqu'elle",
  "//! aurait l'air complète. Les valeurs manquantes sont donc des libellés",
  "//! explicites, surlignés à l'écran, et `est_a_completer` les reconnaît.",
  "",
  "/// Vrai si la valeur n'a pas encore été renseignée.",
  "///",
  "/// Le site REFUSE de se construire pour un vrai domaine tant qu'il en reste",
  "/// une : c'est le même critère que du côté React, et il tient à un préfixe.",
  "pub fn est_a_completer(valeur: &str) -> bool {",
  '    valeur.starts_with("[à compléter")',
  "}",
  "",
  "pub struct Hebergeur {",
  "    pub nom: &'static str,",
  "    pub adresse: &'static str,",
  "    pub telephone: &'static str,",
  "    /// Ex. « dans l'Union européenne (Irlande) » — sert aussi aux transferts.",
  "    pub region: &'static str,",
  "}",
  "",
  "pub struct Editeur {",
  "    /// Dénomination sociale, ou nom et prénom pour une personne physique.",
  "    pub raison_sociale: &'static str,",
  "    pub forme_juridique: &'static str,",
  "    pub adresse: &'static str,",
  "    /// SIREN ou SIRET, et numéro RCS avec sa ville d'immatriculation.",
  "    pub immatriculation: &'static str,",
  "    pub tva: &'static str,",
  "    /// La personne physique responsable du contenu.",
  "    pub directeur_publication: &'static str,",
  "    /// La LCEN exige un moyen de contact direct et effectif.",
  "    pub telephone: &'static str,",
  "    pub hebergeur: Hebergeur,",
  "}",
  "",
  "pub struct Contact {",
  "    /// Exercice des droits RGPD et questions sur les données.",
  "    pub confidentialite: &'static str,",
  "    /// Assistance, résiliation, réclamations.",
  "    pub support: &'static str,",
  "    /// Point de contact unique exigé par le règlement sur les services",
  "    /// numériques.",
  "    pub signalements: &'static str,",
  "}",
  "",
  "pub const EDITEUR: Editeur = Editeur {",
  `    raison_sociale: ${r(EDITEUR.raisonSociale)},`,
  `    forme_juridique: ${r(EDITEUR.formeJuridique)},`,
  `    adresse: ${r(EDITEUR.adresse)},`,
  `    immatriculation: ${r(EDITEUR.immatriculation)},`,
  `    tva: ${r(EDITEUR.tva)},`,
  `    directeur_publication: ${r(EDITEUR.directeurPublication)},`,
  `    telephone: ${r(EDITEUR.telephone)},`,
  "    hebergeur: Hebergeur {",
  `        nom: ${r(EDITEUR.hebergeur.nom)},`,
  `        adresse: ${r(EDITEUR.hebergeur.adresse)},`,
  `        telephone: ${r(EDITEUR.hebergeur.telephone)},`,
  `        region: ${r(EDITEUR.hebergeur.region)},`,
  "    },",
  "};",
  "",
  "pub const CONTACT: Contact = Contact {",
  `    confidentialite: ${r(CONTACT.confidentialite)},`,
  `    support: ${r(CONTACT.support)},`,
  `    signalements: ${r(CONTACT.signalements)},`,
  "};",
  "",
];

/*
 * La liste à plat de tout ce qui se renseigne, pour la garde de construction.
 *
 * Un `struct` Rust ne se parcourt pas comme un objet JavaScript : la version
 * TypeScript descend `EDITEUR`, `CONTACT` et `SITE` par réflexion, ce que Rust
 * ne sait pas faire. On engendre donc la liste, avec LES MÊMES NOMS de champ —
 * ce sont ceux que l'éditeur doit retrouver dans `identite.ts`, et un nom
 * traduit en passant l'enverrait chercher une clé qui n'existe pas.
 */
const aplat: [string, string][] = [];
const descendre = (objet: Record<string, unknown>, prefixe: string) => {
  for (const [cle, valeur] of Object.entries(objet)) {
    if (typeof valeur === "string") aplat.push([`${prefixe}${cle}`, valeur]);
    else if (valeur && typeof valeur === "object") {
      descendre(valeur as Record<string, unknown>, `${prefixe}${cle}.`);
    }
  }
};
descendre(EDITEUR, "EDITEUR.");
descendre(CONTACT, "CONTACT.");
// L'adresse du site n'est pas une mention légale, mais elle se renseigne au
// même endroit et son oubli coûte le référencement des pages juridiques.
descendre(SITE, "SITE.");

identite.push(
  "/// Tout ce qui se renseigne dans `identite.ts`, à plat : le chemin du champ",
  "/// et sa valeur actuelle.",
  "///",
  "/// Les noms sont ceux du fichier TypeScript, et non leur transcription en",
  "/// Rust : c'est là que l'éditeur doit aller les remplir.",
  `pub const VALEURS_A_RENSEIGNER: [(&str, &str); ${aplat.length}] = [`,
  ...aplat.map(([chemin, valeur]) => `    (${r(chemin)}, ${r(valeur)}),`),
  "];",
  "",
);

/* — Les métadonnées de l'accueil ——————————————————————————————— */

const metadonnees: string[] = [
  "//! Ce qu'un moteur, un réseau social et un navigateur lisent de l'accueil.",
  "//! ENGENDRÉ. Ne pas modifier à la main.",
  "//!",
  "//! Produit par `apps/site/outils/engendrer-contrat.ts` à partir de",
  "//! `apps/web/src/pages/metadonnees.ts`, qui en reste la source.",
  "",
  "use crate::langues::Langue;",
  "",
  "pub struct Metadonnees {",
  "    /// Titre de l'onglet et des résultats de recherche.",
  "    pub titre: &'static str,",
  "    /// Description de référencement.",
  "    pub description: &'static str,",
  "    /// Titre du partage — plus court, et sans le nom du site répété.",
  "    pub partage_titre: &'static str,",
  "    pub partage_description: &'static str,",
  "    /// Texte de remplacement de l'image de partage.",
  "    pub partage_image_alt: &'static str,",
  "    /// Étiquette Open Graph : langue ET région, séparées par un tiret bas.",
  "    ///",
  "    /// Ce n'est PAS `fr`/`en`/`es` avec une région devinée : l'anglais du",
  "    /// site est `en_GB`, et ses prix se mettent en forme en `en-IE`. Trois",
  "    /// étiquettes pour trois usages, et aucune ne se déduit des autres.",
  "    pub og_locale: &'static str,",
  "    /// Étiquette BCP 47, pour les données structurées.",
  "    pub bcp47: &'static str,",
  "    /// Description de l'application, dans les données structurées.",
  "    pub application_description: &'static str,",
  "    pub offre_description: &'static str,",
  "}",
  "",
  "/// Les métadonnées d'une langue.",
  "pub const fn metadonnees(langue: Langue) -> &'static Metadonnees {",
  "    match langue {",
];
for (const l of LANGUES) {
  const variante = { fr: "Fr", en: "En", es: "Es" }[l];
  metadonnees.push(`        Langue::${variante} => &${variante.toUpperCase()},`);
}
metadonnees.push("    }", "}", "");
for (const l of LANGUES) {
  const m = METADONNEES[l];
  metadonnees.push(
    `const ${l.toUpperCase()}: Metadonnees = Metadonnees {`,
    `    titre: ${r(m.titre)},`,
    `    description: ${r(m.description)},`,
    `    partage_titre: ${r(m.partageTitre)},`,
    `    partage_description: ${r(m.partageDescription)},`,
    `    partage_image_alt: ${r(m.partageImageAlt)},`,
    `    og_locale: ${r(m.ogLocale)},`,
    `    bcp47: ${r(m.bcp47)},`,
    `    application_description: ${r(m.applicationDescription)},`,
    `    offre_description: ${r(m.offreDescription)},`,
    "};",
    "",
  );
}

const verifier = Bun.argv.includes("--verifier");

let aDerive = false;

/** Écrit un fichier engendré, ou vérifie que celui du dépôt lui est égal. */
async function poser(nom: string, contenu: string[]) {
  const sortie = new URL(`../src/${nom}`, import.meta.url).pathname;
  const temporaire = verifier ? `${sortie}.verification` : sortie;
  await Bun.write(temporaire, contenu.join("\n"));

  /*
   * On repasse `rustfmt` dessus.
   *
   * Écrire du Rust déjà bien coupé depuis un script TypeScript demanderait de
   * reproduire les règles de coupe de `rustfmt` — la largeur de ligne, le
   * moment où un littéral de structure passe en plusieurs lignes — et de s'en
   * écarter à la première occasion. Autant laisser l'outil qui les connaît
   * faire le travail : sans quoi `cargo fmt` reformate le fichier engendré, la
   * garde qui vérifie qu'il est à jour échoue, et on régénère en rond.
   */
  const miseEnForme = Bun.spawnSync(["rustfmt", "--edition", "2024", temporaire]);
  if (!miseEnForme.success) {
    // Pas une erreur fatale : le Rust engendré est valide, seulement mal
    // coupé. Le taire, en revanche, laisserait croire qu'il est canonique.
    console.warn(
      `rustfmt n'a pas pu mettre en forme ${nom} : ${new TextDecoder().decode(miseEnForme.stderr).trim()}`,
    );
  }

  const relu = await Bun.file(temporaire).text();
  const lignesLues = relu.split("\n").length;

  if (!verifier) {
    console.log(`src/${nom} engendré — ${lignesLues} lignes`);
    return;
  }

  const dansLeDepot = await Bun.file(sortie)
    .text()
    .catch(() => null);
  await Bun.file(temporaire).delete();

  if (dansLeDepot === relu) {
    console.log(`src/${nom} est à jour — ${lignesLues} lignes`);
  } else {
    console.error(
      `src/${nom} a DÉRIVÉ de sa source.\n` +
        "Le site annoncerait donc autre chose que ce que la source dit.\n" +
        "  bun run apps/site/outils/engendrer-contrat.ts",
    );
    aDerive = true;
  }
}

await poser("contrat.rs", lignes);
await poser("metadonnees.rs", metadonnees);
await poser("identite.rs", identite);

if (aDerive) process.exit(1);
