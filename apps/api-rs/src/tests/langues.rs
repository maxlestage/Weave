//! Les messages du service répondent dans la langue demandée.
//!
//! L'application iOS rend le `message` du serveur tel quel — `WeaveAPI.swift`
//! le dit. Ce qui est vérifié ici n'est donc pas un détail d'en-tête : c'est
//! la phrase qu'un utilisateur anglophone verra à l'écran.

use serde_json::json;

use super::Service;
use crate::langue::Langue;
use crate::messages::Msg;

/// Une erreur arrive en anglais quand on la demande en anglais.
#[tokio::test]
async fn une_erreur_repond_en_anglais() {
    let service = Service::monter().await;

    let (statut, entetes, corps) = service
        .post_dans_la_langue(
            "/v1/auth/otp/request",
            None,
            json!({ "email": "pas-une-adresse" }),
            "en-GB",
        )
        .await;

    assert!(statut.is_client_error(), "statut inattendu : {statut}");
    assert_eq!(corps["error"], "validation");
    assert_eq!(
        corps["message"],
        Msg::AdresseEmailInvalide.dans(Langue::En),
        "le message n'est pas celui de l'anglais : {corps}"
    );

    // `Content-Language` dit au client — et à tout cache entre les deux —
    // dans quelle langue la réponse a été rendue. Sans lui, une réponse
    // française mise en cache serait resservie à qui demandait l'anglais.
    assert_eq!(
        entetes
            .get(axum::http::header::CONTENT_LANGUAGE)
            .and_then(|v| v.to_str().ok()),
        Some("en"),
        "la réponse n'annonce pas sa langue"
    );
}

/// Et en espagnol quand on la demande en espagnol.
#[tokio::test]
async fn une_erreur_repond_en_espagnol() {
    let service = Service::monter().await;

    let (_, entetes, corps) = service
        .post_dans_la_langue(
            "/v1/auth/otp/request",
            None,
            json!({ "email": "pas-une-adresse" }),
            "es-ES,es;q=0.9",
        )
        .await;

    assert_eq!(corps["message"], Msg::AdresseEmailInvalide.dans(Langue::Es));
    assert_eq!(
        entetes
            .get(axum::http::header::CONTENT_LANGUAGE)
            .and_then(|v| v.to_str().ok()),
        Some("es")
    );
}

/// Sans en-tête, le français : c'est la langue du service.
#[tokio::test]
async fn sans_entete_le_service_repond_en_francais() {
    let service = Service::monter().await;

    let (_, corps) = service
        .post(
            "/v1/auth/otp/request",
            None,
            json!({ "email": "pas-une-adresse" }),
        )
        .await;

    assert_eq!(corps["message"], Msg::AdresseEmailInvalide.dans(Langue::Fr));
}

/// Une langue que le service ne parle pas retombe sur le français, et le dit.
///
/// Répondre en allemand n'est pas possible ; répondre en annonçant l'allemand
/// le serait, et un client afficherait une phrase française en la croyant
/// traduite.
#[tokio::test]
async fn une_langue_inconnue_retombe_sur_le_francais_et_l_annonce() {
    let service = Service::monter().await;

    let (_, entetes, corps) = service
        .post_dans_la_langue(
            "/v1/auth/otp/request",
            None,
            json!({ "email": "pas-une-adresse" }),
            "de-DE, it;q=0.8",
        )
        .await;

    assert_eq!(corps["message"], Msg::AdresseEmailInvalide.dans(Langue::Fr));
    assert_eq!(
        entetes
            .get(axum::http::header::CONTENT_LANGUAGE)
            .and_then(|v| v.to_str().ok()),
        Some("fr")
    );
}

