//! Bloquer, puis débloquer — et ce que le fil en montre.
//!
//! Le blocage est le geste de protection du produit : il coupe des deux côtés,
//! sans prévenir l'autre. Aucun test ne l'exerçait par HTTP, alors qu'il
//! touche trois sous-systèmes à la fois — le fil, les demandes et les
//! conversations — et que son effet passe par un cache.
//!
//! Le cache est précisément ce qui rend ces tests nécessaires. Le fil vit
//! quelques minutes sous la clé du compte : une route qui pose la bonne ligne
//! en base mais oublie d'invalider le fil de l'autre n'a aucun effet visible
//! pour lui pendant tout ce temps. C'est pourquoi les lectures ci-dessous
//! n'oublient jamais le cache elles-mêmes : c'est à la route de le faire.

use super::Service;
use axum::http::StatusCode;
use serde_json::{Value, json};

#[tokio::test]
async fn bloquer_retire_les_plans_des_deux_fils_sans_attendre_le_cache() {
    let service = Service::monter().await;
    let bruno = deux_voisins(&service).await;

    let plan_d_anne = plan_de(&service, "anne").await;
    let plan_de_bruno = plan_de(&service, "bruno").await;

    // Chacun voit l'autre, et chaque fil est désormais en cache.
    assert!(fil_contient(&service, "bruno", &plan_d_anne).await);
    assert!(fil_contient(&service, "anne", &plan_de_bruno).await);

    bloquer(&service, "anne", &bruno).await;

    assert!(
        !fil_contient(&service, "anne", &plan_de_bruno).await,
        "le fil de qui bloque montre encore les plans de la personne bloquée"
    );
    assert!(
        !fil_contient(&service, "bruno", &plan_d_anne).await,
        "le fil de la personne bloquée montre encore les plans de qui l'a bloquée"
    );
}

/// Lever un blocage rend les deux fils, pas seulement celui qui lève.
///
/// Poser un blocage oubliait bien les deux fils ; le lever n'en oubliait qu'un.
/// Le fil de l'autre restait celui d'avant — composé sans les plans de qui la
/// bloquait — et le déblocage n'avait aucun effet visible pour elle pendant
/// toute la durée du cache. C'est le côté qui ne peut pas comprendre pourquoi
/// rien ne revient : elle n'a rien demandé et ne sait rien de ce qui s'est
/// passé.
#[tokio::test]
async fn lever_un_blocage_rend_les_deux_fils_sans_attendre_le_cache() {
    let service = Service::monter().await;
    let bruno = deux_voisins(&service).await;

    let plan_d_anne = plan_de(&service, "anne").await;
    let plan_de_bruno = plan_de(&service, "bruno").await;

    bloquer(&service, "anne", &bruno).await;

    // Les deux fils sont relus pendant le blocage : ils sont donc en cache,
    // chacun composé sans l'autre.
    assert!(!fil_contient(&service, "anne", &plan_de_bruno).await);
    assert!(!fil_contient(&service, "bruno", &plan_d_anne).await);

    let (statut, corps) = service
        .delete(&format!("/v1/blocks/{bruno}"), Some(&service.jeton("anne")))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    assert!(
        fil_contient(&service, "anne", &plan_de_bruno).await,
        "le fil de qui lève le blocage ne rend pas les plans de l'autre"
    );
    assert!(
        fil_contient(&service, "bruno", &plan_d_anne).await,
        "le fil de la personne débloquée ne rend pas les plans de l'autre"
    );
}

#[tokio::test]
async fn on_ne_se_bloque_pas_soi_meme() {
    let service = Service::monter().await;
    let anne = service.compte("anne", "depart").await;

    let (statut, corps) = service
        .post(
            "/v1/blocks",
            Some(&service.jeton("anne")),
            json!({ "accountId": anne }),
        )
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY, "{corps}");
}

/// Deux comptes voisins, avec une fiche : sans elle, le fil ne compose rien.
/// Rend l'identifiant de Bruno, le seul que les tests aient à nommer.
async fn deux_voisins(service: &Service) -> String {
    service.compte("anne", "depart").await;
    let bruno = service.compte("bruno", "depart").await;
    fiche(service, "anne", "femme").await;
    fiche(service, "bruno", "homme").await;
    bruno
}

async fn fiche(service: &Service, nom: &str, genre: &str) {
    let (statut, corps) = service
        .put(
            "/v1/me/profile",
            Some(&service.jeton(nom)),
            json!({
                "city": "Lyon",
                "latitude": 45.76,
                "longitude": 4.84,
                "gender": genre,
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
}

async fn plan_de(service: &Service, nom: &str) -> String {
    let (statut, corps) = service
        .post(
            "/v1/plans",
            Some(&service.jeton(nom)),
            json!({
                "title": "Une balade sur les quais",
                "category": "balade",
                "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    corps["id"].as_str().expect("identifiant").to_string()
}

async fn bloquer(service: &Service, nom: &str, cible: &str) {
    let (statut, corps) = service
        .post(
            "/v1/blocks",
            Some(&service.jeton(nom)),
            json!({ "accountId": cible }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
}

/// Lit le fil tel que la route le rend — **sans oublier le cache**.
///
/// C'est tout l'objet de ces tests : si la route qui vient de changer quelque
/// chose n'a pas invalidé le fil concerné, cette lecture rend la composition
/// d'avant, et l'assertion tombe. Oublier le cache ici masquerait exactement
/// le défaut qu'on cherche.
async fn fil_contient(service: &Service, nom: &str, plan: &str) -> bool {
    let (statut, corps): (StatusCode, Value) =
        service.get("/v1/plans", Some(&service.jeton(nom))).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    corps["plans"]
        .as_array()
        .expect("le fil rend une liste")
        .iter()
        .any(|p| p["id"] == plan)
}
