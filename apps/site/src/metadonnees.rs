//! Ce qu'un moteur, un réseau social et un navigateur lisent de l'accueil.
//! ENGENDRÉ. Ne pas modifier à la main.
//!
//! Produit par `apps/site/outils/engendrer-contrat.ts` à partir de
//! `apps/web/src/pages/metadonnees.ts`, qui en reste la source.

use crate::langues::Langue;

pub struct Metadonnees {
    /// Titre de l'onglet et des résultats de recherche.
    pub titre: &'static str,
    /// Description de référencement.
    pub description: &'static str,
    /// Titre du partage — plus court, et sans le nom du site répété.
    pub partage_titre: &'static str,
    pub partage_description: &'static str,
    /// Texte de remplacement de l'image de partage.
    pub partage_image_alt: &'static str,
    /// Étiquette Open Graph : langue ET région, séparées par un tiret bas.
    ///
    /// Ce n'est PAS `fr`/`en`/`es` avec une région devinée : l'anglais du
    /// site est `en_GB`, et ses prix se mettent en forme en `en-IE`. Trois
    /// étiquettes pour trois usages, et aucune ne se déduit des autres.
    pub og_locale: &'static str,
    /// Étiquette BCP 47, pour les données structurées.
    pub bcp47: &'static str,
    /// Description de l'application, dans les données structurées.
    pub application_description: &'static str,
    pub offre_description: &'static str,
}

/// Les métadonnées d'une langue.
pub const fn metadonnees(langue: Langue) -> &'static Metadonnees {
    match langue {
        Langue::Fr => &FR,
        Langue::En => &EN,
        Langue::Es => &ES,
    }
}

const FR: Metadonnees = Metadonnees {
    titre: "Weave — douze fils par jour",
    description: "Weave est une application de rencontre où l'on ne se décrit pas : on publie ce qu'on compte faire, et les autres demandent à venir en disant pourquoi. Pas de cartes à balayer, pas de classement payant.",
    partage_titre: "Weave — des plans, pas des profils",
    partage_description: "On publie ce qu'on compte faire jeudi soir. Les autres demandent à venir — en disant pourquoi.",
    partage_image_alt: "Weave — des plans, pas des profils. Six fils de couleur tissés.",
    og_locale: "fr_FR",
    bcp47: "fr-FR",
    application_description: "Application de rencontre fondée sur les plans plutôt que sur les profils : on publie ce qu'on compte faire, les autres demandent à venir en écrivant pourquoi.",
    offre_description: "Gratuit, avec abonnements et achats à l'unité facultatifs.",
};

const EN: Metadonnees = Metadonnees {
    titre: "Weave — plans, not profiles",
    description: "Weave is a dating app where you don't describe yourself: you post what you're going to do, and other people ask to come along, saying why. No cards to swipe, no paid ranking.",
    partage_titre: "Weave — plans, not profiles",
    partage_description: "You post what you're doing on Thursday evening. Other people ask to come — and say why.",
    partage_image_alt: "Weave — plans, not profiles. Six coloured threads, woven.",
    og_locale: "en_GB",
    bcp47: "en-GB",
    application_description: "A dating app built on plans rather than profiles: you post what you're going to do, and other people ask to come along by writing why.",
    offre_description: "Free, with optional subscriptions and single purchases.",
};

const ES: Metadonnees = Metadonnees {
    titre: "Weave — planes, no perfiles",
    description: "Weave es una aplicación de citas en la que no te describes: publicas lo que piensas hacer, y los demás piden venir diciendo por qué. Sin tarjetas que deslizar, sin posiciones de pago.",
    partage_titre: "Weave — planes, no perfiles",
    partage_description: "Publicas lo que piensas hacer el jueves por la noche. Los demás piden venir — y dicen por qué.",
    partage_image_alt: "Weave — planes, no perfiles. Seis hilos de color, tejidos.",
    og_locale: "es_ES",
    bcp47: "es-ES",
    application_description: "Aplicación de citas basada en los planes y no en los perfiles: publicas lo que piensas hacer, y los demás piden venir escribiendo por qué.",
    offre_description: "Gratis, con suscripciones y compras por unidades opcionales.",
};