/// Aucune phrase du catalogue n'est la même dans deux langues.
///
/// C'est la faute que le compilateur ne voit pas : le `match` exhaustif oblige
/// à écrire les trois branches, il n'oblige pas à les écrire différemment. Une
/// variante ajoutée en copiant la ligne française au-dessus compile, passe
/// tous les tests, et sort en français dans une application anglaise.
///
/// Les variantes à paramètres sont rendues avec des valeurs témoins : c'est la
/// phrase autour qui est comparée, et elle diffère bien d'une langue à
/// l'autre.
#[test]
fn aucune_phrase_n_est_identique_dans_deux_langues() {
    let temoin = |valeur: &str| valeur.to_string();

    let toutes = vec![
        Msg::AuthentificationRequise,
        Msg::SessionExpiree,
        Msg::JetonDeMiseAJourInvalide,
        Msg::AdresseEmailInvalide,
        Msg::CodeIncorrect,
        Msg::CodeExpire,
        Msg::TropDeTentativesSurCeCode,
        Msg::CompteSuspendu,
        Msg::CompteEnSuppression,
        Msg::CompteIntrouvable,
        Msg::DateDeNaissanceInvalide,
        Msg::AgeMinimumRequis { minimum: 18 },
        Msg::AgeHorsBornes {
            nom: temoin("minimum"),
            minimum: 18,
            maximum: 99,
        },
        Msg::AgeMinSuperieurAuMax,
        Msg::NomAfficheTropLong { maximum: 40 },
        Msg::PhraseTropLongue { maximum: 160 },
        Msg::IndiquezUneVille,
        Msg::RenseignezDAbordVotreVille,
        Msg::CoordonneesInvalides,
        Msg::FuseauHoraireInvalide,
        Msg::LangueInvalide,
        Msg::GenreInconnu {
            valeur: temoin("x"),
            acceptees: temoin("a, b"),
        },
        Msg::GenreInconnuSimple {
            valeur: temoin("x"),
        },
        Msg::ProfilDejaVerifie,
        Msg::PlanIntrouvable,
        Msg::PlanDisparu,
        Msg::PlanPasLeVotre,
        Msg::PlanDejaEuLieu,
        Msg::PlanComplet,
        Msg::PlanNAcceptePlusDeDemandes,
        Msg::ToutesLesPlacesSontPrises,
        Msg::VotreProprePlan,
        Msg::TitreLongueur {
            minimum: 8,
            maximum: 80,
        },
        Msg::NoteTropLongue { maximum: 280 },
        Msg::MotTropLong { maximum: 500 },
        Msg::CapaciteHorsBornes {
            minimum: 1,
            maximum: 4,
        },
        Msg::DelaiDePublicationTropCourt { minutes: 60 },
        Msg::HorizonDePublicationDepasse { jours: 7 },
        Msg::DateDeRendezVousIllisible,
        Msg::CategorieInconnue,
        Msg::CategorieInconnueNommee {
            valeur: temoin("x"),
        },
        Msg::HuitCategoriesAuPlus,
        Msg::SeptJoursAuPlus,
        Msg::JourInconnu {
            valeur: temoin("9"),
        },
        Msg::RayonHorsBornes { maximum: 100 },
        Msg::ChoixTropNombreux { maximum: 4 },
        Msg::CriteresIntrouvables,
        Msg::BilanTropPeuDePlans {
            minimum: 3,
            passes: 1,
        },
        Msg::DemandeIntrouvable,
        Msg::DemandePasLaVotre,
        Msg::DemandeDejaRepondue,
        Msg::DemandeDejaTranchee,
        Msg::AdresseDejaUtilisee,
        Msg::PlanNonModifiable,
        Msg::CapaciteSousLesPlacesAccordees { accordees: 2 },
        Msg::PlaceNonRendable,
        Msg::RendezVousDejaPasse,
        Msg::DejaDemandeAVenir,
        Msg::MessageTropCourt { minimum: 20 },
        Msg::MessageDeDemandeTropLong { maximum: 600 },
        Msg::ConversationIntrouvable,
        Msg::ConversationClose,
        Msg::ConversationPasLaVotre,
        Msg::MessageVide,
        Msg::MessageTropLong { maximum: 2000 },
        Msg::MotifDeSignalementInconnu,
        Msg::PrecisionsTropLongues { maximum: 1000 },
        Msg::PasDeAutoBlocage,
        Msg::MediaDisparu,
        Msg::LienMediaExpire,
        Msg::ImageTropLourde,
        Msg::FormatDImageNonReconnu,
        Msg::FlouProgressifNonApplique,
        Msg::AppareilInconnu,
        Msg::IdentifiantDAppareilInvalide,
        Msg::TransactionStoreKitIllisible,
        Msg::TransactionStoreKitRefusee,
        Msg::TransactionIncomplete,
        Msg::ProduitDAbonnementInconnu {
            identifiant: temoin("com.weave.app.sub.viree.monthly"),
        },
        Msg::ProduitALUniteInconnu {
            identifiant: temoin("com.weave.app.unit.renfort"),
        },
        Msg::EscaleDejaEnCours,
        Msg::PauseHorsEtat,
        Msg::VerificationDesAchatsIndisponible,
        Msg::ObjetDeConsentementInconnu,
        Msg::ConsentementSurVersionPerimee,
        Msg::CurseurAvantIso8601,
        Msg::TropDeRenforts { maximum: 2 },
        Msg::LimiteAtteinte {
            quoi: temoin("otp"),
            secondes: 30,
        },
        Msg::ErreurInterne,
    ];

    for message in &toutes {
        let fr = message.dans(Langue::Fr);
        let en = message.dans(Langue::En);
        let es = message.dans(Langue::Es);

        assert!(!fr.is_empty(), "{message:?} : phrase française vide");
        assert_ne!(fr, en, "{message:?} : l'anglais reprend le français");
        assert_ne!(fr, es, "{message:?} : l'espagnol reprend le français");
        assert_ne!(en, es, "{message:?} : l'espagnol reprend l'anglais");
    }
}

