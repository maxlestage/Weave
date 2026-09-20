//! Service des médias, sous URL signée.
//!
//! Une photo n'est jamais accessible par une URL devinable, ni servie nette
//! avant que la révélation progressive l'autorise : le niveau de flou est
//! inscrit dans la signature, donc impossible à modifier côté client.

use crate::messages::Msg;
use crate::{
    AppState,
    auth::Authentifie,
    crypto::{signer_url_media, verifier_signature_media},
    entities::{media_objects, profiles},
    error::{AppError, Code, introuvable, invalide},
    limitation::{consommer, regles},
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::header,
    response::{IntoResponse, Response},
    routing::{get, put},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::Deserialize;
use serde_json::{Value, json};

/// Taille maximale d'une photo de profil.
///
/// Deux mégaoctets : de quoi loger une photo de téléphone recompressée sans
/// ouvrir la porte à des envois qui rempliraient la base. L'application
/// recompresse avant d'envoyer ; cette borne est là pour ce qui ne passe pas
/// par elle.
pub(crate) const PHOTO_MAX_OCTETS: usize = 2 * 1024 * 1024;

/// Les formats acceptés, reconnus à leurs octets de tête.
///
/// L'en-tête `Content-Type` de la requête n'est pas une preuve : il est écrit
/// par l'appelant. Ce qui est stocké — et donc ce qui sera resservi comme type
/// de contenu — est déduit du contenu lui-même. Sans cela, on servirait plus
/// tard un `image/jpeg` qui n'en est pas.
const SIGNATURES: [(&[u8], &str); 3] = [
    (&[0xFF, 0xD8, 0xFF], "image/jpeg"),
    (
        &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A],
        "image/png",
    ),
    // HEIC : « ....ftypheic », le type de marque au neuvième octet.
    (b"ftyp", "image/heic"),
];

pub fn routes() -> Router<AppState> {
    Router::new().route("/media/{key}", get(servir)).route(
        "/v1/me/photo",
        put(deposer_photo)
            .layer(DefaultBodyLimit::max(PHOTO_MAX_OCTETS))
            .delete(retirer_photo),
    )
}

/// Reconnaît le format d'une image à ses premiers octets, ou rien.
fn type_reconnu(octets: &[u8]) -> Option<&'static str> {
    for (signature, type_mime) in SIGNATURES {
        // HEIC porte sa marque après la taille de boîte : quatre octets.
        let debut = if type_mime == "image/heic" { 4 } else { 0 };
        if octets.len() > debut + signature.len() && octets[debut..].starts_with(signature) {
            return Some(type_mime);
        }
    }
    None
}

/// Dépose sa photo de profil.
///
/// Le corps est l'image elle-même, brute. Pas de `multipart` : il n'y a qu'un
/// fichier et aucun champ qui l'accompagne, et l'analyse d'un corps composite
/// serait du code en plus pour transporter la même chose.
async fn deposer_photo(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    corps: axum::body::Bytes,
) -> Result<Json<Value>, AppError> {
    let Some(type_mime) = type_reconnu(&corps) else {
        return Err(invalide(Msg::FormatDImageNonReconnu));
    };
    if corps.len() > PHOTO_MAX_OCTETS {
        return Err(invalide(Msg::ImageTropLourde));
    }

    consommer(&state, regles::PHOTO, &compte.id).await?;

    let fiche = profiles::Entity::find()
        .filter(profiles::Column::AccountId.eq(compte.id.as_str()))
        .one(&state.db)
        .await?
        .ok_or_else(|| invalide(Msg::RenseignezDAbordVotreVille))?;

    let ancienne = fiche.photo_key.clone();
    let cle = cuid2::create_id();

    let transaction = state.db.begin().await?;

    media_objects::ActiveModel {
        key: Set(cle.clone()),
        account_id: Set(compte.id.clone()),
        content_type: Set(type_mime.to_string()),
        byte_size: Set(corps.len() as i32),
        bytes: Set(corps.to_vec()),
        created_at: Set(Utc::now().naive_utc()),
    }
    .insert(&transaction)
    .await?;

    let mut maj: profiles::ActiveModel = fiche.into();
    maj.photo_key = Set(Some(cle.clone()));
    // Une photo remplacée n'a pas été relue : la colonne existe pour une
    // modération à venir, et la reconduire mentirait sur ce qui a été vu.
    maj.photo_reviewed_at = Set(None);
    maj.updated_at = Set(Utc::now().naive_utc());
    maj.update(&transaction).await?;

    // L'ancienne part avec la nouvelle, dans la même transaction : deux
    // écritures séparées laisseraient un orphelin si la seconde échouait, et
    // personne ne saurait plus qu'il est là.
    if let Some(precedente) = ancienne {
        media_objects::Entity::delete_by_id(precedente)
            .exec(&transaction)
            .await?;
    }

    transaction.commit().await?;

    Ok(Json(json!({
        "photoUrl": signer_url_media(
            &state.config.media.base_url,
            &state.config.media.signing_secret,
            &cle,
            state.config.media.ttl_url_signee_secondes,
            0,
        ),
    })))
}

