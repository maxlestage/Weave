//! Connexion sans mot de passe.
//!
//! Un code à six chiffres est envoyé par e-mail, valable dix minutes et cinq
//! tentatives. Il n'y a pas de mot de passe à voler, à réutiliser ou à oublier.

use crate::{
    auth::{emettre_jeton, oublier_compte, Authentifie},
    cache,
    crypto::{code_otp, hacher_secret, hash_email, jeton_opaque, normaliser_email, sha256_hex, verifier_secret},
    entities::{audit_events, accounts, join_requests, otp_challenges, plans, preferences, refresh_tokens, subscriptions},
    error::{invalide, non_autorise, AppError, Code},
    limitation::{consommer, regles},
    temps::{age_depuis, iso8601},
    AppState,
};
use axum::{
    extract::{ConnectInfo, State},
    routing::{delete, post},
    Json, Router,
};
use chrono::{Duration, NaiveDate, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
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
/// Le délai légal avant purge des données d'un compte supprimé.
/// `packages/contracts/src/invariants.ts` fait foi : les deux doivent
/// s'accorder, puisque l'application iOS affiche ce nombre.
const PURGE_COMPTE_JOURS: i64 = 30;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/auth/otp/request", post(demander_code))
        .route("/v1/auth/otp/verify", post(verifier_code))
        .route("/v1/auth/refresh", post(renouveler))
        .route("/v1/auth/logout", post(fermer_session))
        .route("/v1/auth/account", delete(supprimer_compte))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DemandeRenouvellement {
    refresh_token: String,
}

/// Renouvelle la session.
///
/// Le jeton présenté est immédiatement révoqué et remplacé — rotation. Une
/// réutilisation ultérieure du même jeton sera donc rejetée : c'est ce qui
/// distingue un jeton volé d'un jeton légitime, qui n'est présenté qu'une fois.
async fn renouveler(
    State(state): State<AppState>,
    ConnectInfo(adresse): ConnectInfo<std::net::SocketAddr>,
    Json(corps): Json<DemandeRenouvellement>,
) -> Result<Json<Value>, AppError> {
    let empreinte = sha256_hex(&corps.refresh_token);
    let maintenant = Utc::now().naive_utc();

    let stocke = refresh_tokens::Entity::find()
        .filter(refresh_tokens::Column::TokenHash.eq(empreinte.as_str()))
        .one(&state.db)
        .await?
        .ok_or_else(|| non_autorise("Session expirée. Reconnectez-vous."))?;

    // Un jeton déjà tourné qu'on nous représente : quelqu'un rejoue une vieille
    // copie. Le porteur légitime a rotationné depuis, donc soit c'est un vol,
    // soit une sauvegarde restaurée — dans les deux cas la session ne vaut plus
    // rien, et laisser vivre les jetons frères reviendrait à laisser la porte
    // ouverte à qui détient la copie.
    //
    // C'est à cela que sert `rotated_to`, écrit depuis toujours et jamais relu.
    // Sans cette lecture, la rotation ne protégeait de rien : un jeton volé
    // fonctionnait jusqu'à son expiration, et le vol ne se voyait jamais.
    if stocke.rotated_to.is_some() {
        revoquer_famille(&state, &stocke.account_id, adresse.ip()).await?;
        return Err(non_autorise("Session expirée. Reconnectez-vous."));
    }

    // La revendication, en une seule écriture conditionnelle.
    //
    // Lire puis écrire laissait deux renouvellements concurrents du MÊME jeton
    // passer tous deux le contrôle et ouvrir deux sessions indépendantes. Un
    // voleur n'avait qu'à courir contre le client légitime.
    let revendique = refresh_tokens::Entity::update_many()
        .col_expr(
            refresh_tokens::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(maintenant),
        )
        .filter(refresh_tokens::Column::TokenHash.eq(empreinte.as_str()))
        .filter(refresh_tokens::Column::RevokedAt.is_null())
        .filter(refresh_tokens::Column::ExpiresAt.gt(maintenant))
        .exec(&state.db)
        .await?;
    if revendique.rows_affected != 1 {
        return Err(non_autorise("Session expirée. Reconnectez-vous."));
    }

    let session = ouvrir_session(&state, &stocke.account_id, stocke.device_id.as_deref()).await?;

    let nouveau = session
        .get("refreshToken")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            tracing::error!("session ouverte sans jeton de renouvellement");
            AppError::new(Code::Internal, "Une erreur interne est survenue.")
        })?;

    // Le successeur, posé après coup : c'est lui qui permettra de reconnaître
    // un réemploi de ce jeton-ci.
    refresh_tokens::Entity::update_many()
        .col_expr(
            refresh_tokens::Column::RotatedTo,
            sea_orm::sea_query::Expr::value(sha256_hex(nouveau)),
        )
        .filter(refresh_tokens::Column::TokenHash.eq(empreinte.as_str()))
        .exec(&state.db)
        .await?;

    Ok(Json(json!({ "session": session })))
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct DemandeFermeture {
    /// Absent : toutes les sessions du compte sont révoquées.
    refresh_token: Option<String>,
}

/// Ferme la session — celle qu'on présente, ou toutes.
async fn fermer_session(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    corps: Option<Json<DemandeFermeture>>,
) -> Result<Json<Value>, AppError> {
    let corps = corps.map(|Json(c)| c).unwrap_or_default();

    let mut a_revoquer = refresh_tokens::Entity::find()
        .filter(refresh_tokens::Column::AccountId.eq(compte.id.as_str()));

    a_revoquer = match &corps.refresh_token {
        Some(jeton) => {
            a_revoquer.filter(refresh_tokens::Column::TokenHash.eq(sha256_hex(jeton)))
        }
        None => a_revoquer.filter(refresh_tokens::Column::RevokedAt.is_null()),
    };

    for jeton in a_revoquer.all(&state.db).await? {
        let mut revoque: refresh_tokens::ActiveModel = jeton.into();
        revoque.revoked_at = Set(Some(Utc::now().naive_utc()));
        revoque.update(&state.db).await?;
    }

    oublier_compte(&state, &compte.id).await;
    Ok(Json(json!({ "ok": true })))
}

