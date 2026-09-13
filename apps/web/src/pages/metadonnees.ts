/*
 * Ce qu'un moteur, un réseau social et un navigateur lisent de l'accueil —
 * dans les trois langues.
 *
 * Ces textes vivaient dans `index.html`, en français et une seule fois. Il
 * n'y avait alors qu'une page d'accueil ; il y en a trois, et les trois
 * s'indexent. Un titre et une description français sur `/en/` feraient
 * indexer la page anglaise comme française : le visiteur anglophone verrait
 * un résultat de recherche qu'il ne comprend pas, et cliquerait ailleurs.
 *
 * `index.html` ne porte donc plus rien de tout cela : il ne garde que ce dont
 * le bundler a besoin pour trouver le script et la feuille de style. La
 * construction écrit la tête des trois pages à partir d'ici.
 */

import type { Langue } from "../langues.ts";

export interface Metadonnees {
  /** Titre de l'onglet et des résultats de recherche. */
  readonly titre: string;
  /** Description de référencement. */
  readonly description: string;
  /** Titre du partage — plus court, et sans le nom du site répété. */
  readonly partageTitre: string;
  readonly partageDescription: string;
  /** Texte de remplacement de l'image de partage. */
  readonly partageImageAlt: string;
  /** Étiquette Open Graph : langue ET région, séparées par un tiret bas. */
  readonly ogLocale: string;
  /** Étiquette BCP 47, pour les données structurées. */
  readonly bcp47: string;
  /** Description de l'application, dans les données structurées. */
  readonly applicationDescription: string;
  readonly offreDescription: string;
}

export const METADONNEES: Readonly<Record<Langue, Metadonnees>> = {
  fr: {
    titre: "Weave — douze fils par jour",
    description:
      "Weave est une application de rencontre où l'on ne se décrit pas : on publie ce qu'on compte faire, et les autres demandent à venir en disant pourquoi. Pas de cartes à balayer, pas de classement payant.",
    partageTitre: "Weave — des plans, pas des profils",
    partageDescription:
      "On publie ce qu'on compte faire jeudi soir. Les autres demandent à venir — en disant pourquoi.",
    partageImageAlt: "Weave — des plans, pas des profils. Six fils de couleur tissés.",
    ogLocale: "fr_FR",
    bcp47: "fr-FR",
    applicationDescription:
      "Application de rencontre fondée sur les plans plutôt que sur les profils : on publie ce qu'on compte faire, les autres demandent à venir en écrivant pourquoi.",
    offreDescription: "Gratuit, avec abonnements et achats à l'unité facultatifs.",
  },
  en: {
    titre: "Weave — plans, not profiles",
    description:
      "Weave is a dating app where you don't describe yourself: you post what you're going to do, and other people ask to come along, saying why. No cards to swipe, no paid ranking.",
    partageTitre: "Weave — plans, not profiles",
    partageDescription:
      "You post what you're doing on Thursday evening. Other people ask to come — and say why.",
    partageImageAlt: "Weave — plans, not profiles. Six coloured threads, woven.",
    ogLocale: "en_GB",
    bcp47: "en-GB",
    applicationDescription:
      "A dating app built on plans rather than profiles: you post what you're going to do, and other people ask to come along by writing why.",
    offreDescription: "Free, with optional subscriptions and single purchases.",
  },
  es: {
    titre: "Weave — planes, no perfiles",
    description:
      "Weave es una aplicación de citas en la que no te describes: publicas lo que piensas hacer, y los demás piden venir diciendo por qué. Sin tarjetas que deslizar, sin posiciones de pago.",
    partageTitre: "Weave — planes, no perfiles",
    partageDescription:
      "Publicas lo que piensas hacer el jueves por la noche. Los demás piden venir — y dicen por qué.",
    partageImageAlt: "Weave — planes, no perfiles. Seis hilos de color, tejidos.",
    ogLocale: "es_ES",
    bcp47: "es-ES",
    applicationDescription:
      "Aplicación de citas basada en los planes y no en los perfiles: publicas lo que piensas hacer, y los demás piden venir escribiendo por qué.",
    offreDescription: "Gratis, con suscripciones y compras por unidades opcionales.",
  },
};
