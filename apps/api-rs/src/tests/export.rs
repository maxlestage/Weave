//! L'export de ses données — ce que la politique de confidentialité promet.
//!
//! Deux choses sont éprouvées ici, et la seconde compte autant que la
//! première : que l'export soit complet, et qu'il ne déborde pas sur les
//! données de quelqu'un d'autre.

use super::Service;
use axum::http::StatusCode;
use serde_json::json;

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

/// L'export rend la photo elle-même, et des listes qui sont des listes.
///
/// Il rendait `photoKey` — un identifiant opaque qui ne désigne rien
/// d'accessible à qui le reçoit. L'article 20 demande un format exploitable :
/// une clé ne l'est pas, et une URL signée ne le serait pas davantage,
/// puisqu'elle expire en quelques minutes alors qu'un export se garde.
///
/// Et les critères rendaient « "[\"femme\"]" » — du JSON dans une chaîne, à
/// relire à la main.
#[tokio::test]
async fn l_export_joint_la_photo_et_rend_de_vraies_listes() {
    let service = Service::monter().await;
    service.compte("c_export_photo", "escapade").await;
    let jeton = service.jeton("c_export_photo");

    let (statut, corps) = service
        .put("/v1/me/profile", Some(&jeton), json!({
            "city": "Rennes",
            "latitude": 48.11,
            "longitude": -1.68,
            "gender": "femme",
        }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    service.consentir(&jeton).await;
    let (statut, corps) = service
        .patch("/v1/me/preferences", Some(&jeton), json!({ "seeking": ["homme", "autre"] }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0];
    jpeg.extend_from_slice(&[7u8; 32]);
    let (statut, corps) = service.put_octets("/v1/me/photo", &jeton, jpeg.clone()).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let (statut, export) = service.get("/v1/me/export", Some(&jeton)).await;
    assert_eq!(statut, StatusCode::OK, "{export}");

    let photo = &export["fiche"]["photo"];
    assert_eq!(photo["typeDeContenu"], "image/jpeg");
    assert_eq!(photo["octets"], jpeg.len());
    let encodee = photo["donneesBase64"].as_str().expect("la photo encodée");
    use base64::{engine::general_purpose::STANDARD, Engine};
    assert_eq!(
        STANDARD.decode(encodee).expect("base64 lisible"),
        jpeg,
        "les octets rendus ne sont pas ceux qui ont été déposés"
    );

    let recherche = export["criteres"]["recherche"]
        .as_array()
        .expect("une liste, pas une chaîne de JSON");
    assert_eq!(recherche.len(), 2);
    assert_eq!(recherche[0], "homme");
}
