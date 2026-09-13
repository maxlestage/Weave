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
    droits::{credits_pour, demandes_restantes, droits_pour, quota_journalier},
    entities::{credit_balances, subscriptions, unit_purchases},
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

/// Les unités achetables, telles que `packages/contracts` les décrit.
/// (sku, nom, identifiant StoreKit sans le préfixe, prix en centimes, dotation)
const UNITES: [(&str, &str, i64, i32); 5] = [
    ("renfort", "Renfort", 149, 1),
    ("horizon", "Horizon", 99, 1),
    ("tablee", "Tablée", 149, 1),
    ("escale", "Escale", 399, 1),
    ("bilan", "Bilan", 299, 1),
];

/// Retrouve l'unité derrière un identifiant StoreKit.
fn unite_depuis_produit(produit_id: &str) -> Option<(&'static str, &'static str, i64, i32)> {
    UNITES
        .iter()
        .find(|(sku, ..)| produit_id == format!("{BUNDLE}.unit.{sku}"))
        .copied()
}

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
        .route("/v1/billing/units", post(enregistrer_unite))
        .route("/v1/billing/apple/notifications", post(notification_app_store))
}

/// Enregistre un achat à l'unité — consommables StoreKit.
async fn enregistrer_unite(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Json(corps): Json<TransactionSignee>,
) -> Result<Json<Value>, AppError> {
    if corps.signed_transaction.len() < 10 {
        return Err(invalide("Transaction StoreKit illisible."));
    }
    let transaction = verifier_transaction(&state, &corps.signed_transaction)?;

    let (sku, _nom, prix_centimes, dotation) = unite_depuis_produit(&transaction.product_id)
        .ok_or_else(|| {
            invalide(&format!(
                "Produit à l'unité inconnu : {}",
                transaction.product_id
            ))
        })?;

    // `transactionId` est unique côté Apple : la contrainte d'unicité empêche
    // qu'une même transaction soit créditée deux fois. On la lit d'abord pour
    // répondre « déjà appliqué » plutôt que de rendre une erreur de base à un
    // client qui n'a fait que réessayer.
    let deja = unit_purchases::Entity::find()
        .filter(unit_purchases::Column::TransactionId.eq(transaction.transaction_id.as_str()))
        .one(&state.db)
        .await?;

    if deja.is_some() {
        return Ok(Json(json!({
            "ok": true,
            "alreadyApplied": true,
            "credits": credits_pour(&state, &compte.id).await?,
        })));
    }

    unit_purchases::ActiveModel {
        id: Set(cuid2::create_id()),
        account_id: Set(compte.id.clone()),
        sku: Set(sku.to_string()),
        transaction_id: Set(transaction.transaction_id.clone()),
        quantity: Set(dotation),
        price_cents: Set(prix_centimes as i32),
        currency: Set("EUR".to_string()),
        environment: Set(transaction.environnement.clone()),
        consumed_at: Set(None),
        refunded_at: Set(None),
        purchased_at: Set(Utc::now().naive_utc()),
    }
    .insert(&state.db)
    .await?;

    crediter(&state, &compte.id, sku, dotation, None).await?;

    Ok(Json(json!({
        "ok": true,
        "alreadyApplied": false,
        "credits": credits_pour(&state, &compte.id).await?,
    })))
}

