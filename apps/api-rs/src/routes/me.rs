//! Son propre compte : sa fiche, ses critères, sa photo.
//!
//! La fiche est volontairement maigre — une ville, un genre, une phrase. Dans
//! Weave, ce n'est pas la fiche qui donne envie, c'est le plan. On ne remplit
//! pas un formulaire pour se rendre désirable ; on écrit ce qu'on compte faire.

use crate::{
    auth::Authentifie,
    crypto::signer_url_media,
    droits::{credits_pour, demandes_restantes, quota_journalier},
    entities::{accounts, profiles},
    error::{introuvable, AppError},
    temps::{age_depuis, iso8601},
    AppState,
};
use axum::{extract::State, routing::get, Json, Router};
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Serialize;
use serde_json::Value;

pub fn routes() -> Router<AppState> {
    Router::new().route("/v1/me", get(lire))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Me {
    pub id: String,
    pub handle: String,
    pub display_name: String,
    pub age: i32,
    pub status: String,
    pub tier: String,
    pub city: String,
    pub bio: String,
    pub photo_url: Option<String>,
    pub verified: bool,
    pub requests_left_today: i64,
    pub credits: Value,
    pub created_at: String,
}

async fn lire(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
) -> Result<Json<Me>, AppError> {
    let ligne = accounts::Entity::find_by_id(compte.id.clone())
        .one(&state.db)
        .await?
        .ok_or_else(|| introuvable("Compte introuvable."))?;

    let profil = profiles::Entity::find()
        .filter(profiles::Column::AccountId.eq(compte.id.as_str()))
        .one(&state.db)
        .await?;

    let quota = quota_journalier(&state, &compte).await;

    Ok(Json(Me {
        id: ligne.id,
        handle: ligne.handle,
        display_name: ligne.display_name,
        age: age_depuis(ligne.birth_date.and_utc(), Utc::now()),
        status: ligne.status,
        tier: compte.tier.clone(),
        city: profil.as_ref().map(|p| p.city.clone()).unwrap_or_default(),
        bio: profil.as_ref().map(|p| p.bio.clone()).unwrap_or_default(),
        // La photo n'est jamais servie par une URL devinable : chaque lecture
        // passe par une signature à durée limitée.
        photo_url: profil.as_ref().and_then(|p| p.photo_key.as_ref()).map(|cle| {
            signer_url_media(
                &state.config.media.base_url,
                &state.config.media.signing_secret,
                cle,
                state.config.media.ttl_url_signee_secondes,
                0,
            )
        }),
        verified: ligne.verified,
        requests_left_today: demandes_restantes(&state, &compte, quota).await,
        credits: serde_json::to_value(credits_pour(&state, &compte.id).await?)
            .unwrap_or_else(|_| Value::Object(Default::default())),
        // `toISOString()` de JavaScript rend « ...517Z », pas « ...517+00:00 ».
        // Le site et l'application iOS lisent ce champ tel quel.
        created_at: iso8601(ligne.created_at.and_utc()),
    }))
}