/// Le catalogue couvre TOUTES les variantes de `Msg`, et pas seulement celles
/// que quelqu'un a pensé à recopier dans le test ci-dessus.
///
/// Sans ce contrôle, ajouter une variante et oublier de l'ajouter à la liste
/// laisserait le test précédent vert tout en ne vérifiant rien d'elle — c'est
/// exactement le trou qu'on croit avoir bouché.
#[test]
fn la_liste_des_messages_verifies_est_complete() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/messages.rs"),
    )
    .expect("messages.rs lisible");

    let debut = source
        .find("pub enum Msg {")
        .expect("l'énumération a disparu");
    let fin = source[debut..].find("\n}").expect("énumération sans fin") + debut;

    let declarees: Vec<&str> = source[debut..fin]
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with("//") && !l.starts_with("/*") && !l.is_empty())
        .filter_map(|l| {
            let nom = l.split(['{', ',']).next()?.trim();
            let premier = nom.chars().next()?;
            (premier.is_ascii_uppercase() && nom.chars().all(|c| c.is_alphanumeric()))
                .then_some(nom)
        })
        .collect();

    let verifiees = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tests/langues.rs"),
    )
    .expect("ce fichier lisible");

    let oubliees: Vec<&&str> = declarees
        .iter()
        .filter(|nom| !verifiees.contains(&format!("Msg::{nom}")))
        .collect();

    assert!(
        oubliees.is_empty(),
        "ces messages ne sont vérifiés dans aucune langue : {oubliees:?}"
    );
    assert!(
        declarees.len() >= 80,
        "seulement {} variantes lues : l'analyse de l'énumération a dérivé",
        declarees.len()
    );
}

/// L'API dit aux caches que sa réponse dépend de la langue demandée.
///
/// Sans `Vary`, un cache partagé garde la première réponse venue sous
/// l'adresse seule : la première erreur reçue en anglais devient l'erreur de
/// tous les francophones qui suivent. `Content-Language` décrit ce qui a été
/// rendu ; `Vary` dit qu'il aurait pu en être autrement, et c'est celui-là que
/// le cache lit.
#[tokio::test]
async fn une_reponse_d_api_previent_les_caches_qu_elle_depend_de_la_langue() {
    let service = Service::monter().await;

    let (_, entetes, _) = service
        .post_dans_la_langue(
            "/v1/auth/otp/request",
            None,
            json!({ "email": "pas-une-adresse" }),
            "en-GB",
        )
        .await;

    let vary = entetes
        .get(axum::http::header::VARY)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    assert!(
        vary.contains("accept-language"),
        "la réponse ne prévient aucun cache : Vary = « {vary} »"
    );
}
