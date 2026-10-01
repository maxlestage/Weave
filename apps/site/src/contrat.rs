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

/// La finesse des critères de recherche qu'un palier ouvre.
///
/// Une énumération, là où le contrat a une union de chaînes. Le libellé
/// affiché n'est PAS ici : il appartient à la section qui le montre, et il
/// est traduit. Ce qui est ici, c'est le fait — et un `match` dessus ne
/// compile pas s'il oublie un cas, là où une chaîne demanderait un cas par
/// défaut qui afficherait « base » tel quel au visiteur.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Criteres {
    Base,
    Etendus,
    Precis,
}

pub struct Palier {
    pub cle: &'static str,
    pub nom: &'static str,
    pub accroche: Traduit<&'static str>,
    pub prix_centimes: u32,
    /// Le prix mis en forme, par langue. Mis en forme ICI : `Intl` n'existe
    /// pas en Rust, et deux mises en forme différentes donneraient deux prix
    /// affichés différents pour un même nombre de centimes.
    pub prix: Traduit<&'static str>,
    pub atouts: Traduit<&'static [&'static str]>,
    pub demandes_par_jour: u32,
    pub jours_a_l_avance: u32,
    pub criteres: Criteres,
    pub plans_de_groupe: bool,
}

pub const PALIERS: [Palier; 5] = [
    Palier {
        cle: "depart",
        nom: "Départ",
        accroche: Traduit([
            "De quoi publier ses plans et demander à venir.",
            "Enough to post your plans and ask to come along.",
            "Lo justo para publicar tus planes y pedir venir.",
        ]),
        prix_centimes: 0,
        prix: Traduit(["0,00 €", "€0.00", "0,00 €"]),
        atouts: Traduit([
            &[
                "3 plans ouverts à la fois",
                "5 demandes par jour",
                "Conversations sans limite, une fois la demande acceptée",
            ],
            &[
                "3 plans open at a time",
                "5 requests a day",
                "Conversations with no limit, once a request is accepted",
            ],
            &[
                "3 planes abiertos a la vez",
                "5 peticiones al día",
                "Conversaciones sin límite, una vez aceptada la petición",
            ],
        ]),
        demandes_par_jour: 5,
        jours_a_l_avance: 7,
        criteres: Criteres::Base,
        plans_de_groupe: false,
    },
    Palier {
        cle: "viree",
        nom: "Virée",
        accroche: Traduit([
            "Pour ceux qui sortent souvent.",
            "For people who go out often.",
            "Para quien sale a menudo.",
        ]),
        prix_centimes: 499,
        prix: Traduit(["4,99 €", "€4.99", "4,99 €"]),
        atouts: Traduit([
            &[
                "12 demandes par jour",
                "Plans de groupe, jusqu'à quatre",
                "Publication jusqu'à deux semaines à l'avance",
            ],
            &[
                "12 requests a day",
                "Group plans, up to four people",
                "Post up to two weeks ahead",
            ],
            &[
                "12 peticiones al día",
                "Planes de grupo, hasta cuatro personas",
                "Publicar hasta dos semanas antes",
            ],
        ]),
        demandes_par_jour: 12,
        jours_a_l_avance: 14,
        criteres: Criteres::Etendus,
        plans_de_groupe: true,
    },
    Palier {
        cle: "escapade",
        nom: "Escapade",
        accroche: Traduit([
            "Des critères qui trient vraiment.",
            "Filters that actually narrow things down.",
            "Criterios que filtran de verdad.",
        ]),
        prix_centimes: 899,
        prix: Traduit(["8,99 €", "€8.99", "8,99 €"]),
        atouts: Traduit([
            &[
                "Critères précis : catégorie, jour, distance fine",
                "Publication jusqu'à un mois à l'avance",
                "1 Escale par mois, pour préparer un départ",
            ],
            &[
                "Precise filters: category, day, fine-grained distance",
                "Post up to a month ahead",
                "1 Escale a month, to plan a trip",
            ],
            &[
                "Criterios precisos: categoría, día, distancia afinada",
                "Publicar hasta un mes antes",
                "1 Escale al mes, para preparar una salida",
            ],
        ]),
        demandes_par_jour: 25,
        jours_a_l_avance: 30,
        criteres: Criteres::Precis,
        plans_de_groupe: true,
    },
    Palier {
        cle: "expedition",
        nom: "Expédition",
        accroche: Traduit([
            "Organiser loin, et savoir ce qui marche.",
            "Organise far ahead, and learn what works.",
            "Organizar con tiempo, y saber qué funciona.",
        ]),
        prix_centimes: 1499,
        prix: Traduit(["14,99 €", "€14.99", "14,99 €"]),
        atouts: Traduit([
            &[
                "Publication jusqu'à deux mois à l'avance",
                "Bilan mensuel : quels plans attirent, et pourquoi",
                "2 Escales par mois",
            ],
            &[
                "Post up to two months ahead",
                "Monthly Bilan: which plans draw people, and why",
                "2 Escales a month",
            ],
            &[
                "Publicar hasta dos meses antes",
                "Bilan mensual: qué planes atraen, y por qué",
                "2 Escales al mes",
            ],
        ]),
        demandes_par_jour: 40,
        jours_a_l_avance: 60,
        criteres: Criteres::Precis,
        plans_de_groupe: true,
    },
    Palier {
        cle: "grandtour",
        nom: "Grand Tour",
        accroche: Traduit([
            "Tout, sans y penser.",
            "Everything, without thinking about it.",
            "Todo, sin pensarlo.",
        ]),
        prix_centimes: 2499,
        prix: Traduit(["24,99 €", "€24.99", "24,99 €"]),
        atouts: Traduit([
            &[
                "Publication jusqu'à trois mois à l'avance",
                "4 Escales par mois",
                "Vérification de profil accélérée et assistance prioritaire",
            ],
            &[
                "Post up to three months ahead",
                "4 Escales a month",
                "Faster profile verification and priority support",
            ],
            &[
                "Publicar hasta tres meses antes",
                "4 Escales al mes",
                "Verificación de perfil acelerada y asistencia prioritaria",
            ],
        ]),
        demandes_par_jour: 60,
        jours_a_l_avance: 90,
        criteres: Criteres::Precis,
        plans_de_groupe: true,
    },
];

