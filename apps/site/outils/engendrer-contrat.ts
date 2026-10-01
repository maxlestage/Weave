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
  TIERS,
  UNIT_PRODUCTS,
  UNIT_SKUS,
  formatPrice,
  // Le chemin relatif, et non « @weave/contracts » : cette crate est du Rust,
  // elle n'est pas membre de l'espace de travail Bun, et le lui ajouter pour un
  // seul script de génération ferait porter un `package.json` à un paquet qui
  // n'en a pas besoin.
} from "../../../packages/contracts/src/index.ts";

const LANGUES = ["fr", "en", "es"] as const;
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
  ["MIN_AGE", MIN_AGE],
  ["MESSAGE_RETENTION_DAYS", MESSAGE_RETENTION_DAYS],
  ["ACCOUNT_PURGE_DAYS", ACCOUNT_PURGE_DAYS],
] as const) {
  ecrire(`pub const ${nom}: u32 = ${valeur};`);
}

ecrire(
  "",
  `pub const POLICY_UPDATED_LABEL: &str = ${r(POLICY_UPDATED_LABEL)};`,
  "",
  "// — Les catégories de plan ——————————————————————————————————————",
  "",
  "/// Les libellés de catégorie, dans les trois langues.",
  "pub const CATEGORIES: [(&str, Traduit<&str>); " + PLAN_CATEGORIES.length + "] = [",
);
for (const categorie of PLAN_CATEGORIES) {
  const par = Object.fromEntries(
    LANGUES.map((l) => [l, PLAN_CATEGORY_LABELS_PAR_LANGUE[l][categorie]]),
  );
  ecrire(`    (${r(categorie)}, ${traduit(par)}),`);
}
ecrire("];", "");

// — Le catalogue ————————————————————————————————————————————————
ecrire(
  "// — Le catalogue ————————————————————————————————————————————————",
  "",
  "pub struct Palier {",
  "    pub cle: &'static str,",
  "    pub nom: &'static str,",
  "    pub accroche: &'static str,",
  "    pub prix_centimes: u32,",
  "    /// Le prix mis en forme, par langue. Mis en forme ICI : `Intl` n'existe",
  "    /// pas en Rust, et deux mises en forme différentes donneraient deux prix",
  "    /// affichés différents pour un même nombre de centimes.",
  "    pub prix: Traduit<&'static str>,",
  "    pub atouts: &'static [&'static str],",
  "}",
  "",
  `pub const PALIERS: [Palier; ${PLAN_TIERS.length}] = [`,
);
for (const cle of PLAN_TIERS) {
  const palier = TIERS[cle];
  const prix = Object.fromEntries(
    LANGUES.map((l) => [l, formatPrice(palier.monthlyPriceCents, LOCALE[l])]),
  );
  ecrire(
    "    Palier {",
    `        cle: ${r(palier.tier)},`,
    `        nom: ${r(palier.name)},`,
    `        accroche: ${r(palier.tagline)},`,
    `        prix_centimes: ${palier.monthlyPriceCents},`,
    `        prix: ${traduit(prix)},`,
    `        atouts: &[${palier.highlights.map(r).join(", ")}],`,
    "    },",
  );
}
ecrire("];", "");

ecrire(
  "pub struct ProduitUnite {",
  "    pub sku: &'static str,",
  "    pub nom: &'static str,",
  "    pub description: &'static str,",
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
    `        description: ${r(produit.description)},`,
    `        prix_centimes: ${produit.priceCents},`,
    `        prix: ${traduit(prix)},`,
    "    },",
  );
}
ecrire("];", "");

const sortie = new URL("../src/contrat.rs", import.meta.url).pathname;
await Bun.write(sortie, lignes.join("\n"));
console.log(`src/contrat.rs engendré — ${lignes.length} lignes`);