/// Demande la suppression du compte.
///
/// En deux temps : le compte sort immédiatement de la circulation, les données
/// sont purgées à l'issue du délai légal. Les plans ouverts disparaissent du
/// fil des autres tout de suite — un rendez-vous auquel personne ne répondra
/// ne doit plus être proposé.
async fn supprimer_compte(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
) -> Result<Json<Value>, AppError> {
    let ligne = accounts::Entity::find_by_id(compte.id.as_str())
        .one(&state.db)
        .await?
        .ok_or_else(|| non_autorise("Session expirée. Reconnectez-vous."))?;

    let mut sortant: accounts::ActiveModel = ligne.into();
    sortant.status = Set("deleting".to_string());
    sortant.deletion_requested_at = Set(Some(Utc::now().naive_utc()));
    sortant.updated_at = Set(Utc::now().naive_utc());
    sortant.update(&state.db).await?;

    for jeton in refresh_tokens::Entity::find()
        .filter(refresh_tokens::Column::AccountId.eq(compte.id.as_str()))
        .filter(refresh_tokens::Column::RevokedAt.is_null())
        .all(&state.db)
        .await?
    {
        let mut revoque: refresh_tokens::ActiveModel = jeton.into();
        revoque.revoked_at = Set(Some(Utc::now().naive_utc()));
        revoque.update(&state.db).await?;
    }

    let ouverts: Vec<String> = plans::Entity::find()
        .filter(plans::Column::AuthorId.eq(compte.id.as_str()))
        .filter(plans::Column::State.eq("ouvert"))
        .select_only()
        .column(plans::Column::Id)
        .into_tuple()
        .all(&state.db)
        .await?;

    for plan in plans::Entity::find()
        .filter(plans::Column::AuthorId.eq(compte.id.as_str()))
        .filter(plans::Column::State.eq("ouvert"))
        .all(&state.db)
        .await?
    {
        let mut annule: plans::ActiveModel = plan.into();
        annule.state = Set("annule".to_string());
        annule.cancelled_at = Set(Some(Utc::now().naive_utc()));
        annule.updated_at = Set(Utc::now().naive_utc());
        annule.update(&state.db).await?;
    }

    // Les demandes encore en attente sur ces plans n'ont plus d'interlocuteur :
    // les laisser « envoyée » retiendrait indéfiniment le quota de qui les a
    // écrites.
    if !ouverts.is_empty() {
        for demande in join_requests::Entity::find()
            .filter(join_requests::Column::PlanId.is_in(ouverts))
            .filter(join_requests::Column::State.eq("envoyee"))
            .all(&state.db)
            .await?
        {
            let mut expiree: join_requests::ActiveModel = demande.into();
            expiree.state = Set("expiree".to_string());
            expiree.decided_at = Set(Some(Utc::now().naive_utc()));
            expiree.update(&state.db).await?;
        }
    }

    if let Err(erreur) = cache::oublier(&state.cache, &cache::cles::fil(&compte.id)).await {
        tracing::warn!(erreur = %erreur, "fil non invalidé");
    }
    oublier_compte(&state, &compte.id).await;

    Ok(Json(json!({ "ok": true, "purgeAfterDays": PURGE_COMPTE_JOURS })))
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

/// Révoque toutes les sessions vivantes d'un compte, et consigne pourquoi.
///
/// Appelée quand un jeton déjà tourné est représenté. On ne sait pas lequel des
/// deux porteurs est le légitime — celui qui a rotationné, ou celui qui rejoue
/// — et c'est précisément pourquoi on coupe tout : la personne se reconnecte,
/// le voleur n'a plus rien. Une demi-mesure laisserait vivre la session du
/// voleur si c'est lui qui a rotationné le dernier.
///
/// L'incident est écrit dans `audit_events`. Cette table existait sans qu'une
/// seule ligne n'y soit jamais posée, alors que la politique de
/// confidentialité annonce des « traces techniques : identifiant de compte,
/// action, adresse IP ». Un vol de session est exactement ce qu'on veut
/// pouvoir établir après coup.
async fn revoquer_famille(
    state: &AppState,
    compte_id: &str,
    ip: std::net::IpAddr,
) -> Result<(), AppError> {
    let maintenant = Utc::now().naive_utc();

    let coupees = refresh_tokens::Entity::update_many()
        .col_expr(
            refresh_tokens::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(maintenant),
        )
        .filter(refresh_tokens::Column::AccountId.eq(compte_id))
        .filter(refresh_tokens::Column::RevokedAt.is_null())
        .exec(&state.db)
        .await?;

    audit_events::ActiveModel {
        id: Set(cuid2::create_id()),
        account_id: Set(Some(compte_id.to_string())),
        action: Set("refresh_reuse".to_string()),
        subject: Set(None),
        meta_json: Set(
            json!({ "sessionsRevoquees": coupees.rows_affected }).to_string(),
        ),
        ip: Set(Some(ip.to_string())),
        created_at: Set(maintenant),
    }
    .insert(&state.db)
    .await?;

    tracing::warn!(
        compte = compte_id,
        sessions = coupees.rows_affected,
        "jeton de renouvellement rejoué : toutes les sessions sont coupées"
    );

    oublier_compte(state, compte_id).await;
    Ok(())
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