/// Ajoute des unités au solde, en créant la ligne si elle manque.
async fn crediter(
    state: &AppState,
    compte_id: &str,
    sku: &str,
    montant: i32,
    remise_a_zero: Option<chrono::NaiveDateTime>,
) -> Result<(), AppError> {
    let existant = credit_balances::Entity::find()
        .filter(credit_balances::Column::AccountId.eq(compte_id))
        .filter(credit_balances::Column::Sku.eq(sku))
        .one(&state.db)
        .await?;

    match existant {
        Some(ligne) => {
            let solde = ligne.balance + montant;
            let mut maj: credit_balances::ActiveModel = ligne.into();
            maj.balance = Set(solde);
            if let Some(date) = remise_a_zero {
                maj.resets_at = Set(Some(date));
            }
            maj.updated_at = Set(Utc::now().naive_utc());
            maj.update(&state.db).await?;
        }
        None => {
            credit_balances::ActiveModel {
                id: Set(cuid2::create_id()),
                account_id: Set(compte_id.to_string()),
                sku: Set(sku.to_string()),
                balance: Set(montant),
                resets_at: Set(remise_a_zero),
                updated_at: Set(Utc::now().naive_utc()),
            }
            .insert(&state.db)
            .await?;
        }
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChargeSignee {
    signed_payload: String,
}

/// Notifications serveur à serveur de l'App Store (V2) : renouvellements,
/// remboursements, expirations, périodes de grâce.
///
/// Cette route n'est pas authentifiée — Apple l'appelle, pas un client. Ce qui
/// la protège, c'est la signature de la charge : une notification qui ne
/// correspond à aucun abonnement connu est ignorée sans rien changer.
async fn notification_app_store(
    State(state): State<AppState>,
    Json(corps): Json<ChargeSignee>,
) -> Result<Json<Value>, AppError> {
    let transaction = verifier_transaction(&state, &corps.signed_payload)?;

    let Some((palier, _periode)) = palier_depuis_produit(&transaction.product_id) else {
        return Ok(Json(json!({ "ok": true, "ignored": true })));
    };

    let abonnement = subscriptions::Entity::find()
        .filter(
            subscriptions::Column::OriginalTransactionId
                .eq(transaction.original_transaction_id.as_str()),
        )
        .one(&state.db)
        .await?;

    let Some(abonnement) = abonnement else {
        return Ok(Json(json!({ "ok": true, "ignored": true })));
    };

    let compte_id = abonnement.account_id.clone();
    let echeance = transaction.expire_le;
    // Une échéance passée fait retomber le compte au palier de départ. Jamais
    // l'inverse : un abonnement expiré ne doit pas conserver ses droits.
    let expire = echeance.is_some_and(|date| date < Utc::now());

    let mut maj: subscriptions::ActiveModel = abonnement.into();
    maj.tier = Set(if expire { "depart".to_string() } else { palier.to_string() });
    maj.renews_at = Set(echeance.map(|d| d.naive_utc()));
    maj.expires_at = Set(echeance.map(|d| d.naive_utc()));
    maj.in_grace_period = Set(false);
    maj.updated_at = Set(Utc::now().naive_utc());
    maj.update(&state.db).await?;

    if let (false, Some(echeance)) = (expire, echeance) {
        // Dotation mensuelle du palier. Les crédits achetés à l'unité ne sont
        // jamais remis à zéro : on ajoute, on ne remplace pas.
        let escales = droits_pour(palier).escales_par_mois;
        if escales > 0 {
            crediter(
                &state,
                &compte_id,
                "escale",
                escales as i32,
                Some(echeance.naive_utc()),
            )
            .await?;
        }
    }

    // Le palier vit dans le résumé d'identité : sans cet oubli, le compte
    // garderait son ancien palier un quart d'heure après le renouvellement.
    crate::auth::oublier_compte(&state, &compte_id).await;

    Ok(Json(json!({ "ok": true, "ignored": false })))
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
    /// Unique côté Apple : c'est ce qui empêche qu'un même achat soit crédité
    /// deux fois.
    transaction_id: String,
    original_transaction_id: String,
    expire_le: Option<DateTime<Utc>>,
    environnement: String,
}

/// La vérification cryptographique de la charge JWS est-elle écrite ?
///
/// Elle ne l'est pas. Ce booléen existe pour que la réponse soit à UN seul
/// endroit, et pour que l'activer soit un geste délibéré plutôt qu'un effet de
/// bord.
///
/// Il faudra, pour le passer à `true` : valider la chaîne de certificats `x5c`
/// de l'en-tête contre la racine Apple, vérifier la signature ES256, puis
/// contrôler le `bundleId` et la fraîcheur de la transaction.
pub(crate) const VERIFICATION_JWS_IMPLEMENTEE: bool = false;

/// La production accepte-t-elle cette transaction ?
///
/// La règle précédente était : refuser en production TANT QUE la configuration
/// App Store est absente. Elle se retournait au pire moment — le jour où l'on
/// pose `APPSTORE_ISSUER_ID`, c'est-à-dire le geste même par lequel on croit
/// activer les achats, la porte s'ouvrait sur des charges JWS non vérifiées.
/// Or décoder du base64 n'est pas vérifier : n'importe qui pouvait encoder un
/// JSON annonçant le produit de son choix et s'offrir l'abonnement.
///
/// La condition ne porte donc plus sur la présence d'identifiants, mais sur
/// l'existence du code qui vérifie. Configurer l'App Store n'ouvre plus rien
/// par lui-même.
fn achat_acceptable(production: bool, verification_ecrite: bool) -> bool {
    !production || verification_ecrite
}

/// Vérifie une transaction signée StoreKit 2.
///
/// Hors production, la charge est décodée et crue sur parole : c'est ce qui
/// permet d'éprouver le parcours d'achat sans compte Apple Developer.
fn verifier_transaction(state: &AppState, signe: &str) -> Result<Transaction, AppError> {
    if !achat_acceptable(state.config.is_production(), VERIFICATION_JWS_IMPLEMENTEE) {
        return Err(invalide(
            "Vérification des achats indisponible : la validation cryptographique \
             des transactions App Store n'est pas encore en service.",
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

    // C'est ici que la vérification prendra place, et `VERIFICATION_JWS_IMPLEMENTEE`
    // passera à `true` le jour où elle sera écrite.

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
        transaction_id: transaction_id.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// La règle précédente se retournait au pire moment : poser les
    /// identifiants App Store — le geste par lequel on croit activer les
    /// achats — ouvrait la production aux charges non vérifiées.
    ///
    /// Elle ne dépend plus de la configuration, mais de l'existence du code qui
    /// vérifie. Ce test tombera le jour où quelqu'un remettra la condition à
    /// l'endroit, et c'est son but.
    #[test]
    fn la_production_refuse_tant_que_la_verification_n_est_pas_ecrite() {
        assert!(
            !achat_acceptable(true, false),
            "en production, sans vérification écrite, aucun achat ne passe"
        );
        assert!(
            achat_acceptable(true, true),
            "avec la vérification, la production accepte"
        );
    }

    /// Hors production, on décode et on croit sur parole : c'est ce qui permet
    /// d'éprouver le parcours d'achat sans compte Apple Developer.
    #[test]
    fn le_developpement_accepte_sans_verification() {
        assert!(achat_acceptable(false, false));
    }

    /// Le garde-fou est armé. S'il ne l'était pas sans que la vérification soit
    /// écrite, la production accepterait n'importe quelle transaction forgée.
    #[test]
    fn la_verification_n_est_pas_annoncee_comme_ecrite_par_erreur() {
        assert!(
            !VERIFICATION_JWS_IMPLEMENTEE,
            "si la vérification JWS est écrite, retirez ce test — sinon, le \
             passer à `true` ouvre la production aux transactions forgées"
        );
    }
}
