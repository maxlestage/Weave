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

export interface Document {
  /** Adresse de la page, sans barre oblique initiale. */
  readonly slug: string;
  /** Titre de l'onglet et du partage. */
  readonly titre: string;
  /** Description de référencement — celle que les moteurs affichent. */
  readonly description: string;
  /** Libellé court, pour le pied de page. */
  readonly lien: string;
}

export const DOCUMENTS: readonly Document[] = [
  {
    slug: "confidentialite",
    titre: "Politique de confidentialité",
    lien: "Confidentialité",
    description:
      "Ce que Weave collecte, pourquoi, combien de temps, et ce que nous nous interdisons. Position arrondie au kilomètre, aucun cookie, aucune revente de données.",
  },
  {
    slug: "cgu",
    titre: "Conditions générales d'utilisation",
    lien: "Conditions d'utilisation",
    description:
      "Ce que Weave propose, ce que nous attendons de vous, et les règles qu'aucun abonnement ne lève.",
  },
  {
    slug: "cgv",
    titre: "Conditions générales de vente",
    lien: "Conditions de vente",
    description:
      "Abonnements et achats à l'unité : prix, reconduction, résiliation, rétractation et remboursement.",
  },
  {
    slug: "suppression-compte",
    titre: "Supprimer votre compte",
    lien: "Supprimer son compte",
    description:
      "Comment supprimer votre compte Weave, ce qui est effacé sous trente jours, et les trois choses que nous sommes tenus de conserver.",
  },
  {
    slug: "mentions-legales",
    titre: "Mentions légales",
    lien: "Mentions légales",
    description: "Qui édite Weave, qui l'héberge, et à qui écrire pour un signalement.",
  },
];
