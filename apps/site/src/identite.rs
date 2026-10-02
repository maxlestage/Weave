//! L'identité de l'éditeur. ENGENDRÉ. Ne pas modifier à la main.
//!
//! Produit par `apps/site/outils/engendrer-contrat.ts` à partir de
//! `apps/web/src/pages/identite.ts`, qui reste LE SEUL FICHIER À COMPLÉTER.
//!
//! Rien n'est inventé. Une mention légale qui annoncerait une raison sociale
//! ou un numéro d'immatriculation plausibles mais faux serait une fausse
//! déclaration — et personne ne s'apercevrait qu'elle est fausse, puisqu'elle
//! aurait l'air complète. Les valeurs manquantes sont donc des libellés
//! explicites, surlignés à l'écran, et `est_a_completer` les reconnaît.

/// Vrai si la valeur n'a pas encore été renseignée.
///
/// Le site REFUSE de se construire pour un vrai domaine tant qu'il en reste
/// une : c'est le même critère que du côté React, et il tient à un préfixe.
pub fn est_a_completer(valeur: &str) -> bool {
    valeur.starts_with("[à compléter")
}

pub struct Hebergeur {
    pub nom: &'static str,
    pub adresse: &'static str,
    pub telephone: &'static str,
    /// Ex. « dans l'Union européenne (Irlande) » — sert aussi aux transferts.
    pub region: &'static str,
}

pub struct Editeur {
    /// Dénomination sociale, ou nom et prénom pour une personne physique.
    pub raison_sociale: &'static str,
    pub forme_juridique: &'static str,
    pub adresse: &'static str,
    /// SIREN ou SIRET, et numéro RCS avec sa ville d'immatriculation.
    pub immatriculation: &'static str,
    pub tva: &'static str,
    /// La personne physique responsable du contenu.
    pub directeur_publication: &'static str,
    /// La LCEN exige un moyen de contact direct et effectif.
    pub telephone: &'static str,
    pub hebergeur: Hebergeur,
}

pub struct Contact {
    /// Exercice des droits RGPD et questions sur les données.
    pub confidentialite: &'static str,
    /// Assistance, résiliation, réclamations.
    pub support: &'static str,
    /// Point de contact unique exigé par le règlement sur les services
    /// numériques.
    pub signalements: &'static str,
}

pub const EDITEUR: Editeur = Editeur {
    raison_sociale: "[à compléter : raison sociale]",
    forme_juridique: "[à compléter : forme juridique et capital social]",
    adresse: "[à compléter : adresse du siège social]",
    immatriculation: "[à compléter : SIREN et RCS]",
    tva: "[à compléter : numéro de TVA intracommunautaire]",
    directeur_publication: "[à compléter : directeur de la publication]",
    telephone: "[à compléter : numéro de téléphone]",
    hebergeur: Hebergeur {
        nom: "[à compléter : nom de l'hébergeur]",
        adresse: "[à compléter : adresse de l'hébergeur]",
        telephone: "[à compléter : téléphone de l'hébergeur]",
        region: "[à compléter : région d'hébergement]",
    },
};

pub const CONTACT: Contact = Contact {
    confidentialite: "[à compléter : adresse de contact données personnelles]",
    support: "[à compléter : adresse de contact assistance]",
    signalements: "[à compléter : adresse de contact signalements]",
};

/// Tout ce qui se renseigne dans `identite.ts`, à plat : le chemin du champ
/// et sa valeur actuelle.
///
/// Les noms sont ceux du fichier TypeScript, et non leur transcription en
/// Rust : c'est là que l'éditeur doit aller les remplir.
pub const VALEURS_A_RENSEIGNER: [(&str, &str); 15] = [
    ("EDITEUR.raisonSociale", "[à compléter : raison sociale]"),
    (
        "EDITEUR.formeJuridique",
        "[à compléter : forme juridique et capital social]",
    ),
    ("EDITEUR.adresse", "[à compléter : adresse du siège social]"),
    ("EDITEUR.immatriculation", "[à compléter : SIREN et RCS]"),
    (
        "EDITEUR.tva",
        "[à compléter : numéro de TVA intracommunautaire]",
    ),
    (
        "EDITEUR.directeurPublication",
        "[à compléter : directeur de la publication]",
    ),
    ("EDITEUR.telephone", "[à compléter : numéro de téléphone]"),
    (
        "EDITEUR.hebergeur.nom",
        "[à compléter : nom de l'hébergeur]",
    ),
    (
        "EDITEUR.hebergeur.adresse",
        "[à compléter : adresse de l'hébergeur]",
    ),
    (
        "EDITEUR.hebergeur.telephone",
        "[à compléter : téléphone de l'hébergeur]",
    ),
    (
        "EDITEUR.hebergeur.region",
        "[à compléter : région d'hébergement]",
    ),
    (
        "CONTACT.confidentialite",
        "[à compléter : adresse de contact données personnelles]",
    ),
    (
        "CONTACT.support",
        "[à compléter : adresse de contact assistance]",
    ),
    (
        "CONTACT.signalements",
        "[à compléter : adresse de contact signalements]",
    ),
    (
        "SITE.origine",
        "[à compléter : adresse publique du site, ex. https://weave.app]",
    ),
];
