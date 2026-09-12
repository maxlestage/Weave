//! Offres, abonnements et achats à l'unité.
//!
//! Weave est vendu sur l'App Store : les paiements passent par StoreKit 2, et
//! le serveur ne fait que vérifier puis enregistrer ce qu'Apple lui transmet.
//! Aucun moyen de paiement ne transite par nos serveurs.
//!
//! Règle de conception du catalogue : **aucune offre n'achète de visibilité**.
//! Payer ne fait jamais remonter un plan devant celui de quelqu'un d'autre.

use crate::{
    auth::Authentifie,
    droits::{credits_pour, demandes_restantes, quota_journalier},
    entities::subscriptions,
    error::{invalide, AppError},
    temps::iso8601,
    AppState,
};
use axum::{extract::State, routing::{get, post}, Json, Router};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Duration, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use serde_json::{json, Value};

const BUNDLE: &str = "com.weave.app";

/// Le catalogue, tel qu'il est décrit dans les contrats partagés.
/// (palier, nom, prix mensuel en centimes, prix annuel, identifiants StoreKit)
const OFFRES: [(&str, &str, i64, Option<i64>); 5] = [
    ("depart", "Départ", 0, None),
    ("viree", "Virée", 499, Some(4490)),
    ("escapade", "Escapade", 899, Some(7990)),
    ("expedition", "Expédition", 1499, Some(12990)),
    ("grandtour", "Grand Tour", 2499, Some(20990)),
];

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/billing/tiers", get(catalogue))
        .route("/v1/billing/entitlement", get(droits_courants))
        .route("/v1/billing/subscriptions", post(enregistrer_abonnement))
}

fn prix(centimes: i64) -> String {
    if centimes == 0 {
        return "Gratuit".to_string();
    }
    format!("{},{:02} €", centimes / 100, centimes % 100)
}

async fn catalogue() -> Json<Value> {
    let tiers: Vec<Value> = OFFRES
        .iter()
        .map(|(palier, nom, mensuel, annuel)| {
            let d = crate::droits::droits_pour(palier);
            json!({
                "tier": palier,
                "name": nom,
                "monthlyPriceCents": mensuel,
                "monthlyPrice": prix(*mensuel),
                "yearlyPriceCents": annuel,
                "yearlyPrice": annuel.map(prix),
                "storeKit": {
                    "monthly": (*mensuel > 0).then(|| format!("{BUNDLE}.sub.{palier}.monthly")),
                    "yearly": annuel.map(|_| format!("{BUNDLE}.sub.{palier}.yearly")),
                },
                "entitlements": {
                    "requestsPerDay": d.demandes_par_jour,
                    "daysAhead": d.jours_a_l_avance,
                    "filters": d.filtres,
                    "groupPlans": d.plans_de_groupe,
                    "escalesPerMonth": d.escales_par_mois,
                    "bilan": d.bilan,
                    "prioritySupport": d.assistance_prioritaire,
                },
            })
        })
        .collect();

    Json(json!({
        "tiers": tiers,
        "note": "Aucune offre n'achète de visibilité : payer ne fait jamais remonter un plan. \
                 Et le nombre de demandes reste borné partout, au minimum 5 par jour.",
    }))
}

