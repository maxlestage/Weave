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
 * À remplacer par le domaine définitif le jour où il est branché.
 */
export const ORIGINE = "https://weave.app";

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
  return trouvees;
}
