//! La langue de la réponse, lue dans la requête.
//!
//! ## Pourquoi le service traduit
//!
//! Les messages d'erreur de ce service sont rendus TELS QUELS par
//! l'application iOS — `WeaveAPI.swift` le dit : « `message` est celui du
//! serveur, et il est rendu tel quel ». Une application en anglais afficherait
//! donc « Ce plan est complet. » au milieu de son propre texte.
//!
//! Les codes d'erreur, eux, sont stables et partagés (`@weave/contracts`),
//! mais treize codes ne distinguent pas quatre-vingts situations : « Ce plan
//! est complet. » et « Ce plan a déjà eu lieu. » portent le même
//! `plan_closed`. C'est la phrase qui dit ce qui s'est passé, et c'est donc
//! elle qu'il faut traduire.
//!
//! ## Pourquoi une variable de tâche
//!
//! La langue est un fait de la requête, et les messages se construisent au
//! fond des routes — souvent à dix appels de profondeur, dans des fonctions
//! qui n'ont aucune raison de connaître un en-tête HTTP. La passer en
//! paramètre aurait traversé tout le service pour n'être lue qu'aux feuilles.
//!
//! `tokio::task_local!` la porte pour la durée de la requête, et seulement
//! elle : chaque requête est une tâche, et deux requêtes simultanées ne
//! partagent rien. Hors requête — une migration, une purge, la console — la
//! variable n'est pas posée et le français s'applique.

use axum::{extract::Request, middleware::Next, response::Response};

/// Les langues que ce service sait parler. Ce sont celles du site :
/// `apps/web/src/langues.ts`, et un test le vérifie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Langue {
    #[default]
    Fr,
    En,
    Es,
}

impl Langue {
    /// L'étiquette BCP 47 courte, celle que porte `Content-Language`.
    pub fn etiquette(self) -> &'static str {
        match self {
            Langue::Fr => "fr",
            Langue::En => "en",
            Langue::Es => "es",
        }
    }

    /// Reconnaît une étiquette de langue, sous-étiquettes comprises : « en »,
    /// « en-GB » et « EN_gb » désignent tous l'anglais.
    fn depuis_etiquette(etiquette: &str) -> Option<Langue> {
        let base = etiquette
            .split(['-', '_'])
            .next()
            .unwrap_or(etiquette)
            .trim()
            .to_ascii_lowercase();
        match base.as_str() {
            "fr" => Some(Langue::Fr),
            "en" => Some(Langue::En),
            "es" => Some(Langue::Es),
            _ => None,
        }
    }
}

tokio::task_local! {
    static LANGUE: Langue;
}

/// La langue de la requête en cours, ou le français hors requête.
///
/// Ne jamais faire échouer une réponse faute de langue : un message dans la
/// mauvaise langue reste un message, une erreur à sa place n'en est plus une.
pub fn courante() -> Langue {
    LANGUE.try_with(|langue| *langue).unwrap_or_default()
}

/// Choisit la langue d'un en-tête `Accept-Language`.
///
/// Le format est celui de la norme : une liste de préférences, chacune
/// pouvant porter un poids `;q=`. Les poids sont RESPECTÉS et non ignorés —
/// « fr;q=0.2, en;q=0.9 » demande l'anglais, et prendre la première venue
/// aurait répondu en français à quelqu'un qui a dit préférer l'anglais.
///
/// Une préférence de poids nul est un REFUS explicite de cette langue : la
/// norme le prévoit, et la retenir dirait le contraire de ce qui est demandé.
///
/// Une langue inconnue est ignorée plutôt que de faire échouer l'en-tête
/// entier : « de, en » doit encore donner l'anglais.
pub fn depuis_l_entete(entete: &str) -> Langue {
    let mut meilleure: Option<(Langue, f32)> = None;

    for preference in entete.split(',') {
        let mut morceaux = preference.split(';');
        let Some(etiquette) = morceaux.next().map(str::trim) else {
            continue;
        };

        // `*` dit « n'importe laquelle » : la nôtre est le français.
        let langue = if etiquette == "*" {
            Langue::Fr
        } else {
            match Langue::depuis_etiquette(etiquette) {
                Some(langue) => langue,
                None => continue,
            }
        };

        // Sans `q=`, le poids vaut 1 — c'est la norme.
        let poids = morceaux
            .find_map(|parametre| {
                let parametre = parametre.trim();
                parametre
                    .strip_prefix("q=")
                    .and_then(|v| v.parse::<f32>().ok())
            })
            .unwrap_or(1.0);

        if poids <= 0.0 {
            continue;
        }

        // À poids égal, la première l'emporte : la norme ne départage pas, et
        // l'ordre d'écriture est la seule intention exprimée.
        if meilleure.is_none_or(|(_, meilleur_poids)| poids > meilleur_poids) {
            meilleure = Some((langue, poids));
        }
    }

    meilleure.map(|(langue, _)| langue).unwrap_or_default()
}

