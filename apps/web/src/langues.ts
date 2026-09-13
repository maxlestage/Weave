/*
 * Les langues du site.
 *
 * Le français reste la langue de référence : c'est celle dans laquelle les
 * textes sont écrits et relus, et celle qui fait foi pour les pages
 * juridiques. Les autres en sont des traductions.
 */

export const LANGUES = ["fr", "en", "es"] as const;
export type Langue = (typeof LANGUES)[number];

export const LANGUE_PAR_DEFAUT: Langue = "fr";

/** Le nom de chaque langue, dans cette langue : c'est ainsi qu'on les propose. */
export const NOM_DE_LA_LANGUE: Readonly<Record<Langue, string>> = {
  fr: "Français",
  en: "English",
  es: "Español",
};

/** L'étiquette `lang` de la page, pour le balisage et les synthèses vocales. */
export const ETIQUETTE_LANG: Readonly<Record<Langue, string>> = {
  fr: "fr",
  en: "en",
  es: "es",
};

/**
 * Le préfixe d'adresse d'une langue. Le français n'en a pas : il occupe la
 * racine, et lui en donner un casserait les adresses déjà publiées.
 */
export function prefixe(langue: Langue): string {
  return langue === LANGUE_PAR_DEFAUT ? "" : `/${langue}`;
}

/** L'adresse d'une page dans une langue donnée. */
export function chemin(langue: Langue, slug = ""): string {
  const base = prefixe(langue);
  if (slug === "") return base === "" ? "/" : `${base}/`;
  return `${base}/${slug}`;
}

/**
 * La langue que désigne une adresse.
 *
 * Lue au démarrage côté client pour hydrater dans la même langue que celle du
 * rendu : une page rendue en anglais qui s'hydraterait en français
 * remplacerait tout son texte au premier affichage.
 */
export function langueDuChemin(chemin: string): Langue {
  const premier = chemin.split("/").filter(Boolean)[0];
  return LANGUES.find((l) => l === premier) ?? LANGUE_PAR_DEFAUT;
}

/** Un contenu décliné dans les trois langues. */
export type Traduit<T> = Readonly<Record<Langue, T>>;
