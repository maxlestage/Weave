//! Le site vitrine, servi par le même processus que l'API.
//!
//! Le `Procfile` n'a qu'un seul dyno web : si ces routes manquaient, l'adresse
//! publique du service rendrait une 404 — et rien, dans l'API, ne le
//! signalerait. C'est exactement ce qui a failli partir en production lors du
//! passage à Rust.

use super::Service;
use axum::http::{StatusCode, header::CACHE_CONTROL};

/// Un `dist` de test : un index et un fichier d'empreinte, comme en produit.
fn dist_de_test() -> std::path::PathBuf {
    static COMPTEUR: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = COMPTEUR.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let racine = std::env::temp_dir().join(format!("weave-dist-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&racine).expect("répertoire de test");
    std::fs::write(
        racine.join("index.html"),
        "<!doctype html><title>Weave</title>",
    )
    .unwrap();
    std::fs::write(racine.join("chunk-abc123.js"), "console.log('weave')").unwrap();
    // Les quatre adresses fixes : elles ne portent pas d'empreinte, parce
    // qu'elles sont demandées depuis l'extérieur.
    std::fs::write(racine.join("partage.png"), b"\x89PNG\r\n\x1a\n").unwrap();
    std::fs::write(racine.join("apple-touch-icon.png"), b"\x89PNG\r\n\x1a\n").unwrap();
    std::fs::write(racine.join("favicon.svg"), "<svg/>").unwrap();
    std::fs::write(racine.join("site.webmanifest"), "{}").unwrap();
    // Un nom composé, sans empreinte : c'est ce qu'un mot français assez long
    // ferait passer pour une empreinte si l'on ne demandait pas de chiffre.
    std::fs::write(racine.join("photo-couverture.png"), b"\x89PNG\r\n\x1a\n").unwrap();
    // Une page juridique, construite comme en produit : un répertoire portant
    // le nom de l'adresse, et l'index dedans.
    std::fs::create_dir_all(racine.join("cgv")).unwrap();
    std::fs::write(
        racine.join("cgv/index.html"),
        "<!doctype html><title>Conditions de vente</title>",
    )
    .unwrap();
    racine
}

#[tokio::test]
async fn la_racine_rend_le_site() {
    let dist = dist_de_test();
    let service = Service::monter_avec(Some(dist.display().to_string())).await;

    let (statut, entetes, corps) = service.get_brut("/").await;
    assert_eq!(statut, StatusCode::OK, "la racine du service est vide");
    assert!(
        corps.contains("<title>Weave</title>"),
        "corps rendu : {corps}"
    );
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
    assert_eq!(
        statut,
        StatusCode::OK,
        "l'API refuse de servir sans vitrine : {corps}"
    );
}

/// Une adresse d'API mal orthographiée doit rendre 404, pas 405 : le service
/// de fichiers, seul, répond « méthode interdite » à tout ce qui n'est pas un
/// GET — y compris là où rien n'existe.
#[tokio::test]
async fn un_post_sur_une_adresse_inconnue_rend_404() {
    let dist = dist_de_test();
    let service = Service::monter_avec(Some(dist.display().to_string())).await;

    let (statut, _) = service
        .post(
            "/v1/auth/adresse-qui-n-existe-pas",
            None,
            serde_json::json!({}),
        )
        .await;
    assert_eq!(statut, StatusCode::NOT_FOUND);
}

/// Une page juridique répond à son adresse, sans redirection.
///
/// Chaque page est construite en `cgv/index.html`. Le service de fichiers y
/// voyait un répertoire et renvoyait une 307 vers « /cgv/ » — or « /cgv » est
/// l'adresse que met en lien le pied de page, que déclare le plan du site, et
/// que désigne l'URL canonique. L'adresse annoncée aux moteurs n'était donc
/// pas celle qui répondait.
#[tokio::test]
async fn une_page_juridique_repond_a_son_adresse_sans_redirection() {
    let dist = dist_de_test();
    let service = Service::monter_avec(Some(dist.display().to_string())).await;

    let (statut, _, corps) = service.get_brut("/cgv").await;
    assert_eq!(
        statut,
        StatusCode::OK,
        "« /cgv » devait rendre la page, pas une redirection"
    );
    assert!(
        corps.contains("Conditions de vente"),
        "corps rendu : {corps}"
    );
}

/// Une page rendue sans barre oblique ni extension reste du HTML.
///
/// Le cache se décidait sur la forme de l'adresse : « /cgv » n'ayant ni barre
/// oblique finale ni extension, la page aurait été déclarée immuable pour un
/// an. Un texte juridique corrigé serait resté invisible tout ce temps —
/// et c'est le genre de correction qui a une date d'effet.
#[tokio::test]
async fn une_page_juridique_ne_se_garde_pas_un_an() {
    let dist = dist_de_test();
    let service = Service::monter_avec(Some(dist.display().to_string())).await;

    let (statut, entetes, _) = service.get_brut("/cgv").await;
    assert_eq!(statut, StatusCode::OK);
    assert_eq!(
        entetes.get(CACHE_CONTROL).map(|v| v.to_str().unwrap()),
        Some("no-cache"),
        "une page juridique gardée un an ne se corrige plus"
    );
}

/// La barre oblique ajoutée ne doit pas inventer de pages.
#[tokio::test]
async fn une_adresse_sans_extension_et_sans_page_reste_une_404() {
    let dist = dist_de_test();
    let service = Service::monter_avec(Some(dist.display().to_string())).await;

    let (statut, _, _) = service.get_brut("/cgv-qui-n-existe-pas").await;
    assert_eq!(statut, StatusCode::NOT_FOUND);
}

/// Une adresse fixe ne se garde pas un an.
///
/// Quatre fichiers vivent à une adresse que l'on ne peut pas changer, parce
/// que c'est ailleurs qu'elle est écrite : `partage.png` chez les réseaux
/// sociaux, `apple-touch-icon.png` et `site.webmanifest` sur l'écran
/// d'accueil, `favicon.svg` chez tous les navigateurs et la moitié des robots.
///
/// Ils prenaient « un an, immuable » comme les fichiers empreints. Changer
/// l'image de partage n'aurait rien changé pour personne pendant un an, et
/// aucun moyen de forcer : l'adresse ne peut pas bouger, c'est tout l'intérêt.
#[tokio::test]
async fn les_adresses_fixes_ne_se_gardent_pas_un_an() {
    let dist = dist_de_test();
    let service = Service::monter_avec(Some(dist.display().to_string())).await;

    for adresse in [
        "/partage.png",
        "/apple-touch-icon.png",
        "/favicon.svg",
        "/site.webmanifest",
        // Un nom composé n'est pas une empreinte : sans le chiffre exigé,
        // « couverture » en aurait la longueur et l'alphabet, et le fichier
        // serait figé un an sur une adresse qu'on ne peut pas changer.
        "/photo-couverture.png",
    ] {
        let (statut, entetes, _) = service.get_brut(adresse).await;
        assert_eq!(statut, StatusCode::OK, "« {adresse} » n'est pas servie");
        let cache = entetes
            .get(CACHE_CONTROL)
            .map(|v| v.to_str().unwrap())
            .unwrap_or_default();
        assert!(
            !cache.contains("immutable"),
            "« {adresse} » est figée un an alors que son adresse ne peut pas changer : {cache}"
        );
        assert!(
            cache.contains("max-age"),
            "« {adresse} » sans durée : {cache}"
        );
    }

    // Un fichier empreint, lui, se garde : son nom change avec son contenu.
    let (_, entetes, _) = service.get_brut("/chunk-abc123.js").await;
    assert_eq!(
        entetes.get(CACHE_CONTROL).map(|v| v.to_str().unwrap()),
        Some("public, max-age=31536000, immutable"),
        "un fichier empreint doit se garder indéfiniment"
    );
}
