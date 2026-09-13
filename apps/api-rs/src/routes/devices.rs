//! Les appareils d'un compte.
//!
//! Un appareil se déclare une fois, puis se re-déclare à chaque changement de
//! jeton APNs. L'enregistrement est donc idempotent par `vendorId` : c'est
//! l'identifiant stable que fournit iOS, et deux déclarations successives
//! doivent mettre à jour la même ligne, pas en créer une seconde.

use crate::messages::Msg;
use crate::{
    AppState,
    auth::Authentifie,
    cache,
    entities::{devices, live_activity_sessions},
    error::{AppError, introuvable, invalide},
    live_activity,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get, post, put},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use serde_json::{Value, json};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/devices", put(declarer))
        .route("/v1/live-activity/sessions", post(declarer_activite))
        .route(
            "/v1/live-activity/sessions/{token}",
            delete(terminer_activite),
        )
        .route("/v1/live-activity/state", get(etat_activite))
        .route("/v1/live-activity/start", post(demarrer_activite))
        .route("/v1/watch/summary", get(resume_montre))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeclarationActivite {
    vendor_id: String,
    update_token: String,
}

/// Déclarer une Live Activity en cours.
///
/// L'application transmet le jeton de mise à jour que lui donne ActivityKit ;
/// sans lui, le serveur n'a aucun moyen de pousser l'état.
async fn declarer_activite(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Json(corps): Json<DeclarationActivite>,
) -> Result<Json<Value>, AppError> {
    if corps.vendor_id.len() < 4 || corps.vendor_id.len() > 128 {
        return Err(invalide(Msg::IdentifiantDAppareilInvalide));
    }
    if corps.update_token.len() < 10 || corps.update_token.len() > 400 {
        return Err(invalide(Msg::JetonDeMiseAJourInvalide));
    }

    let appareil = devices::Entity::find()
        .filter(devices::Column::AccountId.eq(compte.id.as_str()))
        .filter(devices::Column::VendorId.eq(corps.vendor_id.as_str()))
        .one(&state.db)
        .await?
        .ok_or_else(|| introuvable(Msg::AppareilInconnu))?;

    let peremption = live_activity::peremption_session();

    // Idempotent par jeton : ActivityKit re-déclare la même activité après un
    // redémarrage, et cela doit mettre à jour la ligne, pas en créer une
    // seconde.
    let existante = live_activity_sessions::Entity::find()
        .filter(live_activity_sessions::Column::UpdateToken.eq(corps.update_token.as_str()))
        .one(&state.db)
        .await?;

    match existante {
        Some(ligne) => {
            let mut maj: live_activity_sessions::ActiveModel = ligne.into();
            maj.stale_at = Set(peremption);
            maj.ended_at = Set(None);
            maj.update(&state.db).await?;
        }
        None => {
            live_activity_sessions::ActiveModel {
                id: Set(cuid2::create_id()),
                account_id: Set(compte.id.clone()),
                device_id: Set(appareil.id),
                update_token: Set(corps.update_token),
                started_at: Set(Utc::now().naive_utc()),
                last_state_json: Set(String::new()),
                last_push_at: Set(None),
                ended_at: Set(None),
                stale_at: Set(peremption),
            }
            .insert(&state.db)
            .await?;
        }
    }

    let etat = live_activity::publier(&state, &compte.id).await?;
    Ok(Json(json!({ "ok": true, "state": etat })))
}

/// Signaler la fin d'une Live Activity.
async fn terminer_activite(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Path(jeton): Path<String>,
) -> Result<Json<Value>, AppError> {
    // Filtré sur le compte : un jeton ne ferme que les activités de qui le
    // présente.
    for session in live_activity_sessions::Entity::find()
        .filter(live_activity_sessions::Column::AccountId.eq(compte.id.as_str()))
        .filter(live_activity_sessions::Column::UpdateToken.eq(jeton.as_str()))
        .filter(live_activity_sessions::Column::EndedAt.is_null())
        .all(&state.db)
        .await?
    {
        // La ligne part, elle n'est pas seulement marquée close.
        //
        // La politique de confidentialité promet que les activités en direct
        // sont « effacées dès la fin de l'activité ». Poser `ended_at` et
        // attendre la purge laissait vivre `last_state_json` — l'instantané de
        // ce qui s'est affiché sur un écran verrouillé — jusqu'à la péremption
        // de la session, soit des heures après que la personne l'a fermée.
        live_activity_sessions::Entity::delete_by_id(session.id)
            .exec(&state.db)
            .await?;
    }

    Ok(Json(json!({ "ok": true })))
}

