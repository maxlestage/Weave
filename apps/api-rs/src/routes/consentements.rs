//! Les consentements : ce que la politique de confidentialité promettait sans
//! que rien ne l'écrive.
//!
//! Les personnes que l'on cherche, rapprochées de son propre genre, peuvent
//! révéler l'orientation sexuelle. Le règlement européen range cette
//! information parmi les catégories particulières de l'article 9 : elle ne
//! peut être traitée que sur un **consentement explicite et distinct**.
//!
//! La page « Confidentialité » le promet en toutes lettres — « ce consentement
//! vous est demandé séparément, jamais par une case unique valant acceptation
//! de tout le reste », « vous pouvez le retirer à tout moment depuis
//! l'application », « le retrait est enregistré avec sa date ».
//!
//! `consent_records` existait pour cela, avec son objet, sa version, sa date
//! d'octroi et sa date de retrait. **Rien ne l'écrivait.** Aucune route, aucun
//! parcours. Le critère de genre s'enregistrait sans qu'aucun consentement
//! n'ait jamais été demandé, et l'export « vos consentements » rendait une
//! liste vide à tout le monde. Trois promesses, zéro ligne en base.
//!
//! ## Un consentement porte une version
//!
//! La politique promet qu'en cas de changement substantiel, « un nouveau
//! consentement vous est demandé ». Un consentement donné sur une version
//! antérieure cesse donc de valoir, et le traitement s'arrête — sans que
//! personne ait à repasser sur les comptes existants. `POLICY_VERSION`, dans
//! les contrats partagés, est ce qui déclenche cet arrêt.
//!
//! ## Retirer, c'est arrêter le traitement
//!
//! Un retrait qui laisserait le critère en base ferait durer le traitement
//! sans base légale — et la promesse « le service continue de fonctionner,
//! avec un fil non filtré sur ce critère » dit exactement quoi faire :
//! le critère est effacé. La trace du consentement, elle, reste : c'est la
//! période d'activité qui permet d'établir que le traitement était licite
//! quand il a eu lieu.

use crate::{
    auth::Authentifie,
    entities::{consent_records, preferences},
    error::{invalide, AppError},
    temps::iso8601,
    AppState,
};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder,
    Set, TransactionTrait,
};
use serde::Deserialize;
use serde_json::{json, Value};

/// Les objets sur lesquels un consentement distinct est demandé.
/// `packages/contracts` fait foi : `CONSENT_KINDS`.
pub const OBJETS: [&str; 1] = [DONNEES_SENSIBLES];

/// Le critère de genre, et lui seul, relève de l'article 9.
pub const DONNEES_SENSIBLES: &str = "donnees_sensibles";

/// La version des textes en vigueur.
/// `packages/contracts` fait foi : `POLICY_VERSION`.
pub const VERSION_POLITIQUE: &str = "2026-09-12";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/me/consents", get(lire).post(poser))
        .route("/v1/me/consents/revoke", post(retirer))
}

/// L'état de chaque objet, y compris ceux jamais consentis.
///
/// La liste entière, toujours : une clé absente se lirait comme une erreur du
/// côté de l'application, et « jamais demandé » est une réponse, pas un trou.
async fn lire(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
) -> Result<Json<Value>, AppError> {
    let mut rendus = Vec::with_capacity(OBJETS.len());
    for objet in OBJETS {
        let dernier = dernier_enregistrement(&state.db, &compte.id, objet).await?;
        rendus.push(json!({
            "kind": objet,
            // `active` est la seule valeur dont dépend un traitement. Les
            // autres champs racontent l'histoire ; celle-ci décide.
            "active": dernier.as_ref().is_some_and(actif),
            "version": dernier.as_ref().map(|c| c.version.clone()),
            "grantedAt": dernier.as_ref().map(|c| iso8601(c.granted_at.and_utc())),
            "revokedAt": dernier
                .as_ref()
                .and_then(|c| c.revoked_at)
                .map(|d| iso8601(d.and_utc())),
        }));
    }
    Ok(Json(json!({
        "consents": rendus,
        "policyVersion": VERSION_POLITIQUE,
    })))
}

#[derive(Deserialize)]
struct Objet {
    kind: String,
    /// La version du texte affiché au moment du geste.
    ///
    /// Exigée, et comparée : consentir « à la politique » sans dire laquelle
    /// ne prouve rien, et c'est précisément ce qu'un registre de consentements
    /// doit pouvoir établir.
    version: Option<String>,
}

