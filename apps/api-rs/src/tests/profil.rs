//! La fiche et les critères du fil.
//!
//! Quatre routes que le portage n'avait pas : sans elles, un compte créé reste
//! en « onboarding » pour toujours — il ne peut ni publier ni demander.

use super::Service;
use axum::http::StatusCode;
use sea_orm::{ConnectionTrait, Statement};
use serde_json::json;

/// Lit une colonne d'une table, pour vérifier ce qui a réellement été écrit.
async fn colonne(service: &Service, sql: &str) -> String {
    service
        .db
        .query_one_raw(Statement::from_string(
            service.db.get_database_backend(),
            sql.to_string(),
        ))
        .await
        .unwrap()
        .map(|l| l.try_get::<String>("", "v").unwrap())
        .expect("une ligne")
}

#[tokio::test]
async fn deposer_sa_fiche_rend_le_compte_actif() {
    let service = Service::monter().await;
    let compte = service.compte("c_fiche_neuve", "depart").await;
    // Le compte créé par `compte_de_test` a déjà un profil ; on le remet en
    // onboarding pour dérouler le parcours réel.
    service
        .db
        .execute_unprepared(&format!(
            "UPDATE accounts SET status='onboarding' WHERE id='{compte}'"
        ))
        .await
        .unwrap();

    let (statut, corps) = service
        .put("/v1/me/profile", Some(&service.jeton("c_fiche_neuve")), json!({
            "city": "Lyon",
            "latitude": 45.7578137,
            "longitude": 4.8320114,
            "gender": "autre",
            "bio": "On se croise sur les quais.",
        }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let statut_compte = colonne(
        &service,
        &format!("SELECT status AS v FROM accounts WHERE id='{compte}'"),
    )
    .await;
    assert_eq!(statut_compte, "active", "le compte est resté en onboarding");
}

/// Weave ne conserve jamais une position plus précise que le kilomètre, et
/// l'arrondi se fait au dépôt : ce qui n'est pas enregistré ne peut pas fuir.
#[tokio::test]
async fn la_position_est_arrondie_avant_d_etre_enregistree() {
    let service = Service::monter().await;
    let compte = service.compte("c_position", "depart").await;

    let (statut, _) = service
        .put("/v1/me/profile", Some(&service.jeton("c_position")), json!({
            "city": "Lyon",
            "latitude": 45.7578137,
            "longitude": 4.8320114,
            "gender": "autre",
        }))
        .await;
    assert_eq!(statut, StatusCode::OK);

    let position = colonne(
        &service,
        &format!(
            "SELECT CAST(latRounded AS TEXT) || ',' || CAST(lonRounded AS TEXT) AS v \
             FROM profiles WHERE accountId='{compte}'"
        ),
    )
    .await;
    assert_eq!(
        position, "45.76,4.83",
        "une position au dix-millième a été enregistrée"
    );
}

#[tokio::test]
async fn une_fiche_sans_ville_est_refusee() {
    let service = Service::monter().await;
    service.compte("c_sans_ville", "depart").await;

    let (statut, _) = service
        .put("/v1/me/profile", Some(&service.jeton("c_sans_ville")), json!({
            "city": "   ",
            "latitude": 45.75,
            "longitude": 4.85,
            "gender": "autre",
        }))
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn une_phrase_trop_longue_est_refusee() {
    let service = Service::monter().await;
    service.compte("c_bavard", "depart").await;

    let (statut, _) = service
        .put("/v1/me/profile", Some(&service.jeton("c_bavard")), json!({
            "city": "Lyon",
            "latitude": 45.75,
            "longitude": 4.85,
            "gender": "autre",
            "bio": "é".repeat(161),
        }))
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY, "161 caractères sont passés");
}

#[tokio::test]
async fn modifier_son_nom_affiche() {
    let service = Service::monter().await;
    service.compte("c_renomme", "depart").await;

    let (statut, _) = service
        .patch("/v1/me", Some(&service.jeton("c_renomme")), json!({ "displayName": "Camille" }))
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (statut, corps) = service.get("/v1/me", Some(&service.jeton("c_renomme"))).await;
    assert_eq!(statut, StatusCode::OK);
    assert_eq!(
        corps["displayName"], "Camille",
        "le résumé en cache a survécu à la modification : {corps}"
    );
}

#[tokio::test]
async fn lire_ses_criteres() {
    let service = Service::monter().await;
    service.compte("c_criteres", "depart").await;

    let (statut, corps) = service
        .get("/v1/me/preferences", Some(&service.jeton("c_criteres")))
        .await;
    assert_eq!(statut, StatusCode::OK);
    assert_eq!(corps["minAge"], 18);
    assert_eq!(corps["maxDistanceKm"], 25);
    assert!(corps["seeking"].is_array(), "les listes sortent en tableaux : {corps}");
    assert!(corps["categories"].is_array());
}

#[tokio::test]
async fn ajuster_ses_criteres() {
    let service = Service::monter().await;
    service.compte("c_ajuste", "depart").await;

    let (statut, _) = service
        .patch("/v1/me/preferences", Some(&service.jeton("c_ajuste")), json!({
            "minAge": 25,
            "maxAge": 45,
            "maxDistanceKm": 40,
            "categories": ["balade", "repas"],
        }))
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (_, corps) = service
        .get("/v1/me/preferences", Some(&service.jeton("c_ajuste")))
        .await;
    assert_eq!(corps["minAge"], 25);
    assert_eq!(corps["maxAge"], 45);
    assert_eq!(corps["maxDistanceKm"], 40);
    assert_eq!(corps["categories"], json!(["balade", "repas"]));
}

#[tokio::test]
async fn un_age_minimum_au_dessus_du_maximum_est_refuse() {
    let service = Service::monter().await;
    service.compte("c_bornes", "depart").await;

    let (statut, _) = service
        .patch("/v1/me/preferences", Some(&service.jeton("c_bornes")), json!({
            "minAge": 50, "maxAge": 30,
        }))
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);
}

/// Le rayon est ce qui borne la surveillance qu'un compte peut exercer : le
/// laisser dépasser cent kilomètres changerait la nature du produit.
#[tokio::test]
async fn un_rayon_hors_bornes_est_refuse() {
    let service = Service::monter().await;
    service.compte("c_rayon", "depart").await;

    for rayon in [0, 101, 5_000] {
        let (statut, _) = service
            .patch("/v1/me/preferences", Some(&service.jeton("c_rayon")), json!({
                "maxDistanceKm": rayon,
            }))
            .await;
        assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY, "rayon {rayon} accepté");
    }
}

#[tokio::test]
async fn une_categorie_inconnue_est_refusee() {
    let service = Service::monter().await;
    service.compte("c_categorie", "depart").await;

    let (statut, _) = service
        .patch("/v1/me/preferences", Some(&service.jeton("c_categorie")), json!({
            "categories": ["balade", "croisiere-en-yacht"],
        }))
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);
}
