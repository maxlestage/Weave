//! Les appareils d'un compte.
//!
//! Un appareil se déclare une fois, puis se re-déclare à chaque changement de
//! jeton APNs. L'enregistrement est donc idempotent par `vendorId` : c'est
//! l'identifiant stable que fournit iOS, et deux déclarations successives
//! doivent mettre à jour la même ligne, pas en créer une seconde.

use crate::{
    auth::Authentifie,
    entities::devices,
    error::{invalide, AppError},
    AppState,
};
use axum::{extract::State, routing::put, Json, Router};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn routes() -> Router<AppState> {
    Router::new().route("/v1/devices", put(declarer))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeclarationAppareil {
    vendor_id: String,
    platform: String,
    model: Option<String>,
    os_version: Option<String>,
    app_version: Option<String>,
    apns_token: Option<String>,
    push_to_start_token: Option<String>,
    apns_environment: Option<String>,
}

async fn declarer(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Json(corps): Json<DeclarationAppareil>,
) -> Result<Json<Value>, AppError> {
    if corps.vendor_id.len() < 4 || corps.vendor_id.len() > 128 {
        return Err(invalide("Identifiant d'appareil invalide."));
    }

    let existant = devices::Entity::find()
        .filter(devices::Column::AccountId.eq(compte.id.as_str()))
        .filter(devices::Column::VendorId.eq(corps.vendor_id.as_str()))
        .one(&state.db)
        .await?;

    let id = match existant {
        Some(ligne) => {
            let id = ligne.id.clone();
            let mut maj: devices::ActiveModel = ligne.into();
            maj.platform = Set(corps.platform);
            // Un champ absent de la requête n'efface pas ce qui est en base :
            // l'application ne renvoie pas toujours tout, et un jeton APNs
            // effacé par mégarde couperait les notifications sans rien dire.
            if let Some(v) = corps.model {
                maj.model = Set(Some(v));
            }
            if let Some(v) = corps.os_version {
                maj.os_version = Set(Some(v));
            }
            if let Some(v) = corps.app_version {
                maj.app_version = Set(Some(v));
            }
            if let Some(v) = corps.apns_token {
                maj.apns_token = Set(Some(v));
            }
            if let Some(v) = corps.push_to_start_token {
                maj.push_to_start_token = Set(Some(v));
            }
            if let Some(v) = corps.apns_environment {
                maj.apns_environment = Set(v);
            }
            maj.last_seen_at = Set(Utc::now().naive_utc());
            maj.update(&state.db).await?;
            id
        }
        None => {
            let id = cuid2::create_id();
            devices::ActiveModel {
                id: Set(id.clone()),
                account_id: Set(compte.id.clone()),
                vendor_id: Set(corps.vendor_id),
                platform: Set(corps.platform),
                model: Set(corps.model),
                os_version: Set(corps.os_version),
                app_version: Set(corps.app_version),
                apns_token: Set(corps.apns_token),
                push_to_start_token: Set(corps.push_to_start_token),
                apns_environment: Set(corps
                    .apns_environment
                    .unwrap_or_else(|| "sandbox".to_string())),
                last_seen_at: Set(Utc::now().naive_utc()),
                created_at: Set(Utc::now().naive_utc()),
            }
            .insert(&state.db)
            .await?;
            id
        }
    };

    Ok(Json(json!({ "ok": true, "deviceId": id })))
}