async fn poser(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Json(corps): Json<Objet>,
) -> Result<Json<Value>, AppError> {
    let objet = valider_objet(&corps.kind)?;
    let version = corps.version.unwrap_or_default();
    if version != VERSION_POLITIQUE {
        return Err(invalide(
            "Ce consentement porte sur une version du texte qui n'est plus en vigueur.",
        ));
    }

    let transaction = state.db.begin().await?;

    // Un consentement déjà actif n'est pas redoublé : la date d'octroi est
    // celle du premier oui, et la réécrire raccourcirait la période établie.
    if dernier_enregistrement(&transaction, &compte.id, objet)
        .await?
        .as_ref()
        .is_some_and(actif)
    {
        transaction.commit().await?;
        return Ok(Json(json!({ "ok": true, "active": true })));
    }

    consent_records::ActiveModel {
        id: Set(cuid2::create_id()),
        account_id: Set(compte.id.clone()),
        kind: Set(objet.to_string()),
        version: Set(VERSION_POLITIQUE.to_string()),
        granted: Set(true),
        granted_at: Set(Utc::now().naive_utc()),
        revoked_at: Set(None),
    }
    .insert(&transaction)
    .await?;
    transaction.commit().await?;

    Ok(Json(json!({ "ok": true, "active": true })))
}

async fn retirer(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Json(corps): Json<Objet>,
) -> Result<Json<Value>, AppError> {
    let objet = valider_objet(&corps.kind)?;
    let transaction = state.db.begin().await?;

    // Le retrait est daté sur tout enregistrement encore ouvert de cet objet,
    // pas seulement le dernier : une ligne ouverte oubliée en arrière ferait
    // état d'un consentement sans fin.
    consent_records::Entity::update_many()
        .col_expr(
            consent_records::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(Utc::now().naive_utc()),
        )
        .filter(consent_records::Column::AccountId.eq(compte.id.as_str()))
        .filter(consent_records::Column::Kind.eq(objet))
        .filter(consent_records::Column::RevokedAt.is_null())
        .exec(&transaction)
        .await?;

    // Et le traitement s'arrête. Un retrait qui laisserait le critère en base
    // ferait durer un traitement de données sensibles sans base légale ; la
    // page publique promet d'ailleurs « un fil non filtré sur ce critère ».
    if objet == DONNEES_SENSIBLES {
        preferences::Entity::update_many()
            .col_expr(
                preferences::Column::SeekingJson,
                sea_orm::sea_query::Expr::value("[]"),
            )
            .col_expr(
                preferences::Column::UpdatedAt,
                sea_orm::sea_query::Expr::value(Utc::now().naive_utc()),
            )
            .filter(preferences::Column::AccountId.eq(compte.id.as_str()))
            .exec(&transaction)
            .await?;
    }

    transaction.commit().await?;

    // Le fil en cache a été composé avec le critère : il ne vaut plus rien.
    super::me::oublier_fil(&state, &compte.id).await;

    Ok(Json(json!({ "ok": true, "active": false })))
}

fn valider_objet(kind: &str) -> Result<&'static str, AppError> {
    OBJETS
        .into_iter()
        .find(|objet| *objet == kind)
        .ok_or_else(|| invalide("Objet de consentement inconnu."))
}

/// Vrai quand le consentement vaut encore : donné, non retiré, et sur la
/// version en vigueur.
///
/// Les trois conditions comptent. La troisième est celle qu'on oublie : la
/// politique promet qu'un changement substantiel fait redemander le
/// consentement, et un texte accepté il y a deux versions ne dit plus ce que
/// la personne a accepté.
fn actif(enregistrement: &consent_records::Model) -> bool {
    enregistrement.granted
        && enregistrement.revoked_at.is_none()
        && enregistrement.version == VERSION_POLITIQUE
}

async fn dernier_enregistrement<C: sea_orm::ConnectionTrait>(
    db: &C,
    compte: &str,
    objet: &str,
) -> Result<Option<consent_records::Model>, DbErr> {
    consent_records::Entity::find()
        .filter(consent_records::Column::AccountId.eq(compte))
        .filter(consent_records::Column::Kind.eq(objet))
        .order_by_desc(consent_records::Column::GrantedAt)
        .one(db)
        .await
}

/// Le consentement aux données sensibles vaut-il en ce moment ?
///
/// Lu par les critères, qui refusent d'enregistrer un genre recherché sans
/// lui, et par le fil, qui cesse de filtrer dessus quand il tombe.
pub async fn sensibles_autorisees(
    db: &DatabaseConnection,
    compte: &str,
) -> Result<bool, DbErr> {
    Ok(dernier_enregistrement(db, compte, DONNEES_SENSIBLES)
        .await?
        .as_ref()
        .is_some_and(actif))
}
