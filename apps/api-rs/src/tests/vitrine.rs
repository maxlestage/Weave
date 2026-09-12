//! Le site vitrine, servi par le même processus que l'API.
//!
//! Le `Procfile` n'a qu'un seul dyno web : si ces routes manquaient, l'adresse
//! publique du service rendrait une 404 — et rien, dans l'API, ne le
//! signalerait. C'est exactement ce qui a failli partir en production lors du
//! passage à Rust.

use super::Service;
use axum::http::{header::CACHE_CONTROL, StatusCode};

/// Un `dist` de test : un index et un fichier d'empreinte, comme en produit.
fn dist_de_test() -> std::path::PathBuf {
    static COMPTEUR: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = COMPTEUR.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let racine = std::env::temp_dir().join(format!("weave-dist-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&racine).expect("répertoire de test");
    std::fs::write(racine.join("index.html"), "<!doctype html><title>Weave</title>").unwrap();
    std::fs::write(racine.join("chunk-abc123.js"), "console.log('weave')").unwrap();
    racine
}

#[tokio::test]
async fn la_racine_rend_le_site() {
    let dist = dist_de_test();
    let service = Service::monter_avec(Some(dist.display().to_string())).await;

    let (statut, entetes, corps) = service.get_brut("/").await;
    assert_eq!(statut, StatusCode::OK, "la racine du service est vide");
    assert!(corps.contains("<title>Weave</title>"), "corps rendu : {corps}");
    assert_eq!(
        entetes.get(CACHE_CONTROL).map(|v| v.to_str().unwrap()),
        Some("no-cache"),
        "index.html mis en cache : la publication suivante servirait l'ancien site"
    );
}

#[tokio::test]
async fn les_fichiers_empreints_se_gardent() {
    let dist = dist_de_test();
    let service = Service::monter_avec(Some(dist.display().to_string())).await;

    let (statut, entetes, corps) = service.get_brut("/chunk-abc123.js").await;
    assert_eq!(statut, StatusCode::OK);
    assert!(corps.contains("weave"));
    assert_eq!(
        entetes.get(CACHE_CONTROL).map(|v| v.to_str().unwrap()),
        Some("public, max-age=31536000, immutable")
    );
}

/// Le site passe en recours, jamais devant l'API : c'est l'ordre qui compte.
#[tokio::test]
async fn le_site_ne_masque_pas_l_api() {
    let dist = dist_de_test();
    // Un fichier nommé comme la sonde : s'il passait devant, la sonde de santé
    // d'Heroku rendrait du HTML et le dyno serait déclaré sain à tort.
    std::fs::write(dist.join("health"), "piège").unwrap();
    let service = Service::monter_avec(Some(dist.display().to_string())).await;

    let (statut, corps) = service.get("/health", None).await;
    assert_eq!(statut, StatusCode::OK);
    assert!(
        corps.get("database").is_some(),
        "la sonde a été masquée par le site : {corps}"
    );
}

#[tokio::test]
async fn une_adresse_inconnue_reste_une_404() {
    let dist = dist_de_test();
    let service = Service::monter_avec(Some(dist.display().to_string())).await;

    let (statut, _, _) = service.get_brut("/rien-de-tel").await;
    assert_eq!(statut, StatusCode::NOT_FOUND);
}

/// Sans site construit — le cas du développement — l'API démarre quand même.
#[tokio::test]
async fn un_site_absent_n_empeche_pas_l_api() {
    let service = Service::monter_avec(Some("/chemin/qui/n/existe/pas".to_string())).await;

    let (statut, corps) = service.get("/health", None).await;
    assert_eq!(statut, StatusCode::OK, "l'API refuse de servir sans vitrine : {corps}");
}

/// Une adresse d'API mal orthographiée doit rendre 404, pas 405 : le service
/// de fichiers, seul, répond « méthode interdite » à tout ce qui n'est pas un
/// GET — y compris là où rien n'existe.
#[tokio::test]
async fn un_post_sur_une_adresse_inconnue_rend_404() {
    let dist = dist_de_test();
    let service = Service::monter_avec(Some(dist.display().to_string())).await;

    let (statut, _) = service
        .post("/v1/auth/adresse-qui-n-existe-pas", None, serde_json::json!({}))
        .await;
    assert_eq!(statut, StatusCode::NOT_FOUND);
}
