//! Les demandes vues de part et d'autre : les miennes, celles que je reçois.
//!
//! Le quota journalier est l'invariant central du produit — c'est lui qui
//! empêche d'arroser. Ces tests portent autant sur ce qu'il rend que sur ce
//! qu'il retient.

use super::Service;
use sea_orm::ConnectionTrait;
use axum::http::StatusCode;
use serde_json::json;

/// Publie un plan et rend son identifiant.
async fn plan_de(service: &Service, hote: &str, titre: &str) -> String {
    let (statut, corps) = service
        .post("/v1/plans", Some(&service.jeton(hote)), json!({
            "title": titre,
            "category": "balade",
            "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
        }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    corps["id"].as_str().unwrap().to_string()
}

async fn demander(service: &Service, invite: &str, plan_id: &str) -> (StatusCode, serde_json::Value) {
    service
        .post("/v1/requests", Some(&service.jeton(invite)), json!({
            "planId": plan_id,
            "message": "Cette balade me tente beaucoup, je serais ravi de venir.",
        }))
        .await
}

#[tokio::test]
async fn mes_demandes_envoyees_portent_le_plan_et_son_hote() {
    let service = Service::monter().await;
    service.compte("c_hote_envoi", "depart").await;
    service.compte("c_invite_envoi", "depart").await;
    let plan = plan_de(&service, "c_hote_envoi", "Une balade au bord de l eau").await;

    let (statut, _) = demander(&service, "c_invite_envoi", &plan).await;
    assert_eq!(statut, StatusCode::OK);

    let (statut, corps) = service
        .get("/v1/requests/sent", Some(&service.jeton("c_invite_envoi")))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["requestsLeftToday"], 4);

    let demandes = corps["requests"].as_array().expect("un tableau");
    assert_eq!(demandes.len(), 1, "{corps}");
    assert_eq!(demandes[0]["planId"], plan);
    assert_eq!(demandes[0]["planTitle"], "Une balade au bord de l eau");
    assert_eq!(demandes[0]["state"], "envoyee");
    assert_eq!(demandes[0]["author"]["id"], service.id("c_hote_envoi"));
    assert_eq!(demandes[0]["conversationId"], json!(null));
    // Les dates sortent au format de `toISOString()`.
    let envoyee = demandes[0]["sentAt"].as_str().unwrap();
    assert!(envoyee.ends_with('Z'), "date rendue : {envoyee}");
}

/// Se raviser vite ne doit pas coûter la journée.
#[tokio::test]
async fn retirer_une_demande_rend_son_unite() {
    let service = Service::monter().await;
    service.compte("c_hote_retrait", "depart").await;
    service.compte("c_invite_retrait", "depart").await;
    let plan = plan_de(&service, "c_hote_retrait", "Un plan dont on se ravise").await;

    let (_, demande) = demander(&service, "c_invite_retrait", &plan).await;
    let demande_id = demande["id"].as_str().expect("un identifiant");

    let (statut, _) = service
        .delete(&format!("/v1/requests/{demande_id}"), Some(&service.jeton("c_invite_retrait")))
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (_, corps) = service.get("/v1/me", Some(&service.jeton("c_invite_retrait"))).await;
    assert_eq!(corps["requestsLeftToday"], 5, "l'unité n'a pas été rendue : {corps}");
}

#[tokio::test]
async fn on_ne_retire_pas_la_demande_d_un_autre() {
    let service = Service::monter().await;
    service.compte("c_hote_vol", "depart").await;
    service.compte("c_invite_vol", "depart").await;
    service.compte("c_tiers_vol", "depart").await;
    let plan = plan_de(&service, "c_hote_vol", "Un plan que l on convoite").await;

    let (_, demande) = demander(&service, "c_invite_vol", &plan).await;
    let demande_id = demande["id"].as_str().unwrap();

    let (statut, _) = service
        .delete(&format!("/v1/requests/{demande_id}"), Some(&service.jeton("c_tiers_vol")))
        .await;
    assert_eq!(statut, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn une_demande_deja_tranchee_ne_se_retire_plus() {
    let service = Service::monter().await;
    service.compte("c_hote_tranche", "depart").await;
    service.compte("c_invite_tranche", "depart").await;
    let plan = plan_de(&service, "c_hote_tranche", "Un plan deja tranche ici").await;

    let (_, demande) = demander(&service, "c_invite_tranche", &plan).await;
    let demande_id = demande["id"].as_str().unwrap().to_string();

    let (statut, _) = service
        .post(&format!("/v1/requests/{demande_id}/accept"), Some(&service.jeton("c_hote_tranche")), json!({}))
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (statut, _) = service
        .delete(&format!("/v1/requests/{demande_id}"), Some(&service.jeton("c_invite_tranche")))
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);
}

/// L'unité reste dépensée : elle a été lue. Un refus ne rend rien, sans quoi
/// écrire à tout-va redeviendrait gratuit.
#[tokio::test]
async fn refuser_une_demande_ne_rend_pas_l_unite() {
    let service = Service::monter().await;
    service.compte("c_hote_refus", "depart").await;
    service.compte("c_invite_refus", "depart").await;
    let plan = plan_de(&service, "c_hote_refus", "Un plan ou l on dit non").await;

    let (_, demande) = demander(&service, "c_invite_refus", &plan).await;
    let demande_id = demande["id"].as_str().unwrap().to_string();

    let (statut, _) = service
        .post(&format!("/v1/requests/{demande_id}/decline"), Some(&service.jeton("c_hote_refus")), json!({}))
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (_, corps) = service.get("/v1/me", Some(&service.jeton("c_invite_refus"))).await;
    assert_eq!(corps["requestsLeftToday"], 4, "le refus a rendu l'unité : {corps}");
}

#[tokio::test]
async fn seul_l_hote_refuse() {
    let service = Service::monter().await;
    service.compte("c_hote_seul", "depart").await;
    service.compte("c_invite_seul", "depart").await;
    let plan = plan_de(&service, "c_hote_seul", "Un plan que l on refuse mal").await;

    let (_, demande) = demander(&service, "c_invite_seul", &plan).await;
    let demande_id = demande["id"].as_str().unwrap();

    let (statut, _) = service
        .post(&format!("/v1/requests/{demande_id}/decline"), Some(&service.jeton("c_invite_seul")), json!({}))
        .await;
    assert_eq!(statut, StatusCode::FORBIDDEN);
}

/// Les demandes reçues ne sont pas publiques : le message qu'on écrit pour
/// rejoindre un plan n'est lu que par qui l'organise.
#[tokio::test]
async fn les_demandes_recues_ne_se_lisent_qu_en_hote() {
    let service = Service::monter().await;
    service.compte("c_hote_recu", "depart").await;
    service.compte("c_invite_recu", "depart").await;
    service.compte("c_curieux", "depart").await;
    let plan = plan_de(&service, "c_hote_recu", "Un plan aux demandes privees").await;

    let (statut, _) = demander(&service, "c_invite_recu", &plan).await;
    assert_eq!(statut, StatusCode::OK);

    let (statut, corps) = service
        .get(&format!("/v1/plans/{plan}/requests"), Some(&service.jeton("c_hote_recu")))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    let recues = corps.as_array().expect("un tableau");
    assert_eq!(recues.len(), 1);
    assert_eq!(recues[0]["author"]["id"], service.id("c_invite_recu"));
    assert!(recues[0]["message"].as_str().unwrap().contains("tente"));

    let (statut, _) = service
        .get(&format!("/v1/plans/{plan}/requests"), Some(&service.jeton("c_curieux")))
        .await;
    assert_eq!(statut, StatusCode::FORBIDDEN, "un tiers a lu les demandes reçues");
}

#[tokio::test]
async fn mes_plans_comptent_les_places_et_les_demandes() {
    let service = Service::monter().await;
    service.compte("c_hote_miens", "depart").await;
    service.compte("c_invite_miens", "depart").await;
    let plan = plan_de(&service, "c_hote_miens", "Un plan dont on compte les places").await;

    let (statut, _) = demander(&service, "c_invite_miens", &plan).await;
    assert_eq!(statut, StatusCode::OK);

    let (statut, corps) = service
        .get("/v1/plans/mine", Some(&service.jeton("c_hote_miens")))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    let miens = corps.as_array().expect("un tableau");
    assert_eq!(miens.len(), 1, "{corps}");
    assert_eq!(miens[0]["id"], plan);
    assert_eq!(miens[0]["pendingRequests"], 1);
    assert_eq!(miens[0]["seatsLeft"], 1);
    assert_eq!(miens[0]["state"], "ouvert");
}

/// Le renfort ajoute cinq demandes, et seulement si l'unité a été achetée.
#[tokio::test]
async fn un_renfort_sans_credit_est_refuse_et_ne_grignote_pas_le_plafond() {
    let service = Service::monter().await;
    let compte = service.compte("c_renfort", "depart").await;

    let (statut, corps) = service
        .post("/v1/requests/renfort", Some(&service.jeton("c_renfort")), json!({}))
        .await;
    assert_eq!(statut, StatusCode::PAYMENT_REQUIRED, "{corps}");

    // La place réservée a bien été rendue : le quota du jour est inchangé.
    let (_, fiche) = service.get("/v1/me", Some(&service.jeton("c_renfort"))).await;
    assert_eq!(fiche["requestsLeftToday"], 5, "le plafond a été grignoté : {fiche}");

    // Avec un crédit, il passe.
    service
        .db
        .execute_unprepared(&format!(
            "INSERT INTO credit_balances (id,accountId,sku,balance,updatedAt) \
             VALUES ('cb_{compte}','{compte}','renfort',1,'2026-09-12 10:00:00')"
        ))
        .await
        .unwrap();

    let (statut, corps) = service
        .post("/v1/requests/renfort", Some(&service.jeton("c_renfort")), json!({}))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["granted"], 5);
    assert_eq!(corps["requestsLeftToday"], 10, "le renfort n'a pas augmenté le quota");
}

/// La dernière place d'un plan ne doit être accordée qu'une fois.
///
/// La capacité est comptée AVANT d'ouvrir la transaction. Deux acceptations
/// concurrentes de demandes DIFFÉRENTES lisent donc toutes deux le même
/// compte, et passent toutes deux : le filtre d'état sur la demande ne
/// sérialise que deux acceptations de la MÊME demande.
#[tokio::test]
async fn deux_acceptations_concurrentes_ne_donnent_pas_deux_fois_la_meme_place() {
    let service = Service::monter().await;
    service.compte("c_hote_place", "depart").await;
    service.compte("c_premier_place", "depart").await;
    service.compte("c_second_place", "depart").await;

    // Un plan en solo : une seule place à prendre.
    let plan = plan_de(&service, "c_hote_place", "Un cafe en tete a tete").await;

    let (s1, c1) = demander(&service, "c_premier_place", &plan).await;
    assert_eq!(s1, StatusCode::OK, "{c1}");
    let (s2, c2) = demander(&service, "c_second_place", &plan).await;
    assert_eq!(s2, StatusCode::OK, "{c2}");

    let premier = c1["id"].as_str().unwrap().to_string();
    let second = c2["id"].as_str().unwrap().to_string();
    let jeton = service.jeton("c_hote_place");

    let route_premier = format!("/v1/requests/{premier}/accept");
    let route_second = format!("/v1/requests/{second}/accept");
    let (a, b) = tokio::join!(
        service.post(&route_premier, Some(&jeton), json!({})),
        service.post(&route_second, Some(&jeton), json!({})),
    );

    let acceptees = [a.0, b.0].iter().filter(|s| s.is_success()).count();
    assert_eq!(
        acceptees, 1,
        "une seule place, une seule acceptation — obtenu {acceptees} (réponses : {a:?} / {b:?})"
    );
}

/// Deux retraits simultanés de la même demande ne rendent qu'une unité.
///
/// Le retrait lisait l'état, le contrôlait, puis écrivait : deux appels
/// concurrents franchissaient le contrôle ensemble et REMBOURSAIENT TOUS LES
/// DEUX. Le script Lua du cache borne le compteur à zéro — il empêche de passer
/// sous le plancher, pas de récupérer plus qu'on n'a dépensé.
///
/// Le quota journalier est l'invariant central du produit : « on ne peut pas
/// arroser, et aucun achat ne lève cette limite du jour ». Cinq retraits
/// simultanés d'une seule demande rendaient cinq unités.
#[tokio::test]
async fn deux_retraits_simultanes_ne_rendent_qu_une_unite() {
    let service = Service::monter().await;
    service.compte("c_hote_course", "depart").await;
    service.compte("c_invite_course", "depart").await;
    let jeton = service.jeton("c_invite_course");

    // Trois demandes envoyées : le compteur est à trois, loin du plancher que
    // le script Lua défend. C'est au-dessus de zéro que la course se voit.
    let mut premiere = String::new();
    for n in 0..3 {
        let plan = plan_de(&service, "c_hote_course", &format!("Un plan numero {n} ici")).await;
        let (_, demande) = demander(&service, "c_invite_course", &plan).await;
        if n == 0 {
            premiere = demande["id"].as_str().expect("un identifiant").to_string();
        }
    }

    let (_, avant) = service.get("/v1/me", Some(&jeton)).await;
    assert_eq!(avant["requestsLeftToday"], 2, "trois demandes envoyées : {avant}");

    // Quatre retraits de LA MÊME demande, lancés ensemble.
    let chemin = format!("/v1/requests/{premiere}");
    let (a, b, c, d) = tokio::join!(
        service.delete(&chemin, Some(&jeton)),
        service.delete(&chemin, Some(&jeton)),
        service.delete(&chemin, Some(&jeton)),
        service.delete(&chemin, Some(&jeton)),
    );
    let reussis = [a, b, c, d].iter().filter(|(s, _)| *s == StatusCode::OK).count();
    assert_eq!(reussis, 1, "un seul retrait doit aboutir");

    let (_, apres) = service.get("/v1/me", Some(&jeton)).await;
    assert_eq!(
        apres["requestsLeftToday"], 3,
        "un seul retrait, donc une seule unité rendue : {apres}"
    );
}
