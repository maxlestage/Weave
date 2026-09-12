//! Service des médias, sous URL signée.
//!
//! Une photo n'est jamais accessible par une URL devinable, ni servie nette
//! avant que la révélation progressive l'autorise : le niveau de flou est
//! inscrit dans la signature, donc impossible à modifier côté client.

use crate::{
    crypto::verifier_signature_media,
    error::{AppError, Code},
    AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::header,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

pub fn routes() -> Router<AppState> {
    Router::new().route("/media/{key}", get(servir))
}

#[derive(Deserialize)]
struct Signature {
    exp: String,
    sig: String,
    blur: Option<String>,
}

async fn servir(
    State(state): State<AppState>,
    Path(cle): Path<String>,
    Query(params): Query<Signature>,
) -> Result<Response, AppError> {
    // Une expiration ou un flou illisibles ne sont pas des valeurs par défaut :
    // les accepter reviendrait à servir un média dont la signature ne couvre
    // pas ce qu'on rend.
    let expire_le: i64 = params
        .exp
        .parse()
        .map_err(|_| AppError::new(Code::Forbidden, "Lien média expiré ou invalide."))?;
    let flou: u32 = match params.blur.as_deref() {
        None => 0,
        Some(v) => v
            .parse()
            .map_err(|_| AppError::new(Code::Forbidden, "Lien média expiré ou invalide."))?,
    };

    if !verifier_signature_media(
        &state.config.media.signing_secret,
        &cle,
        expire_le,
        flou,
        &params.sig,
    ) {
        return Err(AppError::new(
            Code::Forbidden,
            "Lien média expiré ou invalide.",
        ));
    }

    // En production, la requête est relayée vers le stockage objet, qui
    // applique la transformation de flou correspondant au paramètre signé.
    let corps = Json(json!({
        "key": cle,
        "blur": flou,
        "note": "Le relais vers le stockage objet est branché au déploiement.",
    }));

    Ok((
        [(header::CACHE_CONTROL, "private, max-age=60")],
        corps,
    )
        .into_response())
}
