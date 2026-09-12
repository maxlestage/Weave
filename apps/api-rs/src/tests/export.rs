//! L'export de ses données — ce que la politique de confidentialité promet.
//!
//! Deux choses sont éprouvées ici, et la seconde compte autant que la
//! première : que l'export soit complet, et qu'il ne déborde pas sur les
//! données de quelqu'un d'autre.

use super::Service;
use axum::http::StatusCode;

#[tokio::test]
async fn un_export_demande_une_session() {
    let service = Service::monter().await;
    let (statut, _) = service.get("/v1/me/export", None).await;
    assert_eq!(
        statut,
        StatusCode::UNAUTHORIZED,
        "les données d'un compte ne s'obtiennent pas sans s'y connecter"
    );
}

#[tokio::test]
async fn l_export_contient_le_compte_et_ses_sections() {
    let service = Service::monter().await;
    service.compte("alice", "depart").await;

    let (statut, corps) = service
        .get("/v1/me/export", Some(&service.jeton("alice")))
        .await;

    assert_eq!(statut, StatusCode::OK);
    assert_eq!(corps["format"], "weave.export.v1");
    assert_eq!(corps["compte"]["id"], service.id("alice"));
    assert!(corps["exportedAt"].is_string());

    // Chaque catégorie de données doit être présente, même vide : un tableau
    // absent laisserait croire que le service ne détient rien de ce type.
    for section in [
        "plans",
        "demandesEnvoyees",
        "conversations",
        "messagesEnvoyes",
        "blocages",
        "signalementsEnvoyes",
        "consentements",
        "appareils",
        "abonnements",
        "achats",
        "credits",
    ] {
        assert!(
            corps[section].is_array(),
            "la section « {section} » manque à l'export"
        );
    }
}

/// Le point sensible : un export ne doit jamais servir à lire autrui.
#[tokio::test]
async fn l_export_ne_contient_ni_secret_ni_donnee_d_autrui() {
    let service = Service::monter().await;
    service.compte("alice", "depart").await;
    service.compte("bob", "depart").await;

    let (_, corps) = service
        .get("/v1/me/export", Some(&service.jeton("alice")))
        .await;

    let texte = corps.to_string();
    assert!(
        !texte.contains(&service.id("bob")),
        "l'export d'alice ne doit rien contenir de bob"
    );
    for secret in ["emailHash", "email_hash", "codeHash", "tokenHash"] {
        assert!(
            !texte.contains(secret),
            "l'export ne doit pas rendre l'empreinte « {secret} »"
        );
    }
}