/// Pose la langue de la requête pour la durée de celle-ci.
///
/// Cette couche NE POSE AUCUN EN-TÊTE, et c'est délibéré. Elle enveloppe tout
/// le service, site vitrine compris ; or une page traduite ne se rend pas dans
/// la langue demandée, elle se rend dans la sienne. Annoncer ici la langue de
/// la requête étiquetait « /en/ » comme française dès qu'un francophone
/// l'ouvrait, et la page juridique française comme anglaise dès qu'un
/// anglophone la demandait.
///
/// L'annonce revient donc à qui sait ce qui a été rendu : `annoncer_la_langue`
/// pour l'API, qui compose vraiment sa réponse dans cette langue, et le
/// service de fichiers pour les pages, qui la lisent dans leur adresse.
pub async fn poser_la_langue(requete: Request, suite: Next) -> Response {
    let langue = requete
        .headers()
        .get(axum::http::header::ACCEPT_LANGUAGE)
        .and_then(|valeur| valeur.to_str().ok())
        .map(depuis_l_entete)
        .unwrap_or_default();

    LANGUE.scope(langue, suite.run(requete)).await
}

/// Annonce sur la réponse la langue dans laquelle l'API l'a composée.
///
/// `Content-Language` n'est pas décoratif : il dit au client — et à tout cache
/// intermédiaire — dans quelle langue la réponse a été rendue. Sans lui, une
/// erreur française mise en cache serait resservie à qui demandait l'anglais.
///
/// Il n'a de sens que là où la réponse SUIT la demande, c'est-à-dire sur
/// l'API : ses messages sont composés dans la langue négociée. Une page du
/// site, elle, existe dans une langue avant qu'on la demande.
pub async fn annoncer_la_langue(requete: Request, suite: Next) -> Response {
    let mut reponse = suite.run(requete).await;
    poser_l_entete(&mut reponse, courante());

    // `Vary` dit aux caches que cette réponse DÉPEND de l'en-tête de langue.
    //
    // Sans lui, un cache partagé — un proxy d'entreprise, un CDN — garde la
    // première réponse venue sous l'adresse seule et la ressert à tout le
    // monde : la première erreur reçue en anglais devient l'erreur de tous les
    // francophones qui suivent. `Content-Language` décrit la réponse ; `Vary`
    // dit qu'il aurait pu en être autrement.
    //
    // Le site, lui, n'en a pas besoin : ses pages tiennent leur langue de leur
    // adresse, et « /en/ » est anglaise pour tout le monde. C'est ce qui les
    // garde cachables telles quelles.
    reponse.headers_mut().insert(
        axum::http::header::VARY,
        axum::http::HeaderValue::from_static("accept-language"),
    );
    reponse
}

/// La langue d'une page du site, lue dans son adresse.
///
/// C'est la construction qui décide : « /en/… » et « /es/… » sont les accueils
/// traduits, tout le reste — la racine et les pages juridiques — est français.
pub fn langue_du_chemin(chemin: &str) -> Langue {
    let premier = chemin.split('/').find(|morceau| !morceau.is_empty());
    premier
        .and_then(Langue::depuis_etiquette)
        .unwrap_or_default()
}

