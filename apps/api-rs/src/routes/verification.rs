//! Demander la vérification de son profil.
//!
//! `accounts.verified` s'écrit depuis la console de modération. Mais **rien ne
//! permettait de demander à l'être** : le badge ne pouvait se poser que sur
//! quelqu'un dont on aurait su, par un autre chemin, qu'il le voulait.
//!
//! Et le Grand Tour vend « Vérification de profil accélérée ». Une priorité
//! suppose une file d'attente, et il n'y en avait aucune.
//!
//! ## Aucune pièce d'identité ne passe par ici
//!
//! La demande ne porte qu'un texte libre, facultatif. La vérification se fait
//! ensuite par échange avec l'assistance.
//!
//! Faire transiter des papiers d'identité par l'application créerait une
//! réserve de données dont la perte serait irréparable — et pour un service qui
//! n'en a pas besoin : ce qu'on veut établir, c'est qu'il y a quelqu'un
//! derrière le profil, pas son état civil. Une base de données de pièces
//! d'identité est un actif pour qui la vole, et une responsabilité pour tout le
//! monde d'autre.

use crate::{
    AppState,
    auth::Authentifie,
    entities::verification_requests,
    error::{AppError, invalide},
    temps::iso8601,
};
use axum::{Json, Router, extract::State, routing::get};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder,
    Set,
};
use serde::Deserialize;
use serde_json::{Value, json};

pub const EN_ATTENTE: &str = "en_attente";
pub const ACCEPTEE: &str = "acceptee";
pub const REFUSEE: &str = "refusee";

/// Longueur du mot que l'on peut joindre. Une phrase, pas un dossier — la
/// vérification se poursuit par courrier, et rien d'identifiant n'a à être
/// déposé ici.
const NOTE_MAX: usize = 500;

pub fn routes() -> Router<AppState> {
    Router::new().route("/v1/me/verification", get(lire).post(demander))
}

/// Où en est ma demande, et le badge est-il posé ?
async fn lire(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
) -> Result<Json<Value>, AppError> {
    let derniere = derniere_demande(&state.db, &compte.id).await?;
    Ok(Json(json!({
        "verified": compte.verified,
        "request": derniere.map(|d| json!({
            "state": d.state,
            "createdAt": iso8601(d.created_at.and_utc()),
            "handledAt": d.handled_at.map(|h| iso8601(h.and_utc())),
            // Le motif d'un refus est rendu à qui il concerne : les mentions
            // légales promettent qu'une décision de modération se conteste, et
            // un refus dont on ignore la raison ne se conteste pas.
            "decision": d.decision,
        })),
    })))
}

#[derive(Deserialize)]
struct Demande {
    note: Option<String>,
}

async fn demander(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Json(corps): Json<Demande>,
) -> Result<Json<Value>, AppError> {
    if compte.verified {
        return Err(invalide("Votre profil est déjà vérifié."));
    }

    let note = corps.note.unwrap_or_default();
    if note.chars().count() > NOTE_MAX {
        return Err(invalide(&format!(
            "Ce mot ne peut pas dépasser {NOTE_MAX} caractères."
        )));
    }

    // Redemander pendant qu'une demande court ne crée pas de doublon : la file
    // se remplirait de la même personne, et la priorité vendue au Grand Tour
    // s'achèterait en appuyant plusieurs fois.
    if derniere_demande(&state.db, &compte.id)
        .await?
        .is_some_and(|en_cours| en_cours.state == EN_ATTENTE)
    {
        return Ok(Json(json!({ "ok": true, "state": EN_ATTENTE })));
    }

    verification_requests::ActiveModel {
        id: Set(cuid2::create_id()),
        account_id: Set(compte.id.clone()),
        state: Set(EN_ATTENTE.to_string()),
        note: Set(note),
        decision: Set(String::new()),
        created_at: Set(Utc::now().naive_utc()),
        handled_at: Set(None),
    }
    .insert(&state.db)
    .await?;

    Ok(Json(json!({ "ok": true, "state": EN_ATTENTE })))
}

async fn derniere_demande(
    db: &DatabaseConnection,
    compte: &str,
) -> Result<Option<verification_requests::Model>, DbErr> {
    verification_requests::Entity::find()
        .filter(verification_requests::Column::AccountId.eq(compte))
        .order_by_desc(verification_requests::Column::CreatedAt)
        .one(db)
        .await
}
