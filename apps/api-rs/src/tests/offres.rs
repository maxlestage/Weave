//! Les achats : unités consommables et notifications de l'App Store.
//!
//! Weave est vendu sur l'App Store ; le serveur ne fait que vérifier puis
//! enregistrer ce qu'Apple lui transmet. Aucune offre n'achète de visibilité —
//! ce qui se vend, c'est l'horizon de publication et les plans de groupe.

use super::Service;
use axum::http::StatusCode;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sea_orm::ConnectionTrait;
use serde_json::json;

/// Forge une transaction signée telle que StoreKit en émet : trois segments
/// séparés par des points, la charge utile au milieu en base64url.
fn transaction(charge: serde_json::Value) -> String {
    format!(
        "entete.{}.signature",
        URL_SAFE_NO_PAD.encode(charge.to_string())
    )
}

#[tokio::test]
async fn un_achat_a_l_unite_credite_le_solde() {
    let service = Service::monter().await;
    service.compte("c_acheteur", "depart").await;

    let (statut, corps) = service
        .post("/v1/billing/units", Some(&service.jeton("c_acheteur")), json!({
            "signedTransaction": transaction(json!({
                "productId": "com.weave.app.unit.renfort",
                "transactionId": "tx-renfort-001",
                "environment": "Sandbox",
            })),
        }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["alreadyApplied"], false);
    assert_eq!(corps["credits"]["renfort"], 1);
    // Tous les SKU sont présents, même à zéro : l'application affiche la liste
    // entière, et une clé absente s'y lirait comme une erreur.
    assert_eq!(corps["credits"]["bilan"], 0);
}

/// `transactionId` est unique côté Apple. Un client qui réessaie ne doit pas
/// être crédité deux fois — ni recevoir une erreur de base pour autant.
#[tokio::test]
async fn une_meme_transaction_ne_credite_qu_une_fois() {
    let service = Service::monter().await;
    service.compte("c_rejoue", "depart").await;

    let signee = transaction(json!({
        "productId": "com.weave.app.unit.escale",
        "transactionId": "tx-escale-rejouee",
        "environment": "Sandbox",
    }));

    let (statut, premier) = service
        .post("/v1/billing/units", Some(&service.jeton("c_rejoue")), json!({ "signedTransaction": signee.clone() }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{premier}");
    assert_eq!(premier["credits"]["escale"], 1);

    let (statut, second) = service
        .post("/v1/billing/units", Some(&service.jeton("c_rejoue")), json!({ "signedTransaction": signee }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{second}");
    assert_eq!(second["alreadyApplied"], true);
    assert_eq!(second["credits"]["escale"], 1, "le solde a doublé : {second}");
}

#[tokio::test]
async fn un_produit_inconnu_est_refuse() {
    let service = Service::monter().await;
    service.compte("c_produit", "depart").await;

    let (statut, _) = service
        .post("/v1/billing/units", Some(&service.jeton("c_produit")), json!({
            "signedTransaction": transaction(json!({
                "productId": "com.weave.app.unit.licorne",
                "transactionId": "tx-licorne",
            })),
        }))
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);
}

/// L'achat crédite un solde que le renfort dépense : les deux routes se
/// rejoignent, et c'est ce chemin-là qu'emprunte un vrai achat.
#[tokio::test]
async fn un_renfort_achete_se_depense() {
    let service = Service::monter().await;
    service.compte("c_boucle", "depart").await;

    let (statut, _) = service
        .post("/v1/billing/units", Some(&service.jeton("c_boucle")), json!({
            "signedTransaction": transaction(json!({
                "productId": "com.weave.app.unit.renfort",
                "transactionId": "tx-boucle-001",
            })),
        }))
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (statut, corps) = service
        .post("/v1/requests/renfort", Some(&service.jeton("c_boucle")), json!({}))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["requestsLeftToday"], 10);
}

/// Une notification qui ne correspond à aucun abonnement connu est ignorée
/// sans rien changer : la route n'est pas authentifiée, elle ne doit pas
/// servir de levier.
#[tokio::test]
async fn une_notification_orpheline_est_ignoree() {
    let service = Service::monter().await;

    let (statut, corps) = service
        .post("/v1/billing/apple/notifications", None, json!({
            "signedPayload": transaction(json!({
                "productId": "com.weave.app.sub.escapade.monthly",
                "transactionId": "tx-inconnue",
                "originalTransactionId": "orig-inconnue",
            })),
        }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["ignored"], true);
}

/// Une échéance passée fait retomber le compte au palier de départ. Jamais
/// l'inverse : un abonnement expiré ne conserve pas ses droits.
#[tokio::test]
async fn un_abonnement_expire_retombe_au_palier_de_depart() {
    let service = Service::monter().await;
    let compte = service.compte("c_expire", "escapade").await;

    service
        .db
        .execute_unprepared(&format!(
            "UPDATE subscriptions SET originalTransactionId='orig-expire' WHERE accountId='{compte}'"
        ))
        .await
        .unwrap();

    // Le palier d'avant : vingt-cinq demandes par jour.
    let (_, avant) = service.get("/v1/me", Some(&service.jeton("c_expire"))).await;
    assert_eq!(avant["tier"], "escapade");
    assert_eq!(avant["requestsLeftToday"], 25);

    let hier = (chrono::Utc::now() - chrono::Duration::days(1)).timestamp_millis();
    let (statut, corps) = service
        .post("/v1/billing/apple/notifications", None, json!({
            "signedPayload": transaction(json!({
                "productId": "com.weave.app.sub.escapade.monthly",
                "transactionId": "tx-expire",
                "originalTransactionId": "orig-expire",
                "expiresDate": hier,
            })),
        }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["ignored"], false);

    let (_, apres) = service.get("/v1/me", Some(&service.jeton("c_expire"))).await;
    assert_eq!(apres["tier"], "depart", "le palier expiré a survécu : {apres}");
    assert_eq!(apres["requestsLeftToday"], 5);
}

/// Un renouvellement remet le palier et recharge la dotation mensuelle. Les
/// crédits achetés à l'unité ne sont jamais remis à zéro : on ajoute.
#[tokio::test]
async fn un_renouvellement_recharge_la_dotation_sans_effacer_les_achats() {
    let service = Service::monter().await;
    let compte = service.compte("c_renouvelle", "escapade").await;

    service
        .db
        .execute_unprepared(&format!(
            "UPDATE subscriptions SET originalTransactionId='orig-renouvelle' WHERE accountId='{compte}'"
        ))
        .await
        .unwrap();

    // Une escale achetée à l'unité, avant le renouvellement.
    let (statut, _) = service
        .post("/v1/billing/units", Some(&service.jeton("c_renouvelle")), json!({
            "signedTransaction": transaction(json!({
                "productId": "com.weave.app.unit.escale",
                "transactionId": "tx-escale-avant",
            })),
        }))
        .await;
    assert_eq!(statut, StatusCode::OK);

    let demain = (chrono::Utc::now() + chrono::Duration::days(30)).timestamp_millis();
    let (statut, corps) = service
        .post("/v1/billing/apple/notifications", None, json!({
            "signedPayload": transaction(json!({
                "productId": "com.weave.app.sub.escapade.monthly",
                "transactionId": "tx-renouvelle",
                "originalTransactionId": "orig-renouvelle",
                "expiresDate": demain,
            })),
        }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let (_, fiche) = service.get("/v1/me", Some(&service.jeton("c_renouvelle"))).await;
    assert_eq!(fiche["tier"], "escapade");
    // Une escale achetée, plus celle que le palier Escapade donne chaque mois.
    assert_eq!(
        fiche["credits"]["escale"], 2,
        "l'achat à l'unité a été effacé par la dotation : {fiche}"
    );
}
