//! Demander la vérification de son profil.
//!
//! Le badge se posait depuis la console, et personne ne pouvait le demander.
//! Le Grand Tour vend par ailleurs une « vérification accélérée » : une
//! priorité suppose une file, et il n'y en avait aucune.

use super::{refuse, Service};
use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn un_compte_neuf_n_a_pas_de_demande_et_le_dit() {
    let service = Service::monter().await;
    service.compte("c_v_neuf", "depart").await;
    let jeton = service.jeton("c_v_neuf");

    let (statut, corps) = service.get("/v1/me/verification", Some(&jeton)).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["verified"], false);
    assert!(corps["request"].is_null(), "aucune demande : {corps}");
}

#[tokio::test]
async fn demander_place_le_compte_dans_la_file() {
    let service = Service::monter().await;
    let compte = service.compte("c_v_file", "depart").await;
    let jeton = service.jeton("c_v_file");

    let (statut, corps) = service
        .post("/v1/me/verification", Some(&jeton), json!({ "note": "je suis bien moi" }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let (_, etat) = service.get("/v1/me/verification", Some(&jeton)).await;
    assert_eq!(etat["request"]["state"], "en_attente", "{etat}");

    let file = crate::console::verifications_en_attente(&service.db)
        .await
        .expect("file");
    assert_eq!(file.len(), 1);
    assert_eq!(file[0].compte, compte);
    assert_eq!(file[0].note, "je suis bien moi");
}

/// Redemander pendant qu'une demande court ne crée pas de doublon.
///
/// Sans cela, la file se remplirait de la même personne — et la priorité
/// vendue au Grand Tour s'achèterait en appuyant plusieurs fois.
#[tokio::test]
async fn redemander_ne_double_pas_la_file() {
    let service = Service::monter().await;
    service.compte("c_v_double", "depart").await;
    let jeton = service.jeton("c_v_double");

    for _ in 0..3 {
        let (statut, corps) = service.post("/v1/me/verification", Some(&jeton), json!({})).await;
        assert_eq!(statut, StatusCode::OK, "{corps}");
    }

    let file = crate::console::verifications_en_attente(&service.db)
        .await
        .expect("file");
    assert_eq!(file.len(), 1, "la même personne figure plusieurs fois");
}

/// Ce que le Grand Tour achète : passer devant.
#[tokio::test]
async fn le_grand_tour_passe_devant_meme_en_arrivant_apres() {
    let service = Service::monter().await;
    service.compte("c_v_tot", "depart").await;
    service.compte("c_v_tard", "grandtour").await;

    // Le gratuit demande en premier.
    service
        .post("/v1/me/verification", Some(&service.jeton("c_v_tot")), json!({}))
        .await;
    service
        .post("/v1/me/verification", Some(&service.jeton("c_v_tard")), json!({}))
        .await;

    let file = crate::console::verifications_en_attente(&service.db)
        .await
        .expect("file");
    let ordre: Vec<&str> = file.iter().map(|d| d.compte.as_str()).collect();
    assert_eq!(
        ordre,
        [service.id("c_v_tard"), service.id("c_v_tot")],
        "« Vérification de profil accélérée » est vendue au Grand Tour : \
         elle doit se voir dans l'ordre de la file"
    );
    assert_eq!(file[0].palier, "grandtour");
}

/// Poser le badge répond à la demande : elle sort de la file.
#[tokio::test]
async fn verifier_clot_la_demande() {
    let service = Service::monter().await;
    let compte = service.compte("c_v_clos", "depart").await;
    let jeton = service.jeton("c_v_clos");

    service.post("/v1/me/verification", Some(&jeton), json!({})).await;
    crate::console::verifier(&service.db, &compte, "pièce vue en visio")
        .await
        .expect("vérification");

    assert!(
        crate::console::verifications_en_attente(&service.db)
            .await
            .expect("file")
            .is_empty(),
        "la demande ressortirait à chaque relevé, pour un travail déjà fait"
    );

    // Comme la console en ligne de commande : le résumé d'identité vit un
    // quart d'heure dans le cache, et une vérification qui ne l'invalide pas
    // ne se verrait qu'à son expiration.
    crate::auth::oublier_compte(&service.etat, &compte).await;

    let (_, etat) = service.get("/v1/me/verification", Some(&jeton)).await;
    assert_eq!(etat["verified"], true, "{etat}");
    assert_eq!(etat["request"]["state"], "acceptee", "{etat}");
}

/// Un refus sort de la file, ne pose pas le badge, et dit pourquoi.
#[tokio::test]
async fn un_refus_est_motive_et_rendu_a_qui_il_concerne() {
    let service = Service::monter().await;
    let compte = service.compte("c_v_refus", "depart").await;
    let jeton = service.jeton("c_v_refus");

    service.post("/v1/me/verification", Some(&jeton), json!({})).await;
    crate::console::refuser_verification(&service.db, &compte, "aucune réponse au courrier")
        .await
        .expect("refus");

    let (_, etat) = service.get("/v1/me/verification", Some(&jeton)).await;
    assert_eq!(etat["verified"], false, "un refus ne pose pas le badge : {etat}");
    assert_eq!(etat["request"]["state"], "refusee");
    assert_eq!(
        etat["request"]["decision"], "aucune réponse au courrier",
        "une décision dont on ignore la raison ne se conteste pas : {etat}"
    );

    // Et l'on peut redemander : un refus n'est pas une exclusion.
    let (statut, corps) = service.post("/v1/me/verification", Some(&jeton), json!({})).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(
        crate::console::verifications_en_attente(&service.db).await.expect("file").len(),
        1,
        "un refus doit pouvoir être suivi d'une nouvelle demande"
    );
}

/// Un profil déjà vérifié n'a rien à demander.
#[tokio::test]
async fn un_profil_deja_verifie_ne_redemande_pas() {
    let service = Service::monter().await;
    let compte = service.compte("c_v_deja", "depart").await;
    let jeton = service.jeton("c_v_deja");

    crate::console::verifier(&service.db, &compte, "vu").await.expect("badge");

    let (statut, corps) = service.post("/v1/me/verification", Some(&jeton), json!({})).await;
    refuse(statut, &corps, "validation", &format!("{corps}"));
}

/// Rien d'identifiant ne se dépose ici : le mot joint est borné, et la
/// vérification se poursuit par courrier.
#[tokio::test]
async fn le_mot_joint_est_borne() {
    let service = Service::monter().await;
    service.compte("c_v_long", "depart").await;
    let jeton = service.jeton("c_v_long");

    let (statut, corps) = service
        .post(
            "/v1/me/verification",
            Some(&jeton),
            json!({ "note": "a".repeat(501) }),
        )
        .await;
    refuse(statut, &corps, "validation", &format!("{corps}"));
}
