//! Le code de connexion part vraiment par e-mail.
//!
//! Il ne partait pas : la réponse disait `sent: true`, l'application affichait
//! « un code vient de partir », et aucun courrier n'était envoyé. Ces tests
//! relisent ce qui part, à qui, et dans quelle langue.

use super::Service;
use crate::courriel::EchecEnvoi;
use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn demander_un_code_l_envoie_a_cette_adresse() {
    let service = Service::monter().await;
    let adresse = service.email("ines");

    let (statut, corps) = service
        .post("/v1/auth/otp/request", None, json!({ "email": adresse }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    let code = corps["devCode"].as_str().expect("code rendu hors production");

    let envoyes = service.etat.courriel.traces();
    assert_eq!(envoyes.len(), 1, "un seul courrier par demande : {envoyes:?}");
    assert_eq!(envoyes[0].destinataire, adresse);
    // Le code du courrier est celui que la vérification attend.
    assert!(envoyes[0].sujet.contains(code), "{}", envoyes[0].sujet);
    assert!(envoyes[0].corps.contains(code), "{}", envoyes[0].corps);
}

#[tokio::test]
async fn le_courrier_parle_la_langue_de_la_demande() {
    let service = Service::monter().await;
    let (statut, _, corps) = service
        .post_dans_la_langue(
            "/v1/auth/otp/request",
            None,
            json!({ "email": service.email("jo") }),
            "en-GB,en;q=0.9",
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let envoyes = service.etat.courriel.traces();
    assert!(
        envoyes[0].sujet.starts_with("Your Weave code"),
        "{}",
        envoyes[0].sujet
    );
}

#[tokio::test]
async fn changer_d_adresse_envoie_le_code_a_la_nouvelle() {
    let service = Service::monter().await;
    service.compte("lea", "depart").await;
    let nouvelle = service.email("lea-nouvelle");

    let (statut, corps) = service
        .post(
            "/v1/me/email",
            Some(&service.jeton("lea")),
            json!({ "email": nouvelle }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let envoyes = service.etat.courriel.traces();
    assert_eq!(envoyes.len(), 1, "{envoyes:?}");
    // À la nouvelle adresse : c'est elle dont il faut prouver la possession.
    assert_eq!(envoyes[0].destinataire, nouvelle);
}

#[tokio::test]
async fn sans_serveur_de_courrier_la_production_le_dit() {
    let service = Service::monter().await;
    let mut production = (*service.etat.config).clone();
    production.mode = crate::env::Mode::Production;

    let resultat = service
        .etat
        .courriel
        .envoyer_code(&production, "quelqu-un@exemple.fr", "123456", 10)
        .await;
    assert!(
        matches!(resultat, Err(EchecEnvoi::NonConfigure)),
        "en production, un courrier qui ne peut pas partir est un échec, \
         pas un succès simulé : {resultat:?}"
    );
    assert!(service.etat.courriel.traces().is_empty());
}
