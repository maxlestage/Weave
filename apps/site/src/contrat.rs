//! Le contrat partagé, ENGENDRÉ. Ne pas modifier à la main.
//!
//! Produit par `apps/site/outils/engendrer-contrat.ts` à partir de
//! `packages/contracts`. Le contrat reste la seule source de vérité : ce
//! fichier n'en est qu'une projection, et un test vérifie qu'il est à jour.
//!
//! `bun run apps/site/outils/engendrer-contrat.ts` pour le régénérer.

#![allow(dead_code)]

use crate::langues::Traduit;

// — Les nombres —————————————————————————————————————————————————

pub const REQUESTS_PER_DAY_FLOOR: u32 = 5;
pub const MAX_OPEN_PLANS: u32 = 3;
pub const PLAN_MIN_LEAD_MINUTES: u32 = 60;
pub const MIN_AGE: u32 = 18;
pub const MESSAGE_RETENTION_DAYS: u32 = 90;
pub const ACCOUNT_PURGE_DAYS: u32 = 30;

pub const POLICY_UPDATED_LABEL: &str = "12 septembre 2026";

// — Les catégories de plan ——————————————————————————————————————

/// Les libellés de catégorie, dans les trois langues.
pub const CATEGORIES: [(&str, Traduit<&str>); 8] = [
    ("sortie", Traduit(["Sortie", "Going out", "Salida"])),
    ("sport", Traduit(["Sport", "Sport", "Deporte"])),
    ("culture", Traduit(["Culture", "Culture", "Cultura"])),
    ("repas", Traduit(["Repas", "Food", "Comida"])),
    ("musique", Traduit(["Musique", "Music", "Música"])),
    ("jeux", Traduit(["Jeux", "Games", "Juegos"])),
    ("balade", Traduit(["Balade", "Walk", "Paseo"])),
    (
        "benevolat",
        Traduit(["Bénévolat", "Volunteering", "Voluntariado"]),
    ),
];

// — Le catalogue ————————————————————————————————————————————————

pub struct Palier {
    pub cle: &'static str,
    pub nom: &'static str,
    pub accroche: &'static str,
    pub prix_centimes: u32,
    /// Le prix mis en forme, par langue. Mis en forme ICI : `Intl` n'existe
    /// pas en Rust, et deux mises en forme différentes donneraient deux prix
    /// affichés différents pour un même nombre de centimes.
    pub prix: Traduit<&'static str>,
    pub atouts: &'static [&'static str],
}

pub const PALIERS: [Palier; 5] = [
    Palier {
        cle: "depart",
        nom: "Départ",
        accroche: "De quoi publier ses plans et demander à venir.",
        prix_centimes: 0,
        prix: Traduit(["0,00 €", "€0.00", "0,00 €"]),
        atouts: &[
            "3 plans ouverts à la fois",
            "5 demandes par jour",
            "Conversations sans limite, une fois la demande acceptée",
        ],
    },
    Palier {
        cle: "viree",
        nom: "Virée",
        accroche: "Pour ceux qui sortent souvent.",
        prix_centimes: 499,
        prix: Traduit(["4,99 €", "€4.99", "4,99 €"]),
        atouts: &[
            "12 demandes par jour",
            "Plans de groupe, jusqu'à quatre",
            "Publication jusqu'à deux semaines à l'avance",
        ],
    },
    Palier {
        cle: "escapade",
        nom: "Escapade",
        accroche: "Des critères qui trient vraiment.",
        prix_centimes: 899,
        prix: Traduit(["8,99 €", "€8.99", "8,99 €"]),
        atouts: &[
            "Critères précis : catégorie, jour, distance fine",
            "Publication jusqu'à un mois à l'avance",
            "1 Escale par mois, pour préparer un départ",
        ],
    },
    Palier {
        cle: "expedition",
        nom: "Expédition",
        accroche: "Organiser loin, et savoir ce qui marche.",
        prix_centimes: 1499,
        prix: Traduit(["14,99 €", "€14.99", "14,99 €"]),
        atouts: &[
            "Publication jusqu'à deux mois à l'avance",
            "Bilan mensuel : quels plans attirent, et pourquoi",
            "2 Escales par mois",
        ],
    },
    Palier {
        cle: "grandtour",
        nom: "Grand Tour",
        accroche: "Tout, sans y penser.",
        prix_centimes: 2499,
        prix: Traduit(["24,99 €", "€24.99", "24,99 €"]),
        atouts: &[
            "Publication jusqu'à trois mois à l'avance",
            "4 Escales par mois",
            "Vérification de profil accélérée et assistance prioritaire",
        ],
    },
];

pub struct ProduitUnite {
    pub sku: &'static str,
    pub nom: &'static str,
    pub description: &'static str,
    pub prix_centimes: u32,
    pub prix: Traduit<&'static str>,
}

pub const PRODUITS_UNITE: [ProduitUnite; 5] = [
    ProduitUnite {
        sku: "renfort",
        nom: "Renfort",
        description: "Cinq demandes de plus aujourd'hui.",
        prix_centimes: 299,
        prix: Traduit(["2,99 €", "€2.99", "2,99 €"]),
    },
    ProduitUnite {
        sku: "horizon",
        nom: "Horizon",
        description: "Publier un plan jusqu'à soixante jours à l'avance, une fois.",
        prix_centimes: 299,
        prix: Traduit(["2,99 €", "€2.99", "2,99 €"]),
    },
    ProduitUnite {
        sku: "tablee",
        nom: "Tablée",
        description: "Un plan de groupe, jusqu'à quatre personnes, une fois.",
        prix_centimes: 299,
        prix: Traduit(["2,99 €", "€2.99", "2,99 €"]),
    },
    ProduitUnite {
        sku: "escale",
        nom: "Escale",
        description: "Publier depuis une autre ville pendant sept jours.",
        prix_centimes: 599,
        prix: Traduit(["5,99 €", "€5.99", "5,99 €"]),
    },
    ProduitUnite {
        sku: "bilan",
        nom: "Bilan",
        description: "Un retour ponctuel sur vos plans : ce qui attire, ce qui tombe à plat.",
        prix_centimes: 499,
        prix: Traduit(["4,99 €", "€4.99", "4,99 €"]),
    },
];
