//! Offres, abonnements et achats à l'unité.
//!
//! Weave est vendu sur l'App Store : les paiements passent par StoreKit 2, et
//! le serveur ne fait que vérifier puis enregistrer ce qu'Apple lui transmet.
//! Aucun moyen de paiement ne transite par nos serveurs.
//!
//! Règle de conception du catalogue : **aucune offre n'achète de visibilité**.
//! Payer ne fait jamais remonter un plan devant celui de quelqu'un d'autre.

use crate::messages::Msg;
use crate::{
    AppState,
    auth::Authentifie,
    droits::{credits_pour, demandes_restantes, droits_pour, quota_journalier},
    entities::{credit_balances, subscriptions, unit_purchases},
    error::{AppError, invalide},
    temps::iso8601,
};
use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use chrono::{DateTime, Duration, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use serde_json::{Value, json};

const BUNDLE: &str = "com.weave.app";

/// Les unités achetables, telles que `packages/contracts` les décrit.
/// (sku, nom, identifiant StoreKit sans le préfixe, prix en centimes, dotation)
const UNITES: [(&str, &str, i64, i32); 5] = [
    ("renfort", "Renfort", 299, 1),
    ("horizon", "Horizon", 299, 1),
    ("tablee", "Tablée", 299, 1),
    ("escale", "Escale", 599, 1),
    ("bilan", "Bilan", 499, 1),
];

/// Retrouve l'unité derrière un identifiant StoreKit.
fn unite_depuis_produit(produit_id: &str) -> Option<(&'static str, &'static str, i64, i32)> {
    UNITES
        .iter()
        .find(|(sku, ..)| produit_id == format!("{BUNDLE}.unit.{sku}"))
        .copied()
}

/// Le catalogue, tel qu'il est décrit dans les contrats partagés.
/// (palier, nom, prix mensuel en centimes)
///
/// Il n'y a pas d'abonnement annuel : un engagement de douze mois sur un
/// service qu'on peut vouloir quitter du jour au lendemain ne rend service
/// qu'à celui qui l'encaisse.
const OFFRES: [(&str, &str, i64); 5] = [
    ("depart", "Départ", 0),
    ("viree", "Virée", 499),
    ("escapade", "Escapade", 899),
    ("expedition", "Expédition", 1499),
    ("grandtour", "Grand Tour", 2499),
];

/// Les tables de prix, pour le test qui les confronte au contrat partagé.
///
/// Exposées plutôt que recopiées dans le test : une copie de plus serait une
/// divergence de plus, et c'est précisément ce que le test cherche à empêcher.
#[cfg(test)]
pub fn unites_pour_test() -> Vec<(&'static str, i64)> {
    UNITES
        .iter()
        .map(|(sku, _, prix, _)| (*sku, *prix))
        .collect()
}

#[cfg(test)]
pub fn offres_pour_test() -> Vec<(&'static str, i64)> {
    OFFRES
        .iter()
        .map(|(palier, _, prix)| (*palier, *prix))
        .collect()
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/billing/tiers", get(catalogue))
        .route("/v1/billing/entitlement", get(droits_courants))
        .route("/v1/billing/subscriptions", post(enregistrer_abonnement))
        .route("/v1/billing/units", post(enregistrer_unite))
        .route(
            "/v1/billing/apple/notifications",
            post(notification_app_store),
        )
}

/// Enregistre un achat à l'unité — consommables StoreKit.
async fn enregistrer_unite(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Json(corps): Json<TransactionSignee>,
) -> Result<Json<Value>, AppError> {
    if corps.signed_transaction.len() < 10 {
        return Err(invalide(Msg::TransactionStoreKitIllisible));
    }
    let transaction = verifier_transaction(&state, &corps.signed_transaction)?;

    let (sku, _nom, prix_centimes, dotation) = unite_depuis_produit(&transaction.product_id)
        .ok_or_else(|| {
            invalide(Msg::ProduitALUniteInconnu {
                identifiant: transaction.product_id.clone(),
            })
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

/// Accorde la dotation d'une période, une fois et une seule.
///
/// Apple **renvoie** ses notifications tant qu'il n'obtient pas de 200, et en
/// délivre parfois plusieurs pour le même événement. Chaque renvoi rappelait
/// `crediter`, qui ajoute : un renouvellement retenté deux fois donnait deux
/// fois la dotation du mois. C'est gratuit, c'est répétable, et rien ne le
/// signalait.
///
/// La date d'échéance sert de marque : tant que la ligne porte déjà celle de
/// la période en cours, la dotation a été accordée et un nouvel appel ne fait
/// rien. Le mois suivant apporte une autre échéance, et la dotation repart.
///
/// La dotation s'ajoute au solde plutôt que de le remplacer, et c'est
/// délibéré : « escale » se vend aussi à l'unité, et remplacer effacerait ce
/// qui a été payé. Les deux origines se mêlent donc dans un même solde, qui
/// n'est jamais remis à zéro — c'est le choix du produit, pas un oubli.
async fn doter_la_periode(
    state: &AppState,
    compte_id: &str,
    sku: &str,
    montant: i32,
    echeance: chrono::NaiveDateTime,
) -> Result<(), AppError> {
    let deja = credit_balances::Entity::find()
        .filter(credit_balances::Column::AccountId.eq(compte_id))
        .filter(credit_balances::Column::Sku.eq(sku))
        .filter(credit_balances::Column::ResetsAt.eq(echeance))
        .one(&state.db)
        .await?;

    if deja.is_some() {
        tracing::debug!(
            compte = compte_id,
            sku,
            "dotation déjà accordée pour cette période"
        );
        return Ok(());
    }

    crediter(state, compte_id, sku, montant, Some(echeance)).await
}

#[cfg(test)]
pub(crate) async fn doter_la_periode_pour_test(
    state: &AppState,
    compte_id: &str,
    sku: &str,
    montant: i32,
    echeance: chrono::NaiveDateTime,
) -> Result<(), AppError> {
    doter_la_periode(state, compte_id, sku, montant, echeance).await
}

/// Le crédit, exposé aux tests.
///
/// `crediter` est privée parce que rien hors de ce module n'a de raison de
/// créditer un compte. Les tests, eux, doivent pouvoir lancer deux crédits en
/// même temps sans passer par StoreKit.
#[cfg(test)]
pub(crate) async fn crediter_pour_test(
    state: &AppState,
    compte_id: &str,
    sku: &str,
    montant: i32,
) -> Result<(), AppError> {
    crediter(state, compte_id, sku, montant, None).await
}

/// Ajoute des unités au solde, en créant la ligne si elle manque.
///
/// L'incrément se fait dans la base, jamais en Rust.
///
/// Le solde était lu, additionné, puis réécrit. Deux crédits simultanés — un
/// achat et le renvoi de la même notification par Apple, deux achats coup sur
/// coup — lisaient tous deux le même solde et n'en écrivaient qu'un : la
/// seconde dotation disparaissait. Quelqu'un payait et ne recevait rien, et
/// rien nulle part ne l'aurait signalé.
///
/// `balance = balance + N` est évalué par la base, sous le verrou de ligne
/// qu'elle pose : deux appels s'additionnent au lieu de s'écraser.
async fn crediter(
    state: &AppState,
    compte_id: &str,
    sku: &str,
    montant: i32,
    remise_a_zero: Option<chrono::NaiveDateTime>,
) -> Result<(), AppError> {
    use sea_orm::sea_query::{Expr, ExprTrait};

    let mut maj = credit_balances::Entity::update_many()
        .col_expr(
            credit_balances::Column::Balance,
            Expr::col(credit_balances::Column::Balance).add(montant),
        )
        .col_expr(
            credit_balances::Column::UpdatedAt,
            Expr::value(Utc::now().naive_utc()),
        );
    if let Some(date) = remise_a_zero {
        maj = maj.col_expr(credit_balances::Column::ResetsAt, Expr::value(date));
    }

    let touchees = maj
        .filter(credit_balances::Column::AccountId.eq(compte_id))
        .filter(credit_balances::Column::Sku.eq(sku))
        .exec(&state.db)
        .await?;

    if touchees.rows_affected > 0 {
        return Ok(());
    }

    // Pas de ligne : on la crée. Un index unique porte sur (compte, sku) —
    // deux créations simultanées ne peuvent donc pas faire deux lignes, la
    // seconde échoue. On retente alors l'incrément, qui trouvera la ligne que
    // l'autre vient de poser plutôt que de perdre la dotation.
    let creation = credit_balances::ActiveModel {
        id: Set(cuid2::create_id()),
        account_id: Set(compte_id.to_string()),
        sku: Set(sku.to_string()),
        balance: Set(montant),
        resets_at: Set(remise_a_zero),
        updated_at: Set(Utc::now().naive_utc()),
    }
    .insert(&state.db)
    .await;

    if creation.is_ok() {
        return Ok(());
    }

    let mut rattrapage = credit_balances::Entity::update_many()
        .col_expr(
            credit_balances::Column::Balance,
            Expr::col(credit_balances::Column::Balance).add(montant),
        )
        .col_expr(
            credit_balances::Column::UpdatedAt,
            Expr::value(Utc::now().naive_utc()),
        );
    if let Some(date) = remise_a_zero {
        rattrapage = rattrapage.col_expr(credit_balances::Column::ResetsAt, Expr::value(date));
    }

    let touchees = rattrapage
        .filter(credit_balances::Column::AccountId.eq(compte_id))
        .filter(credit_balances::Column::Sku.eq(sku))
        .exec(&state.db)
        .await?;

    if touchees.rows_affected == 0 {
        // Ni l'incrément, ni la création, ni le rattrapage : la création a
        // échoué pour une autre raison que la course. On rend son erreur
        // plutôt que de dire que le crédit est passé.
        creation?;
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
    let notification = verifier_notification(&state, &corps.signed_payload)?;
    let transaction = &notification.transaction;

    let Some(palier) = palier_depuis_produit(&transaction.product_id) else {
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

    // Une notification plus ancienne que ce qu'on sait déjà ne défait rien.
    //
    // Apple relance pendant trois jours et ne garantit pas l'ordre : une
    // notification « EXPIRED » arrivant après un réabonnement ferait retomber
    // au palier de départ un compte qui vient de payer. On compare donc la
    // date de signature à celle du dernier état écrit.
    if let Some(signee_le) = notification.signee_le {
        if signee_le < abonnement.updated_at.and_utc() {
            return Ok(Json(json!({ "ok": true, "ignored": true, "stale": true })));
        }
    }

    let compte_id = abonnement.account_id.clone();

    // Un remboursement ou une révocation coupe l'accès sur-le-champ.
    //
    // L'échéance, elle, ne bouge pas : Apple rend l'argent sans raccourcir la
    // période. S'en tenir à la date laissait donc l'accès ouvert jusqu'au bout
    // d'un mois qui n'a plus été payé.
    let rendu = matches!(notification.genre.as_str(), "REFUND" | "REVOKE");

    // Une période de grâce prolonge l'accès, c'est sa raison d'être : Apple
    // réessaie de prélever et demande qu'on serve la personne pendant ce
    // temps. La date de fin vit dans `signedRenewalInfo` ; sans elle, il n'y a
    // rien à prolonger.
    let en_grace = !rendu
        && notification.sous_genre.as_deref() == Some("GRACE_PERIOD")
        && notification
            .grace_jusqu_a
            .is_some_and(|fin| fin > Utc::now());

    let echeance = if en_grace {
        notification.grace_jusqu_a
    } else {
        transaction.expire_le
    };

    // Une échéance passée fait retomber le compte au palier de départ. Jamais
    // l'inverse : un abonnement expiré ne doit pas conserver ses droits.
    let expire = rendu || echeance.is_some_and(|date| date < Utc::now());

    let mut maj: subscriptions::ActiveModel = abonnement.into();
    maj.tier = Set(if expire {
        "depart".to_string()
    } else {
        palier.to_string()
    });
    maj.renews_at = Set(echeance.map(|d| d.naive_utc()));
    maj.expires_at = Set(echeance.map(|d| d.naive_utc()));
    maj.in_grace_period = Set(en_grace);
    if rendu {
        maj.cancelled_at = Set(Some(Utc::now().naive_utc()));
    }
    maj.updated_at = Set(Utc::now().naive_utc());
    maj.update(&state.db).await?;

    if let (false, Some(echeance)) = (expire, echeance) {
        // Dotation mensuelle du palier. Les crédits achetés à l'unité ne sont
        // jamais remis à zéro : on ajoute, on ne remplace pas — « escale » se
        // vend aussi, et remplacer le solde effacerait ce qui a été payé.
        let escales = droits_pour(palier).escales_par_mois;
        if escales > 0 {
            doter_la_periode(
                &state,
                &compte_id,
                "escale",
                escales as i32,
                echeance.naive_utc(),
            )
            .await?;
        }
    }

    // Le palier vit dans le résumé d'identité : sans cet oubli, le compte
    // garderait son ancien palier un quart d'heure après le renouvellement.
    crate::auth::oublier_compte(&state, &compte_id).await;

    Ok(Json(json!({
        "ok": true,
        "ignored": false,
        "type": notification.genre,
        "inGracePeriod": en_grace,
    })))
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
        .map(|(palier, nom, mensuel)| {
            let d = crate::droits::droits_pour(palier);
            json!({
                "tier": palier,
                "name": nom,
                "monthlyPriceCents": mensuel,
                "monthlyPrice": prix(*mensuel),
                "storeKit": {
                    "monthly": (*mensuel > 0).then(|| format!("{BUNDLE}.sub.{palier}.monthly")),
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
/// Elle l'est, dans `storekit` : chaîne de certificats `x5c` validée contre la
/// racine Apple épinglée, signature ES256 vérifiée, puis `bundleId`,
/// environnement et fraîcheur contrôlés ici même.
///
/// Ce booléen ne commande donc plus rien — et c'est exactement ce qu'on lui
/// demande. Il reste un fil-piège : le repasser à `false` sans retirer le
/// vérificateur ferait refuser tous les achats en production, et le test
/// `la_verification_est_annoncee_comme_ecrite` s'y oppose. Le retirer, lui,
/// supprimerait le seul endroit où la question se pose.
pub(crate) const VERIFICATION_JWS_IMPLEMENTEE: bool = true;

/// Âge maximal d'une transaction, en minutes.
///
/// Une transaction signée reste valable indéfiniment tant que rien ne borne
/// son âge. La borne limite le rejeu à une fenêtre courte ; l'unicité de
/// `transactionId` en base fait le reste. Large parce que l'horloge d'un
/// appareil peut dériver, et qu'un achat fait hors ligne remonte plus tard.
const FRAICHEUR_MINUTES: i64 = 60;

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
/// La signature est vérifiée DANS TOUS LES ENVIRONNEMENTS. Ce qui change hors
/// production n'est pas la rigueur du contrôle mais la racine à laquelle il
/// remonte : les tests épinglent la leur et signent pour de vrai, ce qui
/// permet d'éprouver le parcours d'achat — et surtout le rejet d'une chaîne
/// qui ne mène pas chez Apple — sans compte Apple Developer.
/// Vérifie la signature d'un JWS d'Apple et rend ses revendications.
fn revendications_signees(state: &AppState, signe: &str) -> Result<Value, AppError> {
    if !achat_acceptable(state.config.is_production(), VERIFICATION_JWS_IMPLEMENTEE) {
        return Err(invalide(Msg::VerificationDesAchatsIndisponible));
    }

    crate::storekit::verifier(signe, &state.racine_storekit, Utc::now()).map_err(|refus| {
        // Le motif va au journal, pas à l'appelant : on ne renseigne pas
        // qui essaie de forger sur ce qui l'a trahi.
        tracing::warn!(motif = refus.motif(), "transaction StoreKit refusée");
        invalide(Msg::TransactionStoreKitRefusee)
    })
}

/// Le paquet est-il le nôtre, et l'environnement le bon ?
fn controler_paquet_et_environnement(
    state: &AppState,
    paquet: &str,
    environnement: &str,
) -> Result<(), AppError> {
    if paquet != BUNDLE {
        tracing::warn!(paquet, "transaction émise pour une autre application");
        return Err(invalide(Msg::TransactionStoreKitRefusee));
    }
    if !environnement.eq_ignore_ascii_case(&state.config.app_store.environnement) {
        tracing::warn!(
            environnement,
            attendu = %state.config.app_store.environnement,
            "transaction d'un autre environnement"
        );
        return Err(invalide(Msg::TransactionStoreKitRefusee));
    }
    Ok(())
}

/// La charge a-t-elle été signée assez récemment ?
fn controler_la_fraicheur(claims: &Value) -> Result<(), AppError> {
    let Some(signee_le) = claims
        .get("signedDate")
        .and_then(Value::as_i64)
        .and_then(DateTime::from_timestamp_millis)
    else {
        return Ok(());
    };
    if (Utc::now() - signee_le).num_minutes().abs() > FRAICHEUR_MINUTES {
        tracing::warn!(%signee_le, "transaction trop ancienne");
        return Err(invalide(Msg::TransactionStoreKitRefusee));
    }
    Ok(())
}

/// Compose une `Transaction` à partir des revendications d'un JWSTransaction.
fn transaction_depuis(claims: &Value, environnement: &str) -> Result<Transaction, AppError> {
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
        return Err(invalide(Msg::TransactionIncomplete));
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
        environnement: environnement.to_string(),
    })
}

fn verifier_transaction(state: &AppState, signe: &str) -> Result<Transaction, AppError> {
    let claims = revendications_signees(state, signe)?;

    // La signature d'Apple ne dit pas POUR QUI elle a été émise.
    //
    // Sans ce contrôle, un achat à un euro fait dans une autre application —
    // signé par Apple, chaîne parfaitement valide — se rejouerait ici pour
    // s'offrir l'abonnement le plus cher. C'est le `bundleId` qui distingue,
    // et lui seul. Sandbox et production ne se mélangent pas non plus : une
    // transaction d'essai ne doit pas créditer un compte réel.
    let environnement = claims
        .get("environment")
        .and_then(Value::as_str)
        .unwrap_or(&state.config.app_store.environnement)
        .to_string();
    controler_paquet_et_environnement(
        state,
        claims
            .get("bundleId")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        &environnement,
    )?;
    controler_la_fraicheur(&claims)?;

    transaction_depuis(&claims, &environnement)
}

/// Ce qu'Apple annonce dans une notification serveur à serveur.
struct Notification {
    /// `notificationType` : `DID_RENEW`, `EXPIRED`, `REFUND`…
    genre: String,
    /// `subtype`, quand il y en a un : `GRACE_PERIOD`, `VOLUNTARY`…
    sous_genre: Option<String>,
    transaction: Transaction,
    /// Fin de la période de grâce, lue dans `signedRenewalInfo`.
    grace_jusqu_a: Option<DateTime<Utc>>,
    /// Quand Apple a signé la notification. Sert à ne pas laisser une
    /// notification ancienne défaire un état plus récent.
    signee_le: Option<DateTime<Utc>>,
}

/// Vérifie une notification V2 de l'App Store — l'enveloppe, pas la transaction.
///
/// ## Ce qui n'allait pas
///
/// Cette route lisait le `signedPayload` d'Apple **comme s'il s'agissait d'un
/// JWSTransaction**, c'est-à-dire du format que le client envoie. Ce n'en est
/// pas un. La documentation d'Apple donne pour champs de premier niveau
/// `notificationType`, `subtype`, `data`, `summary`, `externalPurchaseToken`,
/// `appData`, `version`, `signedDate` et `notificationUUID` — et rien d'autre.
/// Pas de `bundleId`, pas d'`environment`, pas de `productId`.
///
/// Le contrôle du paquet lisait donc une chaîne vide, la comparait à
/// `com.weave.app`, et refusait. **Aucune notification réelle d'Apple n'a
/// jamais pu être traitée** : ni renouvellement, ni expiration, ni
/// remboursement, ni période de grâce. Les tests ne le voyaient pas — ils
/// envoyaient à cette route une transaction, la forme qu'elle savait lire.
///
/// Ce que cela coûtait : `expiresAt` n'avançait qu'aux passages de
/// l'application dans la boutique. Entre deux, l'échéance du mois précédent
/// finissait par tomber, et un abonné à jour de ses paiements retombait au
/// palier de départ.
///
/// La vraie forme est emboîtée : l'enveloppe porte `data.bundleId` et
/// `data.environment`, et la transaction vit dans `data.signedTransactionInfo`,
/// qui est elle-même un JWS à vérifier. On vérifie donc les deux.
fn verifier_notification(state: &AppState, signe: &str) -> Result<Notification, AppError> {
    let enveloppe = revendications_signees(state, signe)?;

    // Pas de fenêtre de fraîcheur ici, contrairement au chemin du client.
    //
    // Elle y borne le rejeu d'une transaction qu'on nous présente. Apple, lui,
    // RÉESSAIE une notification non acquittée pendant trois jours : refuser
    // au-delà d'une heure écarterait chaque relance, c'est-à-dire exactement
    // les cas où le serveur était indisponible. Ce qui protège ici, c'est la
    // signature — et, plus bas, le refus de laisser une notification ancienne
    // défaire un état plus récent.
    let donnees = enveloppe.get("data").ok_or_else(|| {
        tracing::warn!("notification sans « data »");
        invalide(Msg::TransactionStoreKitRefusee)
    })?;

    let environnement = donnees
        .get("environment")
        .and_then(Value::as_str)
        .unwrap_or(&state.config.app_store.environnement)
        .to_string();
    controler_paquet_et_environnement(
        state,
        donnees
            .get("bundleId")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        &environnement,
    )?;

    let signee = donnees
        .get("signedTransactionInfo")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            tracing::warn!("notification sans transaction signée");
            invalide(Msg::TransactionStoreKitRefusee)
        })?;
    // La transaction emboîtée porte sa propre signature : on la vérifie comme
    // l'enveloppe, sans quoi l'enveloppe authentifierait un contenu quelconque.
    let claims = revendications_signees(state, signee)?;

    // `signedRenewalInfo` est facultatif, et son absence n'est pas une faute :
    // un remboursement n'en porte pas.
    let grace_jusqu_a = donnees
        .get("signedRenewalInfo")
        .and_then(Value::as_str)
        .and_then(|jws| revendications_signees(state, jws).ok())
        .and_then(|infos| {
            infos
                .get("gracePeriodExpiresDate")
                .and_then(Value::as_i64)
                .and_then(DateTime::from_timestamp_millis)
        });

    Ok(Notification {
        genre: enveloppe
            .get("notificationType")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        sous_genre: enveloppe
            .get("subtype")
            .and_then(Value::as_str)
            .map(str::to_string),
        transaction: transaction_depuis(&claims, &environnement)?,
        grace_jusqu_a,
        signee_le: enveloppe
            .get("signedDate")
            .and_then(Value::as_i64)
            .and_then(DateTime::from_timestamp_millis),
    })
}

/// Retrouve le palier depuis l'identifiant de produit StoreKit.
///
/// Seul le mensuel est reconnu. Un identifiant annuel — il n'en est plus
/// vendu — tombe donc dans l'inconnu et la transaction est refusée, ce qui est
/// le bon comportement : mieux vaut refuser un produit qu'on ne vend plus que
/// d'accorder un palier sur une durée qu'on ne sait plus tenir.
fn palier_depuis_produit(produit: &str) -> Option<&'static str> {
    OFFRES
        .iter()
        .map(|(palier, ..)| *palier)
        .find(|palier| produit == format!("{BUNDLE}.sub.{palier}.monthly"))
}

async fn enregistrer_abonnement(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Json(corps): Json<TransactionSignee>,
) -> Result<Json<Value>, AppError> {
    if corps.signed_transaction.len() < 10 {
        return Err(invalide(Msg::TransactionStoreKitIllisible));
    }
    let transaction = verifier_transaction(&state, &corps.signed_transaction)?;

    let palier = palier_depuis_produit(&transaction.product_id).ok_or_else(|| {
        invalide(Msg::ProduitDAbonnementInconnu {
            identifiant: transaction.product_id.clone(),
        })
    })?;

    // Un mois, faute d'échéance annoncée par Apple. C'est la seule durée
    // vendue.
    let renouvelle_le = transaction
        .expire_le
        .unwrap_or_else(|| Utc::now() + Duration::days(30));

    let existant = subscriptions::Entity::find()
        .filter(subscriptions::Column::AccountId.eq(compte.id.as_str()))
        .one(&state.db)
        .await?;

    match existant {
        Some(ligne) => {
            let mut maj: subscriptions::ActiveModel = ligne.into();
            maj.tier = Set(palier.to_string());
            // La seule période vendue.
            maj.period = Set(Some("monthly".to_string()));
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
                period: Set(Some("monthly".to_string())),
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

    /// Hors production, le garde-fou laisse passer même sans vérificateur.
    ///
    /// C'est le garde-fou qui s'efface, pas la vérification : `verifier_transaction`
    /// contrôle la signature dans tous les environnements. Ce test dit
    /// seulement qu'un service de développement dont le vérificateur aurait
    /// été retiré n'en serait pas bloqué.
    #[test]
    fn le_developpement_passe_le_garde_fou_sans_verificateur() {
        assert!(achat_acceptable(false, false));
    }

    /// La vérification est écrite : le drapeau doit le dire.
    ///
    /// Ce test disait l'inverse jusqu'ici — il vérifiait que le drapeau était
    /// à `false`, et demandait qu'on le retire le jour où la vérification
    /// serait écrite. Elle l'est, dans `storekit`, et ce test garde désormais
    /// l'autre bout : repasser le drapeau à `false` sans retirer le
    /// vérificateur refuserait tous les achats en production, silencieusement.
    #[test]
    fn la_verification_est_annoncee_comme_ecrite() {
        assert!(
            VERIFICATION_JWS_IMPLEMENTEE,
            "le vérificateur existe dans `storekit` : le drapeau doit suivre"
        );
    }
}
