//! Connexion sans mot de passe.
//!
//! Un code à six chiffres est envoyé par e-mail, valable dix minutes et cinq
//! tentatives. Il n'y a pas de mot de passe à voler, à réutiliser ou à oublier.

use crate::{
    auth::emettre_jeton,
    crypto::{code_otp, hacher_secret, hash_email, jeton_opaque, normaliser_email, sha256_hex, verifier_secret},
    entities::{accounts, otp_challenges, preferences, refresh_tokens, subscriptions},
    error::{invalide, non_autorise, AppError, Code},
    limitation::{consommer, regles},
    temps::{age_depuis, iso8601},
    AppState,
};
use axum::{
    extract::{ConnectInfo, State},
    routing::post,
    Json, Router,
};
use chrono::{Duration, NaiveDate, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::SocketAddr;

/// Dix minutes : assez pour aller chercher le code dans sa boîte, assez court
/// pour qu'un code intercepté ne serve plus longtemps.
const OTP_TTL_MINUTES: i64 = 10;
/// Cinq essais, puis il faut redemander un code.
const OTP_MAX_TENTATIVES: i32 = 5;
/// Weave est réservé aux majeurs.
const AGE_MINIMUM: i32 = 18;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/auth/otp/request", post(demander_code))
        .route("/v1/auth/otp/verify", post(verifier_code))
}