pub struct ProduitUnite {
    pub sku: &'static str,
    pub nom: &'static str,
    pub description: Traduit<&'static str>,
    pub prix_centimes: u32,
    pub prix: Traduit<&'static str>,
}

pub const PRODUITS_UNITE: [ProduitUnite; 5] = [
    ProduitUnite {
        sku: "renfort",
        nom: "Renfort",
        description: Traduit([
            "Cinq demandes de plus aujourd'hui.",
            "Five more requests today.",
            "Cinco peticiones más hoy.",
        ]),
        prix_centimes: 299,
        prix: Traduit(["2,99 €", "€2.99", "2,99 €"]),
    },
    ProduitUnite {
        sku: "horizon",
        nom: "Horizon",
        description: Traduit([
            "Publier un plan jusqu'à soixante jours à l'avance, une fois.",
            "Post a plan up to sixty days ahead, once.",
            "Publicar un plan hasta sesenta días antes, una vez.",
        ]),
        prix_centimes: 299,
        prix: Traduit(["2,99 €", "€2.99", "2,99 €"]),
    },
    ProduitUnite {
        sku: "tablee",
        nom: "Tablée",
        description: Traduit([
            "Un plan de groupe, jusqu'à quatre personnes, une fois.",
            "One group plan, up to four people, once.",
            "Un plan de grupo, hasta cuatro personas, una vez.",
        ]),
        prix_centimes: 299,
        prix: Traduit(["2,99 €", "€2.99", "2,99 €"]),
    },
    ProduitUnite {
        sku: "escale",
        nom: "Escale",
        description: Traduit([
            "Publier depuis une autre ville pendant sept jours.",
            "Post from another town for seven days.",
            "Publicar desde otra ciudad durante siete días.",
        ]),
        prix_centimes: 599,
        prix: Traduit(["5,99 €", "€5.99", "5,99 €"]),
    },
    ProduitUnite {
        sku: "bilan",
        nom: "Bilan",
        description: Traduit([
            "Un retour ponctuel sur vos plans : ce qui attire, ce qui tombe à plat.",
            "A one-off look at your plans: what draws people, what falls flat.",
            "Un repaso puntual de tus planes: qué atrae, qué no cuaja.",
        ]),
        prix_centimes: 499,
        prix: Traduit(["4,99 €", "€4.99", "4,99 €"]),
    },
];
