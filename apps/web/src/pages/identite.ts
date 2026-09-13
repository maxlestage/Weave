/*
 * Identité de l'éditeur — LE SEUL FICHIER À COMPLÉTER.
 *
 * Les pages juridiques ne codent aucune de ces valeurs en dur : elles les
 * lisent ici. Remplacer les chaînes ci-dessous suffit à rendre les quatre
 * pages exactes.
 *
 * Rien n'est inventé. Une mention légale qui annoncerait une raison sociale ou
 * un numéro d'immatriculation plausibles mais faux serait une fausse
 * déclaration — et personne ne s'apercevrait qu'elle est fausse, puisqu'elle
 * aurait l'air complète. Les valeurs manquantes sont donc des libellés
 * explicites, surlignés à l'écran par le composant `AC`, et le test de
 * construction refuse de les laisser passer inaperçues.
 *
 * Obligations correspondantes : article 6-III de la loi pour la confiance dans
 * l'économie numérique (identité de l'éditeur et de l'hébergeur), et
 * articles 13 et 14 du RGPD (identité du responsable du traitement).
 */

/** Marque une valeur que l'éditeur doit renseigner avant toute publication. */
const A_COMPLETER = (quoi: string) => `[à compléter : ${quoi}]`;

export const EDITEUR = {
  /** Dénomination sociale, ou nom et prénom si l'éditeur est une personne physique. */
  raisonSociale: A_COMPLETER("raison sociale"),
  /** SARL, SAS, auto-entrepreneur… et capital social le cas échéant. */
  formeJuridique: A_COMPLETER("forme juridique et capital social"),
  /** Siège social : numéro, voie, code postal, commune. */
  adresse: A_COMPLETER("adresse du siège social"),
  /** SIREN ou SIRET, et numéro RCS avec sa ville d'immatriculation. */
  immatriculation: A_COMPLETER("SIREN et RCS"),
  /** Numéro de TVA intracommunautaire, si l'éditeur y est assujetti. */
  tva: A_COMPLETER("numéro de TVA intracommunautaire"),
  /** Directeur de la publication : la personne physique responsable du contenu. */
  directeurPublication: A_COMPLETER("directeur de la publication"),
  /** Téléphone : la LCEN exige un moyen de contact direct et effectif. */
  telephone: A_COMPLETER("numéro de téléphone"),
  hebergeur: {
    nom: A_COMPLETER("nom de l'hébergeur"),
    adresse: A_COMPLETER("adresse de l'hébergeur"),
    telephone: A_COMPLETER("téléphone de l'hébergeur"),
    /** Ex. « dans l'Union européenne (Irlande) » — sert aussi à la section transferts. */
    region: A_COMPLETER("région d'hébergement"),
  },
} as const;

export const CONTACT = {
  /** Exercice des droits RGPD et questions sur les données. */
  confidentialite: A_COMPLETER("adresse de contact données personnelles"),
  /** Assistance, résiliation, réclamations. */
  support: A_COMPLETER("adresse de contact assistance"),
  /** Point de contact unique exigé par le règlement sur les services numériques. */
  signalements: A_COMPLETER("adresse de contact signalements"),
} as const;

/**
 * Date de la version en vigueur des textes.
 *
 * Elle est écrite à la main, et c'est voulu : une date calculée à la
 * construction changerait à chaque déploiement et laisserait croire à une
 * révision qui n'a pas eu lieu. Or c'est cette date qui atteste quelle version
 * l'utilisateur a acceptée.
 */
export const MISE_A_JOUR = "12 septembre 2026";

/**
 * Adresse publique du site, pour les URL canoniques et le plan du site.
 *
 * Lue à la construction dans `SITE_ORIGINE`, et `null` tant qu'elle n'est pas
 * posée. Elle valait « https://weave.app » en dur — un domaine qui ne répond
 * pas. Toutes les pages juridiques se déclaraient donc canoniques à une
 * adresse morte, et le plan du site n'énumérait que des URL injoignables :
 * un moteur qui suit ces indications retire les pages de son index plutôt que
 * de les y mettre. Mieux vaut ne rien déclarer que de désigner le vide.
 *
 * Le jour où le domaine est branché, poser `SITE_ORIGINE` suffit à tout
 * rallumer — sans barre oblique finale.
 */
export const SITE = {
  /** Adresse publique du site, sans barre oblique finale. Ex. https://weave.app */
  origine: A_COMPLETER("adresse publique du site, ex. https://weave.app"),
} as const;

/**
 * L'origine effective : celle du site, ou celle que la construction impose.
 *
 * `SITE_ORIGINE` l'emporte pour les préproductions, qui vivent à une autre
 * adresse que la production sans qu'on veuille dupliquer ce fichier.
 *
 * Vaut `null` tant que rien n'est renseigné, et c'est le point : l'adresse
 * était codée en dur à « https://weave.app », un domaine qui ne répond pas.
 * Les pages juridiques se déclaraient donc canoniques à une adresse morte, et
 * le plan du site n'énumérait que des URL injoignables — un moteur qui suit
 * ces indications retire les pages de son index plutôt que de les y mettre.
 * Mieux vaut ne rien déclarer que de désigner le vide.
 */
export const ORIGINE: string | null = (() => {
  const impose = (globalThis as { process?: { env?: Record<string, string | undefined> } }).process
    ?.env?.SITE_ORIGINE;
  const choisie = impose || SITE.origine;
  if (!choisie || choisie.startsWith("[à compléter")) return null;
  return choisie.replace(/\/+$/, "");
})();

/** Toutes les valeurs restées à compléter, pour le test de construction. */
export function valeursManquantes(): readonly string[] {
  const trouvees: string[] = [];
  const parcourir = (objet: Record<string, unknown>, prefixe: string) => {
    for (const [cle, valeur] of Object.entries(objet)) {
      if (typeof valeur === "string") {
        if (valeur.startsWith("[à compléter")) trouvees.push(`${prefixe}${cle}`);
      } else if (valeur && typeof valeur === "object") {
        parcourir(valeur as Record<string, unknown>, `${prefixe}${cle}.`);
      }
    }
  };
  parcourir(EDITEUR, "EDITEUR.");
  parcourir(CONTACT, "CONTACT.");
  // L'adresse du site n'est pas une mention légale, mais elle se renseigne au
  // même endroit et son oubli coûte le référencement des pages juridiques :
  // elle mérite la même liste.
  parcourir(SITE, "SITE.");
  return trouvees;
}