/// Pose `Content-Language`, si l'étiquette est écrivable en en-tête.
///
/// Un échec n'est jamais fatal : une réponse sans cet en-tête reste une
/// réponse, une erreur à sa place n'en est plus une.
pub fn poser_l_entete(reponse: &mut Response, langue: Langue) {
    if let Ok(valeur) = axum::http::HeaderValue::from_str(langue.etiquette()) {
        reponse
            .headers_mut()
            .insert(axum::http::header::CONTENT_LANGUAGE, valeur);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_langue_simple_est_reconnue() {
        assert_eq!(depuis_l_entete("en"), Langue::En);
        assert_eq!(depuis_l_entete("es"), Langue::Es);
        assert_eq!(depuis_l_entete("fr"), Langue::Fr);
    }

    /// « en-GB » et « en_gb » désignent l'anglais : c'est la sous-étiquette de
    /// région qui change, pas la langue. Un iPhone britannique envoie « en-GB »
    /// et un iPhone américain « en-US » — refuser l'un des deux répondrait en
    /// français à la moitié des anglophones.
    #[test]
    fn les_sous_etiquettes_de_region_ne_changent_pas_la_langue() {
        for etiquette in ["en-GB", "en-US", "EN_gb", "en-Latn-GB"] {
            assert_eq!(depuis_l_entete(etiquette), Langue::En, "« {etiquette} »");
        }
        assert_eq!(depuis_l_entete("es-419"), Langue::Es);
        assert_eq!(depuis_l_entete("fr-CA"), Langue::Fr);
    }

    /// Les poids sont respectés, et non l'ordre d'écriture.
    ///
    /// Prendre la première venue aurait répondu en français à « fr;q=0.2,
    /// en;q=0.9 » — c'est-à-dire à quelqu'un qui vient de dire préférer
    /// l'anglais.
    #[test]
    fn le_poids_l_emporte_sur_l_ordre() {
        assert_eq!(depuis_l_entete("fr;q=0.2, en;q=0.9"), Langue::En);
        assert_eq!(depuis_l_entete("en;q=0.3, es;q=0.8"), Langue::Es);
        // Sans poids, la valeur est 1 : elle bat un 0,9 écrit avant elle.
        assert_eq!(depuis_l_entete("es;q=0.9, en"), Langue::En);
    }

    /// Un poids nul est un refus explicite, et la norme le dit. Le retenir
    /// dirait le contraire de ce qui est demandé.
    #[test]
    fn un_poids_nul_est_un_refus() {
        assert_eq!(depuis_l_entete("en;q=0"), Langue::Fr);
        assert_eq!(depuis_l_entete("en;q=0, es;q=0.5"), Langue::Es);
    }

    /// Une langue inconnue est ignorée, pas fatale : « de, en » doit encore
    /// donner l'anglais. Rejeter l'en-tête entier ferait répondre en français
    /// à un iPhone allemand qui accepte aussi l'anglais.
    #[test]
    fn une_langue_inconnue_n_annule_pas_les_autres() {
        assert_eq!(depuis_l_entete("de, en"), Langue::En);
        assert_eq!(depuis_l_entete("de;q=0.9, es;q=0.4"), Langue::Es);
        assert_eq!(depuis_l_entete("de, it, ja"), Langue::Fr);
    }

    /// Rien de lisible : le français, qui est la langue du service.
    #[test]
    fn a_defaut_le_francais() {
        for entete in ["", "   ", ",,,", "q=0.9", "en;q=beaucoup"] {
            // « en;q=beaucoup » : le poids illisible retombe à 1, et l'anglais
            // reste demandé — c'est l'étiquette qui porte l'intention.
            let attendue = if entete == "en;q=beaucoup" {
                Langue::En
            } else {
                Langue::Fr
            };
            assert_eq!(depuis_l_entete(entete), attendue, "« {entete} »");
        }
    }

    /// Hors requête — migration, purge, console — le français s'applique sans
    /// que rien n'échoue.
    #[test]
    fn hors_requete_le_francais_s_applique() {
        assert_eq!(courante(), Langue::Fr);
    }

    /// Les langues du service sont celles du site.
    ///
    /// Le site et l'application doivent proposer les mêmes : une langue
    /// traduite d'un seul côté donne une page traduite dont chaque message
    /// d'erreur est en français, ou l'inverse.
    #[test]
    fn les_langues_sont_celles_du_site() {
        let chemin =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/web/src/langues.ts");
        let source = std::fs::read_to_string(&chemin)
            .unwrap_or_else(|e| panic!("langues.ts illisible en {} : {e}", chemin.display()));

        for langue in [Langue::Fr, Langue::En, Langue::Es] {
            assert!(
                source.contains(&format!("\"{}\"", langue.etiquette())),
                "« {} » a disparu de langues.ts",
                langue.etiquette()
            );
        }
    }
}
