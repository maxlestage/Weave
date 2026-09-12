//! Les conversations, ouvertes par une demande acceptée.
//!
//! Sans accusé de lecture : savoir si l'autre a lu n'aide personne à décider.

use crate::{
    auth::Authentifie,
    entities::{conversations, messages},
    error::{introuvable, invalide, AppError, Code},
    limitation::{consommer, Regle},
    temps::iso8601,
    AppState,
};
use axum::{
    extract::{Path, State},
    routing::post,
    Json, Router,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set, TransactionTrait};
use serde::Deserialize;
use serde_json::{json, Value};

/// Borne haute très large, uniquement anti-abus : une conversation entamée ne
/// doit jamais buter sur un plafond.
const REGLE_MESSAGE: Regle = Regle {
    seau: "message",
    limite: 240,
    fenetre_secondes: 60 * 60,
};

const MESSAGE_MAX: usize = 2000;

pub fn routes() -> Router<AppState> {
    Router::new().route("/v1/conversations/{id}/messages", post(ecrire))
}

#[derive(Deserialize)]
struct NouveauMessage {
    body: String,
}

async fn ecrire(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Path(id): Path<String>,
    Json(corps): Json<NouveauMessage>,
) -> Result<Json<Value>, AppError> {
    consommer(&state, REGLE_MESSAGE, &compte.id).await?;

    let conversation = conversation_de(&state, &id, &compte.id).await?;
    if conversation.closed_at.is_some() {
        return Err(invalide("Cette conversation est close."));
    }

    let texte = corps.body.trim().to_string();
    if texte.is_empty() {
        return Err(invalide("Un message vide ne dit rien."));
    }
    if texte.chars().count() > MESSAGE_MAX {
        return Err(invalide(&format!(
            "Un message ne peut pas dépasser {MESSAGE_MAX} caractères."
        )));
    }

    // Les deux écritures vont ensemble : une conversation dont `lastMessageAt`
    // ne suivrait pas remonterait au mauvais rang dans la liste.
    let transaction = state.db.begin().await?;

    let envoye = Utc::now();
    let message = messages::ActiveModel {
        id: Set(cuid2::create_id()),
        conversation_id: Set(conversation.id.clone()),
        author_id: Set(compte.id.clone()),
        body: Set(texte),
        sent_at: Set(envoye.naive_utc()),
        ..Default::default()
    }
    .insert(&transaction)
    .await?;

    let mut maj: conversations::ActiveModel = conversation.into();
    maj.last_message_at = Set(Some(message.sent_at));
    maj.update(&transaction).await?;

    transaction.commit().await?;

    Ok(Json(json!({
        "id": message.id,
        "conversationId": message.conversation_id,
        "author": "moi",
        "body": message.body,
        "sentAt": iso8601(message.sent_at.and_utc()),
        // Weave ne pose pas d'accusé de lecture : le champ existe pour le
        // contrat, il reste nul.
        "readAt": null,
    })))
}

/// Charge une conversation en vérifiant qu'on en fait partie.
async fn conversation_de(
    state: &AppState,
    id: &str,
    compte_id: &str,
) -> Result<conversations::Model, AppError> {
    let conversation = conversations::Entity::find_by_id(id.to_string())
        .one(&state.db)
        .await?
        .ok_or_else(|| introuvable("Conversation introuvable."))?;

    if conversation.host_id != compte_id && conversation.guest_id != compte_id {
        return Err(AppError::new(
            Code::Forbidden,
            "Cette conversation n'est pas la vôtre.",
        ));
    }
    Ok(conversation)
}