/// Retire sa photo de profil.
///
/// ## Ce qui manquait
///
/// On pouvait déposer une photo, la remplacer, jamais la RETIRER. Une fois
/// posée, elle restait — sauf à supprimer tout son compte, ce qui est une
/// réponse démesurée à « je ne veux plus montrer mon visage ».
///
/// C'est pourtant la donnée la plus identifiante de la fiche, et la seule
/// qu'on ne pouvait pas reprendre. Tout le reste s'édite : la ville, le genre,
/// la phrase de présentation.
///
/// ## La machinerie existait, mais pas pour l'intéressé
///
/// `console::photo_retirer` la retire depuis la console de modération, efface
/// ses octets et consigne le geste. Un modérateur pouvait donc retirer votre
/// photo ; vous, non.
///
/// Cette route délègue à cette fonction plutôt que d'en réécrire une seconde.
/// Deux façons d'effacer une photo finiraient par ne plus effacer la même
/// chose — c'est déjà la raison pour laquelle `oublier_fil` est partagée.
///
/// Le geste est consigné comme celui d'un modérateur, et c'est volontaire :
/// une photo retirée juste avant sa relecture laisse une trace qui explique sa
/// disparition. L'entrée s'anonymise à la purge du compte, comme toutes les
/// autres.
///
/// ## Retirer deux fois ne se plaint pas
///
/// Une fiche sans photo est déjà dans l'état voulu. Rendre une erreur ferait
/// dépendre la réponse de ce qu'on ignorait — et l'appel vient souvent d'un
/// écran qui ne sait pas s'il y en avait une.
async fn retirer_photo(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
) -> Result<Json<Value>, AppError> {
    let issue = crate::console::photo_retirer(&state.db, &compte.id).await?;

    // Le fil des autres porte des adresses signées vers cette photo. Elles
    // pointeraient vers rien jusqu'à l'expiration du cache — quelques minutes
    // pendant lesquelles une fiche s'afficherait cassée.
    if issue == crate::console::Issue::Fait {
        crate::routes::me::oublier_fil(&state, &compte.id).await;
    }

    Ok(Json(json!({ "ok": true })))
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
        .map_err(|_| AppError::new(Code::Forbidden, Msg::LienMediaExpire.t()))?;
    let flou: u32 = match params.blur.as_deref() {
        None => 0,
        Some(v) => v
            .parse()
            .map_err(|_| AppError::new(Code::Forbidden, Msg::LienMediaExpire.t()))?,
    };

    if !verifier_signature_media(
        &state.config.media.signing_secret,
        &cle,
        expire_le,
        flou,
        &params.sig,
    ) {
        return Err(AppError::new(Code::Forbidden, Msg::LienMediaExpire.t()));
    }

    // Le flou n'est pas encore appliqué — et c'est un refus, pas un oubli.
    //
    // Le niveau est inscrit dans la signature pour qu'il ne puisse pas être
    // changé côté client : servir net une photo dont la signature réclamait un
    // flou reviendrait à ouvrir exactement ce que la signature protège. Aucun
    // appelant ne signe aujourd'hui autre chose que zéro ; le jour où la
    // révélation progressive s'écrira, elle butera ici plutôt que de découvrir
    // quelqu'un sans le vouloir.
    if flou > 0 {
        return Err(AppError::new(
            Code::Forbidden,
            Msg::FlouProgressifNonApplique.t(),
        ));
    }

    let objet = media_objects::Entity::find_by_id(cle)
        .one(&state.db)
        .await?
        .ok_or_else(|| introuvable(Msg::MediaDisparu))?;

    Ok((
        [
            (header::CONTENT_TYPE, objet.content_type),
            // Privé : l'URL est signée et personnelle. Une minute, comme la
            // durée de vie de la signature elle-même le suppose.
            (header::CACHE_CONTROL, "private, max-age=60".to_string()),
        ],
        objet.bytes,
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le type de contenu vient des octets, jamais de l'appelant.
    ///
    /// L'en-tête `Content-Type` d'une requête est écrit par qui l'envoie.
    /// S'y fier reviendrait à resservir plus tard un « image/jpeg » qui n'en
    /// est pas — et c'est le navigateur du destinataire qui déciderait quoi en
    /// faire.
    #[test]
    fn le_format_se_lit_dans_les_octets() {
        assert_eq!(
            type_reconnu(&[0xFF, 0xD8, 0xFF, 0xE0, 0x00]),
            Some("image/jpeg")
        );
        assert_eq!(
            type_reconnu(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00]),
            Some("image/png")
        );

        let mut heic = vec![0x00, 0x00, 0x00, 0x18];
        heic.extend_from_slice(b"ftypheic");
        assert_eq!(type_reconnu(&heic), Some("image/heic"));
    }

    /// Ce qui n'est pas une image est refusé, quel que soit son nom.
    #[test]
    fn ce_qui_n_est_pas_une_image_n_est_pas_reconnu() {
        for imposteur in [
            b"GIF89a".as_slice(),
            b"<?php echo 1; ?>".as_slice(),
            b"%PDF-1.7".as_slice(),
            b"".as_slice(),
            &[0xFF, 0xD8],
        ] {
            assert_eq!(
                type_reconnu(imposteur),
                None,
                "octets acceptés à tort : {imposteur:?}"
            );
        }
    }
}