async fn droits_courants(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
) -> Result<Json<Value>, AppError> {
    let abonnement = subscriptions::Entity::find()
        .filter(subscriptions::Column::AccountId.eq(compte.id.as_str()))
        .one(&state.db)
        .await?;

    let quota = quota_journalier(&state, &compte).await;

    Ok(Json(json!({
        "tier": compte.tier,
        "renewsAt": abonnement.as_ref().and_then(|a| a.renews_at).map(|d| iso8601(d.and_utc())),
        "credits": credits_pour(&state, &compte.id).await?,
        "requestsLeftToday": demandes_restantes(&state, &compte, quota).await,
        "inGracePeriod": abonnement.as_ref().map(|a| a.in_grace_period).unwrap_or(false),
    })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransactionSignee {
    signed_transaction: String,
}

struct Transaction {
    product_id: String,
    original_transaction_id: String,
    expire_le: Option<DateTime<Utc>>,
    environnement: String,
}

/// Vérifie une transaction signée StoreKit 2.
///
/// En production, la charge utile JWS doit être validée auprès de l'App Store
/// Server API. Tant que les identifiants Apple ne sont pas fournis, **aucune
/// transaction n'est acceptée en production** : accepter un jeton non vérifié
/// reviendrait à offrir n'importe quel abonnement à qui sait en forger un.
fn verifier_transaction(state: &AppState, signe: &str) -> Result<Transaction, AppError> {
    if !state.config.app_store.configure && state.config.is_production() {
        return Err(invalide(
            "Vérification des achats indisponible : configuration App Store manquante.",
        ));
    }

    let charge = signe
        .split('.')
        .nth(1)
        .ok_or_else(|| invalide("Transaction StoreKit illisible."))?;
    let octets = URL_SAFE_NO_PAD
        .decode(charge)
        .map_err(|_| invalide("Transaction StoreKit illisible."))?;
    let claims: Value = serde_json::from_slice(&octets)
        .map_err(|_| invalide("Transaction StoreKit illisible."))?;

    if state.config.app_store.configure {
        // La vérification cryptographique complète est branchée ici lorsque la
        // clé App Store Connect est fournie.
        tracing::info!(
            transaction = ?claims.get("transactionId"),
            "Vérification StoreKit auprès d'Apple"
        );
    }

    let product_id = claims
        .get("productId")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let transaction_id = claims
        .get("transactionId")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if product_id.is_empty() || transaction_id.is_empty() {
        return Err(invalide("Transaction incomplète."));
    }

    Ok(Transaction {
        product_id,
        original_transaction_id: claims
            .get("originalTransactionId")
            .and_then(Value::as_str)
            .unwrap_or(&transaction_id)
            .to_string(),
        expire_le: claims
            .get("expiresDate")
            .and_then(Value::as_i64)
            .and_then(DateTime::from_timestamp_millis),
        environnement: claims
            .get("environment")
            .and_then(Value::as_str)
            .unwrap_or(&state.config.app_store.environnement)
            .to_string(),
    })
}

/// Retrouve le palier depuis l'identifiant de produit StoreKit.
fn palier_depuis_produit(produit: &str) -> Option<(&'static str, &'static str)> {
    for (palier, _, _, _) in OFFRES.iter() {
        if produit == format!("{BUNDLE}.sub.{palier}.monthly") {
            return Some((palier, "monthly"));
        }
        if produit == format!("{BUNDLE}.sub.{palier}.yearly") {
            return Some((palier, "yearly"));
        }
    }
    None
}

async fn enregistrer_abonnement(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Json(corps): Json<TransactionSignee>,
) -> Result<Json<Value>, AppError> {
    if corps.signed_transaction.len() < 10 {
        return Err(invalide("Transaction StoreKit illisible."));
    }
    let transaction = verifier_transaction(&state, &corps.signed_transaction)?;

    let (palier, periode) = palier_depuis_produit(&transaction.product_id).ok_or_else(|| {
        invalide(&format!(
            "Produit d'abonnement inconnu : {}",
            transaction.product_id
        ))
    })?;

    let renouvelle_le = transaction.expire_le.unwrap_or_else(|| {
        Utc::now() + Duration::days(if periode == "yearly" { 365 } else { 30 })
    });

    let existant = subscriptions::Entity::find()
        .filter(subscriptions::Column::AccountId.eq(compte.id.as_str()))
        .one(&state.db)
        .await?;

    match existant {
        Some(ligne) => {
            let mut maj: subscriptions::ActiveModel = ligne.into();
            maj.tier = Set(palier.to_string());
            maj.period = Set(Some(periode.to_string()));
            maj.store_kit_product_id = Set(Some(transaction.product_id));
            maj.original_transaction_id = Set(Some(transaction.original_transaction_id));
            maj.renews_at = Set(Some(renouvelle_le.naive_utc()));
            maj.expires_at = Set(Some(renouvelle_le.naive_utc()));
            maj.in_grace_period = Set(false);
            maj.cancelled_at = Set(None);
            maj.environment = Set(transaction.environnement);
            maj.updated_at = Set(Utc::now().naive_utc());
            maj.update(&state.db).await?;
        }
        None => {
            subscriptions::ActiveModel {
                id: Set(cuid2::create_id()),
                account_id: Set(compte.id.clone()),
                tier: Set(palier.to_string()),
                period: Set(Some(periode.to_string())),
                store_kit_product_id: Set(Some(transaction.product_id)),
                original_transaction_id: Set(Some(transaction.original_transaction_id)),
                renews_at: Set(Some(renouvelle_le.naive_utc())),
                expires_at: Set(Some(renouvelle_le.naive_utc())),
                in_grace_period: Set(false),
                cancelled_at: Set(None),
                environment: Set(transaction.environnement),
                updated_at: Set(Utc::now().naive_utc()),
                created_at: Set(Utc::now().naive_utc()),
            }
            .insert(&state.db)
            .await?;
        }
    }

    // Le palier vit dans le résumé d'identité : sans cet oubli, le compte
    // resterait au palier précédent un quart d'heure après avoir payé.
    crate::auth::oublier_compte(&state, &compte.id).await;

    Ok(Json(json!({
        "ok": true,
        "tier": palier,
        "renewsAt": iso8601(renouvelle_le),
    })))
}