/// Lire l'état courant de la Live Activity, pour l'initialiser côté
/// application au premier lancement.
async fn etat_activite(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
) -> Result<Json<Value>, AppError> {
    let etat = live_activity::publier(&state, &compte.id).await?;
    Ok(Json(
        serde_json::to_value(etat).unwrap_or_else(|_| json!({})),
    ))
}

/// Démarrer la Live Activity à distance, par « push-to-start ».
async fn demarrer_activite(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
) -> Result<Json<Value>, AppError> {
    let demarrees = live_activity::demarrer_pour(&state, &compte.id).await?;
    Ok(Json(json!({ "ok": true, "started": demarrees })))
}

/// Résumé pour Apple Watch : le prochain plan et deux compteurs. Aucun nom,
/// aucune photo, aucun message.
async fn resume_montre(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
) -> Result<Json<Value>, AppError> {
    // La montre lit d'abord le résumé en cache : c'est une requête fréquente,
    // depuis un appareil dont la connexion et la batterie sont contraintes.
    if let Some(resume) =
        cache::lire_json::<Value>(&state.cache, &cache::cles::montre(&compte.id)).await
    {
        return Ok(Json(resume));
    }

    // Absent du cache : le recalcul repeuple la clé au passage.
    live_activity::publier(&state, &compte.id).await?;
    let resume = cache::lire_json::<Value>(&state.cache, &cache::cles::montre(&compte.id))
        .await
        // Le cache peut être indisponible ; la montre doit quand même recevoir
        // une charge lisible plutôt qu'une erreur.
        .unwrap_or_else(|| {
            json!({
                "pendingRequests": 0,
                "awaitingReply": 0,
                "nextPlan": null,
                "generatedAt": crate::temps::iso8601(Utc::now()),
            })
        });
    Ok(Json(resume))
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
        return Err(invalide(Msg::IdentifiantDAppareilInvalide));
    }

    // Cet appareil n'appartient plus à personne d'autre.
    //
    // L'index unique porte sur (compte, appareil) : le même téléphone peut
    // donc figurer sous plusieurs comptes, et la déconnexion ne touche pas la
    // ligne d'appareil — elle ne révoque que les jetons de session.
    //
    // Quelqu'un se déconnecte, quelqu'un d'autre se connecte sur le même
    // téléphone : la ligne du premier survit, avec le même jeton de poussée,
    // puisque ce jeton appartient à l'appareil et non au compte. Les alertes
    // du premier continuaient donc d'arriver sur un téléphone qui n'est plus
    // le sien — « Nouveau message », sur l'écran verrouillé d'un inconnu.
    //
    // Un jeton de poussée désigne un appareil physique : un seul compte peut
    // le détenir à la fois. Les lignes des autres sont donc muettes — elles
    // sont conservées, mais elles ne visent plus rien.
    oublier_ailleurs(&state, &compte.id, &corps.vendor_id).await?;

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

/// Retire les jetons de poussée de cet appareil sur tous les autres comptes.
///
/// Une seule écriture conditionnelle. Les lignes ne sont pas supprimées : elles
/// gardent la trace qu'un compte a utilisé cet appareil, ce qui sert à la
/// purge comme à l'export. Seul ce qui permet de pousser s'en va.
async fn oublier_ailleurs(
    state: &AppState,
    compte_id: &str,
    vendor_id: &str,
) -> Result<(), AppError> {
    let oubliees = devices::Entity::update_many()
        .col_expr(
            devices::Column::ApnsToken,
            sea_orm::sea_query::Expr::value(Option::<String>::None),
        )
        .col_expr(
            devices::Column::PushToStartToken,
            sea_orm::sea_query::Expr::value(Option::<String>::None),
        )
        .filter(devices::Column::VendorId.eq(vendor_id))
        .filter(devices::Column::AccountId.ne(compte_id))
        .filter(
            sea_orm::Condition::any()
                .add(devices::Column::ApnsToken.is_not_null())
                .add(devices::Column::PushToStartToken.is_not_null()),
        )
        .exec(&state.db)
        .await?;

    if oubliees.rows_affected > 0 {
        tracing::info!(
            comptes = oubliees.rows_affected,
            "appareil repris : jetons de poussée retirés des comptes précédents"
        );
    }
    Ok(())
}