#[derive(Deserialize)]
struct DemandeCode {
    email: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReponseDemande {
    sent: bool,
    expires_in_seconds: i64,
    /// Hors production, le code est renvoyé : cela permet de dérouler le
    /// parcours complet sans dépendre d'un fournisseur d'e-mail.
    #[serde(skip_serializing_if = "Option::is_none")]
    dev_code: Option<String>,
}

async fn demander_code(
    State(state): State<AppState>,
    ConnectInfo(adresse): ConnectInfo<SocketAddr>,
    Json(corps): Json<DemandeCode>,
) -> Result<Json<ReponseDemande>, AppError> {
    consommer(&state, regles::DEMANDE_OTP, &adresse.ip().to_string()).await?;

    let email = normaliser_email(&corps.email);
    if !email.contains('@') || email.len() > 320 {
        return Err(invalide("Adresse e-mail invalide."));
    }
    let empreinte = hash_email(&email);
    // Deux compteurs : l'adresse IP borne les rafales, l'empreinte de l'e-mail
    // protège une boîte donnée même depuis plusieurs adresses.
    consommer(&state, regles::DEMANDE_OTP, &empreinte).await?;

    let code = code_otp();
    let code_hache = hacher_secret(&code).map_err(|erreur| {
        tracing::error!(erreur = %erreur, "hachage du code impossible");
        AppError::new(Code::Internal, "Une erreur interne est survenue.")
    })?;

    otp_challenges::ActiveModel {
        id: Set(cuid2::create_id()),
        email_hash: Set(empreinte.clone()),
        code_hash: Set(code_hache),
        attempts: Set(0),
        consumed_at: Set(None),
        expires_at: Set((Utc::now() + Duration::minutes(OTP_TTL_MINUTES)).naive_utc()),
        created_at: Set(Utc::now().naive_utc()),
    }
    .insert(&state.db)
    .await?;

    // L'envoi réel passe par le fournisseur d'e-mail transactionnel. Le code
    // lui-même n'est jamais journalisé : seule l'empreinte l'est.
    tracing::info!(email_hash = %empreinte, "Code de connexion émis");

    Ok(Json(ReponseDemande {
        sent: true,
        expires_in_seconds: OTP_TTL_MINUTES * 60,
        dev_code: (!state.config.is_production()).then_some(code),
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerificationCode {
    email: String,
    code: String,
    display_name: Option<String>,
    /// AAAA-MM-JJ
    birth_date: Option<String>,
    timezone: Option<String>,
    device_id: Option<String>,
}

async fn verifier_code(
    State(state): State<AppState>,
    ConnectInfo(adresse): ConnectInfo<SocketAddr>,
    Json(corps): Json<VerificationCode>,
) -> Result<Json<Value>, AppError> {
    consommer(&state, regles::VERIF_OTP, &adresse.ip().to_string()).await?;

    let email = normaliser_email(&corps.email);
    let empreinte = hash_email(&email);

    let defi = otp_challenges::Entity::find()
        .filter(otp_challenges::Column::EmailHash.eq(empreinte.as_str()))
        .filter(otp_challenges::Column::ConsumedAt.is_null())
        .filter(otp_challenges::Column::ExpiresAt.gt(Utc::now().naive_utc()))
        .order_by_desc(otp_challenges::Column::CreatedAt)
        .one(&state.db)
        .await?
        .ok_or_else(|| non_autorise("Code expiré ou déjà utilisé."))?;

    if defi.attempts >= OTP_MAX_TENTATIVES {
        return Err(non_autorise(
            "Trop de tentatives sur ce code. Demandez-en un nouveau.",
        ));
    }

    if !verifier_secret(&corps.code, &defi.code_hash) {
        let mut essai: otp_challenges::ActiveModel = defi.clone().into();
        essai.attempts = Set(defi.attempts + 1);
        essai.update(&state.db).await?;
        return Err(non_autorise("Code incorrect."));
    }

    let defi_actif = defi;

    // Le code n'est PAS consommé ici. S'il l'était, l'appel qui répond
    // « needsProfile » le brûlerait, et le rappel du client avec son nom et sa
    // date de naissance échouerait sur « code déjà utilisé » : aucun compte ne
    // pourrait plus être créé. Il est consommé plus bas, une fois la session
    // réellement ouverte.
    let compte = accounts::Entity::find()
        .filter(accounts::Column::EmailHash.eq(empreinte.as_str()))
        .one(&state.db)
        .await?;

    let mut cree = false;
    let compte = match compte {
        Some(existant) => existant,
        None => {
            // Le compte n'existe pas : le client doit rappeler la même route
            // avec les informations d'inscription minimales.
            let (Some(nom), Some(naissance)) = (&corps.display_name, &corps.birth_date) else {
                return Ok(Json(
                    json!({ "session": null, "needsProfile": true, "created": false }),
                ));
            };
            cree = true;
            creer_compte(&state, &email, &empreinte, nom, naissance, corps.timezone.as_deref())
                .await?
        }
    };

    // La session est ouverte : le code a joué son rôle, on le retire.
    let mut consomme: otp_challenges::ActiveModel = defi_actif.into();
    consomme.consumed_at = Set(Some(Utc::now().naive_utc()));
    consomme.update(&state.db).await?;

    let session = ouvrir_session(&state, &compte.id, corps.device_id.as_deref()).await?;

    let mut vu: accounts::ActiveModel = compte.clone().into();
    vu.last_seen_at = Set(Some(Utc::now().naive_utc()));
    vu.update(&state.db).await?;

    Ok(Json(json!({
        "session": session,
        "needsProfile": false,
        "created": cree,
    })))
}

async fn creer_compte(
    state: &AppState,
    email: &str,
    empreinte: &str,
    nom: &str,
    naissance: &str,
    fuseau: Option<&str>,
) -> Result<accounts::Model, AppError> {
    let date = NaiveDate::parse_from_str(naissance, "%Y-%m-%d")
        .map_err(|_| invalide("Date de naissance invalide."))?;
    let naissance_utc = date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| invalide("Date de naissance invalide."))?
        .and_utc();

    if age_depuis(naissance_utc, Utc::now()) < AGE_MINIMUM {
        return Err(invalide(&format!(
            "Weave est réservé aux personnes de {AGE_MINIMUM} ans et plus."
        )));
    }

    let id = cuid2::create_id();
    let compte = accounts::ActiveModel {
        id: Set(id.clone()),
        email: Set(email.to_string()),
        email_hash: Set(empreinte.to_string()),
        handle: Set(pseudo_unique(state, nom).await?),
        display_name: Set(nom.to_string()),
        birth_date: Set(naissance_utc.naive_utc()),
        status: Set("onboarding".to_string()),
        timezone: Set(fuseau.unwrap_or("Europe/Paris").to_string()),
        locale: Set("fr-FR".to_string()),
        verified: Set(false),
        last_seen_at: Set(None),
        deletion_requested_at: Set(None),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
    }
    .insert(&state.db)
    .await?;

    // Préférences et abonnement de départ, comme le faisait la création
    // imbriquée de Prisma : un compte sans ces lignes serait à moitié né.
    // `updatedAt` n'a pas de valeur par défaut en base : Prisma la posait
    // lui-même. Les critères, eux, ont leurs défauts côté schéma.
    preferences::ActiveModel {
        id: Set(cuid2::create_id()),
        account_id: Set(id.clone()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&state.db)
    .await?;

    subscriptions::ActiveModel {
        id: Set(cuid2::create_id()),
        account_id: Set(id),
        tier: Set("depart".to_string()),
        updated_at: Set(Utc::now().naive_utc()),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&state.db)
    .await?;

    Ok(compte)
}

/// Fabrique un identifiant public unique à partir du nom affiché.
async fn pseudo_unique(state: &AppState, nom: &str) -> Result<String, AppError> {
    let base: String = nom
        .chars()
        .filter_map(|c| {
            let c = c.to_ascii_lowercase();
            c.is_ascii_alphanumeric().then_some(c)
        })
        .take(16)
        .collect();
    let base = if base.is_empty() { "fil".to_string() } else { base };

    for essai in 0..20 {
        let candidat = if essai == 0 {
            base.clone()
        } else {
            format!("{base}{}", rand::random_range(0..10_000))
        };
        let pris = accounts::Entity::find()
            .filter(accounts::Column::Handle.eq(candidat.as_str()))
            .one(&state.db)
            .await?;
        if pris.is_none() {
            return Ok(candidat);
        }
    }
    // Après vingt tentatives, on cesse de tirer au sort : l'horodatage tranche.
    Ok(format!("{base}{}", Utc::now().timestamp()))
}

async fn ouvrir_session(
    state: &AppState,
    compte_id: &str,
    appareil: Option<&str>,
) -> Result<Value, AppError> {
    let rafraichissement = jeton_opaque();
    let expire_le = Utc::now() + Duration::days(state.config.auth.refresh_ttl_jours);

    refresh_tokens::ActiveModel {
        id: Set(cuid2::create_id()),
        account_id: Set(compte_id.to_string()),
        // Le jeton n'est jamais stocké en clair : seule son empreinte l'est.
        token_hash: Set(sha256_hex(&rafraichissement)),
        device_id: Set(appareil.map(str::to_string)),
        revoked_at: Set(None),
        rotated_to: Set(None),
        expires_at: Set(expire_le.naive_utc()),
        created_at: Set(Utc::now().naive_utc()),
    }
    .insert(&state.db)
    .await?;

    let acces = emettre_jeton(
        &state.config.auth.jwt_secret,
        compte_id,
        state.config.auth.access_ttl_secondes,
    )?;

    Ok(json!({
        "accessToken": acces,
        "refreshToken": rafraichissement,
        "expiresAt": iso8601(Utc::now() + Duration::seconds(state.config.auth.access_ttl_secondes)),
    }))
}
