/*
 * La liste des pages juridiques — métadonnées seules.
 *
 * Ce fichier ne contient AUCUN composant, et c'est délibéré : le pied de page
 * l'importe pour afficher ses liens, et il est présent sur la page d'accueil.
 * Y référencer les composants des pages y ferait entrer les quatre documents
 * dans le paquet de l'accueil — quarante kilo-octets de texte juridique
 * téléchargés par quelqu'un venu lire la page de présentation.
 *
 * La construction fait la jonction entre un `slug` et son composant.
 */

import { POLICY_UPDATED_LABEL } from "@weave/contracts";

export interface Document {
  /** Adresse de la page, sans barre oblique initiale. */
  readonly slug: string;
  /** Titre de l'onglet et du partage. */
  readonly titre: string;
  /** Description de référencement — celle que les moteurs affichent. */
  readonly description: string;
  /** Libellé court, pour le pied de page. */
  readonly lien: string;
  /**
   * Date de la version en vigueur DE CE DOCUMENT.
   *
   * Une date par document, et non une pour tous. Les quatre textes ne changent
   * pas ensemble : corriger un tarif touche les conditions de vente, pas la
   * politique de confidentialité.
   *
   * Une date commune avait une conséquence qu'on ne devine pas. Celle de la
   * politique de confidentialité EST la version que portent les consentements
   * enregistrés : la changer périme ceux donnés sur la précédente, et le
   * traitement des données sensibles s'arrête jusqu'à ce qu'ils soient
   * redonnés. Un changement de prix aurait donc fait redemander à tout le
   * monde son accord sur l'orientation sexuelle.
   */
  readonly miseAJour: string;
}

export const DOCUMENTS: readonly Document[] = [
  {
    slug: "confidentialite",
    titre: "Politique de confidentialité",
    lien: "Confidentialité",
    description:
      "Ce que Weave collecte, pourquoi, combien de temps, et ce que nous nous interdisons. Position arrondie au kilomètre, aucun cookie, aucune revente de données.",
    // Elle gouverne les consentements : elle vient des contrats, et la
    // changer fait redemander l'accord aux données sensibles.
    miseAJour: POLICY_UPDATED_LABEL,
  },
  {
    slug: "cgu",
    titre: "Conditions générales d'utilisation",
    lien: "Conditions d'utilisation",
    description:
      "Ce que Weave propose, ce que nous attendons de vous, et les règles qu'aucun abonnement ne lève.",
    miseAJour: "12 septembre 2026",
  },
  {
    slug: "cgv",
    titre: "Conditions générales de vente",
    lien: "Conditions de vente",
    description:
      "Abonnements et achats à l'unité : prix, reconduction, résiliation, rétractation et remboursement.",
    // Les tarifs à l'unité ont changé ce jour-là.
    miseAJour: "13 septembre 2026",
  },
  {
    slug: "suppression-compte",
    titre: "Supprimer votre compte",
    lien: "Supprimer son compte",
    description:
      "Comment supprimer votre compte Weave, ce qui est effacé sous trente jours, et les trois choses que nous sommes tenus de conserver.",
    miseAJour: "12 septembre 2026",
  },
  {
    slug: "mentions-legales",
    titre: "Mentions légales",
    lien: "Mentions légales",
    description: "Qui édite Weave, qui l'héberge, et à qui écrire pour un signalement.",
    miseAJour: "12 septembre 2026",
  },
];

/**
 * La date de la version en vigueur d'un document, par son adresse.
 *
 * Les pages la lisent ici plutôt que de la porter chacune : la liste sert déjà
 * au pied de page et au plan du site, et deux endroits où écrire une date
 * finiraient par en donner deux différentes.
 */
export function dateDuDocument(slug: string): string {
  const document = DOCUMENTS.find((candidat) => candidat.slug === slug);
  if (!document) throw new Error(`Aucun document à l'adresse « ${slug} ».`);
  return document.miseAJour;
}
