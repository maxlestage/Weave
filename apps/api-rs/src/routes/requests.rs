//! Demander à venir.
//!
//! Le message est obligatoire, et d'une longueur minimale : c'est ce qui
//! distingue une demande d'un geste. Chaque demande consomme une unité du
//! quota journalier — à tous les paliers, sans exception. C'est l'invariant
//! qui empêche d'arroser.

use crate::{
    auth::Authentifie,
    cache,
    crypto::signer_url_media,
    droits::{demandes_restantes, exiger_credit, quota_journalier, RENFORT_GRANT},
    entities::{accounts, blocks, conversations, join_requests, plans, profiles},
    error::{introuvable, invalide, AppError, Code},
    limitation::{consommer, regles},
    live_activity,
    temps::{age_depuis, iso8601, jour_local, secondes_avant_minuit},
    AppState,
};
use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};
use serde::Deserialize;
use serde_json::{json, Value};

/// Vingt caractères : assez pour dire pourquoi on veut venir, trop pour un
/// « salut » envoyé à la chaîne.
pub(crate) const MESSAGE_MIN: usize = 20;
pub(crate) const MESSAGE_MAX: usize = 600;

/// Au plus deux « Renforts » par jour. Le plafond journalier existe même en
/// payant : c'est ce qui distingue « on ne peut pas arroser » de « on ne peut
/// pas arroser gratuitement ». `packages/contracts` fait foi.
const MAX_RENFORTS_PAR_JOUR: i64 = 2;

/// Le plafond de demandes rendues. Une borne, pas une pagination.
const DEMANDES_RENDUES_MAX: u64 = 50;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/requests", post(demander))
        // « sent » et « renfort » avant « {id} » : ce sont des routes
        // littérales, elles ne doivent pas être lues comme des identifiants.
        .route("/v1/requests/sent", get(envoyees))
        .route("/v1/requests/renfort", post(appliquer_renfort))
        .route("/v1/requests/{id}", delete(retirer))
        .route("/v1/requests/{id}/accept", post(accepter))
        .route("/v1/requests/{id}/decline", post(refuser))
}

/// Mes demandes envoyées.
///
/// Sans accusé de lecture : savoir si l'autre a lu n'aide personne à décider,
/// et transformerait l'attente en surveillance.
async fn envoyees(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
) -> Result<Json<Value>, AppError> {
    let lignes = join_requests::Entity::find()
        .filter(join_requests::Column::AuthorId.eq(compte.id.as_str()))
        .order_by_desc(join_requests::Column::SentAt)
        .limit(DEMANDES_RENDUES_MAX)
        .all(&state.db)
        .await?;

    let mut rendues = Vec::with_capacity(lignes.len());
    for ligne in lignes {
        if let Some(demande) = vers_demande(&state, &ligne).await? {
            rendues.push(demande);
        }
    }

    let quota = quota_journalier(&state, &compte).await;
    Ok(Json(json!({
        "requests": rendues,
        "requestsLeftToday": demandes_restantes(&state, &compte, quota).await,
    })))
}

/// Une demande, telle que son auteur la voit : avec le plan visé et qui
/// l'organise.
///
/// Rend `None` si le plan ou son auteur a disparu : une demande orpheline ne
/// doit pas faire échouer la lecture de toute la liste.
async fn vers_demande(
    state: &AppState,
    ligne: &join_requests::Model,
) -> Result<Option<Value>, AppError> {
    let Some(plan) = plans::Entity::find_by_id(ligne.plan_id.as_str())
        .one(&state.db)
        .await?
    else {
        return Ok(None);
    };
    let Some(auteur) = accounts::Entity::find_by_id(plan.author_id.as_str())
        .one(&state.db)
        .await?
    else {
        return Ok(None);
    };

    let photo = profiles::Entity::find()
        .filter(profiles::Column::AccountId.eq(auteur.id.as_str()))
        .one(&state.db)
        .await?
        .and_then(|p| p.photo_key)
        .map(|cle| {
            signer_url_media(
                &state.config.media.base_url,
                &state.config.media.signing_secret,
                &cle,
                state.config.media.ttl_url_signee_secondes,
                0,
            )
        });

    let conversation: Option<String> = conversations::Entity::find()
        .filter(conversations::Column::RequestId.eq(ligne.id.as_str()))
        .select_only()
        .column(conversations::Column::Id)
        .into_tuple()
        .one(&state.db)
        .await?;

    Ok(Some(json!({
        "id": ligne.id,
        "planId": ligne.plan_id,
        "planTitle": plan.title,
        "planStartsAt": iso8601(plan.starts_at.and_utc()),
        "author": {
            "id": auteur.id,
            "displayName": auteur.display_name,
            "age": age_depuis(auteur.birth_date.and_utc(), Utc::now()),
            "photoUrl": photo,
            "verified": auteur.verified,
        },
        "message": ligne.message,
        "state": ligne.state,
        "sentAt": iso8601(ligne.sent_at.and_utc()),
        "decidedAt": ligne.decided_at.map(|d| iso8601(d.and_utc())),
        "conversationId": conversation,
    })))
}

/// Appliquer un « Renfort » : cinq demandes de plus aujourd'hui.
async fn appliquer_renfort(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
) -> Result<Json<Value>, AppError> {
    let jour = jour_local(&compte.timezone, Utc::now());

    let applique = cache::appliquer_renfort(
        &state.cache,
        &compte.id,
        &jour,
        MAX_RENFORTS_PAR_JOUR,
        secondes_avant_minuit(&compte.timezone, Utc::now()),
    )
    .await?;

    if applique.is_none() {
        return Err(invalide(&format!(
            "Au plus {MAX_RENFORTS_PAR_JOUR} renforts par jour. Vos demandes reviennent à minuit."
        )));
    }

    // La place est réservée avant que le crédit soit dépensé — l'inverse
    // consommerait un achat pour rien lorsque le plafond est atteint. Elle est
    // donc rendue si le crédit manque, sans quoi une tentative refusée
    // grignoterait le plafond du jour.
    if let Err(erreur) = exiger_credit(&state, &compte.id, "renfort", "Renfort").await {
        cache::rendre_renfort(&state.cache, &compte.id, &jour).await;
        return Err(erreur);
    }

    let quota = quota_journalier(&state, &compte).await;
    Ok(Json(json!({
        "ok": true,
        "granted": RENFORT_GRANT,
        "requestsLeftToday": demandes_restantes(&state, &compte, quota).await,
    })))
}

/// Retirer une demande. L'unité de quota est rendue.
async fn retirer(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Path(demande_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let demande = join_requests::Entity::find_by_id(demande_id.as_str())
        .one(&state.db)
        .await?
        .ok_or_else(|| introuvable("Demande introuvable."))?;

    if demande.author_id != compte.id {
        return Err(AppError::new(
            Code::Forbidden,
            "Cette demande n'est pas la vôtre.",
        ));
    }
    let envoyee_le = demande.sent_at;

    // Le retrait est UNE écriture, conditionnée sur l'état de départ.
    //
    // Lire l'état puis écrire laissait deux retraits concurrents de la même
    // demande franchir le contrôle ensemble, et REMBOURSER TOUS LES DEUX. Le
    // script Lua du cache borne le compteur à zéro — il empêche de passer sous
    // le plancher, pas de récupérer plus qu'on n'a dépensé : cinq retraits
    // simultanés d'une seule demande rendaient cinq unités, et le quota
    // journalier est l'invariant que le produit défend.
    //
    // Seul l'appel qui a réellement changé l'état rembourse.
    let retiree = join_requests::Entity::update_many()
        .col_expr(join_requests::Column::State, sea_orm::sea_query::Expr::value("retiree"))
        .col_expr(
            join_requests::Column::DecidedAt,
            sea_orm::sea_query::Expr::value(Utc::now().naive_utc()),
        )
        .filter(join_requests::Column::Id.eq(demande_id.as_str()))
        .filter(join_requests::Column::State.eq("envoyee"))
        .exec(&state.db)
        .await?;

    if retiree.rows_affected == 0 {
        return Err(invalide("Cette demande a déjà reçu une réponse."));
    }

    // Se raviser vite ne doit pas coûter la journée. Le jour est celui de
    // l'ENVOI, pas celui du retrait : sans quoi un retrait après minuit
    // créditerait une journée qu'on n'a pas entamée.
    cache::rendre_demande(
        &state.cache,
        &compte.id,
        &jour_local(&compte.timezone, envoyee_le.and_utc()),
    )
    .await;

    live_activity::publier_au_mieux(&state, &compte.id).await;

    Ok(Json(json!({ "ok": true })))
}

/// Refuser une demande.
///
/// L'auteur voit sa demande close, sans motif ni notification accusatrice.
/// L'unité de quota reste dépensée : elle a été lue.
async fn refuser(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Path(demande_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let demande = join_requests::Entity::find_by_id(demande_id.as_str())
        .one(&state.db)
        .await?
        .ok_or_else(|| introuvable("Demande introuvable."))?;

    let plan = plans::Entity::find_by_id(demande.plan_id.as_str())
        .one(&state.db)
        .await?
        .ok_or_else(|| introuvable("Plan introuvable."))?;

    if plan.author_id != compte.id {
        return Err(AppError::new(Code::Forbidden, "Ce plan n'est pas le vôtre."));
    }
    if demande.state != "envoyee" {
        return Err(invalide("Cette demande est déjà tranchée."));
    }

    let auteur_id = demande.author_id.clone();
    let mut refusee: join_requests::ActiveModel = demande.into();
    refusee.state = Set("refusee".to_string());
    refusee.decided_at = Set(Some(Utc::now().naive_utc()));
    refusee.update(&state.db).await?;

    // Les deux côtés changent : une demande de moins à trancher pour l'hôte,
    // une attente de moins pour qui l'avait écrite.
    live_activity::publier_au_mieux(&state, &compte.id).await;
    live_activity::publier_au_mieux(&state, &auteur_id).await;

    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NouvelleDemande {
    plan_id: String,
    message: String,
}

async fn demander(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Json(corps): Json<NouvelleDemande>,
) -> Result<Json<Value>, AppError> {
    consommer(&state, regles::DEMANDE, &compte.id).await?;

    let message = corps.message.trim().to_string();
    if message.chars().count() < MESSAGE_MIN {
        return Err(invalide(&format!(
            "Écrivez au moins {MESSAGE_MIN} caractères : c'est ce qui distingue une demande d'un geste."
        )));
    }
    if message.chars().count() > MESSAGE_MAX {
        return Err(invalide(&format!(
            "Le message ne peut pas dépasser {MESSAGE_MAX} caractères."
        )));
    }

    let plan = plans::Entity::find_by_id(corps.plan_id.clone())
        .one(&state.db)
        .await?
        .ok_or_else(|| introuvable("Ce plan n'existe plus."))?;

    if plan.author_id == compte.id {
        return Err(invalide("C'est votre propre plan."));
    }

    let auteur = accounts::Entity::find_by_id(plan.author_id.clone())
        .one(&state.db)
        .await?
        .ok_or_else(|| introuvable("Ce plan n'existe plus."))?;

    if plan.state != "ouvert" || auteur.status != "active" {
        return Err(AppError::new(
            Code::PlanClosed,
            "Ce plan n'accepte plus de demandes.",
        ));
    }
    if plan.starts_at.and_utc() <= Utc::now() {
        return Err(AppError::new(Code::PlanClosed, "Ce plan a déjà eu lieu."));
    }

    let acceptees = join_requests::Entity::find()
        .filter(join_requests::Column::PlanId.eq(plan.id.as_str()))
        .filter(join_requests::Column::State.eq("acceptee"))
        .count(&state.db)
        .await?;
    if acceptees >= plan.capacity as u64 {
        return Err(AppError::new(Code::PlanClosed, "Ce plan est complet."));
    }

    // Un blocage dans un sens ou dans l'autre rend la demande impossible, sans
    // dire lequel : on ne renseigne jamais quelqu'un sur son blocage. D'où le
    // même message que pour un plan disparu.
    let blocage = blocks::Entity::find()
        .filter(
            Condition::any()
                .add(
                    Condition::all()
                        .add(blocks::Column::AuthorId.eq(compte.id.as_str()))
                        .add(blocks::Column::TargetId.eq(plan.author_id.as_str())),
                )
                .add(
                    Condition::all()
                        .add(blocks::Column::AuthorId.eq(plan.author_id.as_str()))
                        .add(blocks::Column::TargetId.eq(compte.id.as_str())),
                ),
        )
        .one(&state.db)
        .await?;
    if blocage.is_some() {
        return Err(introuvable("Ce plan n'existe plus."));
    }

    let deja = join_requests::Entity::find()
        .filter(join_requests::Column::PlanId.eq(plan.id.as_str()))
        .filter(join_requests::Column::AuthorId.eq(compte.id.as_str()))
        .one(&state.db)
        .await?;
    if deja.is_some() {
        return Err(AppError::new(
            Code::AlreadyRequested,
            "Vous avez déjà demandé à venir. On ne redemande pas deux fois.",
        ));
    }

    // Le quota est prélevé AVANT l'écriture : si l'insertion échoue, la
    // demande est rendue. L'inverse laisserait une demande écrite gratuite.
    let quota = quota_journalier(&state, &compte).await;
    let jour = jour_local(&compte.timezone, Utc::now());
    let restantes = cache::consommer_demande(
        &state.cache,
        &compte.id,
        &jour,
        quota,
        secondes_avant_minuit(&compte.timezone, Utc::now()),
    )
    .await?
    .ok_or_else(|| {
        AppError::new(
            Code::NoRequestsLeft,
            format!("Vous avez utilisé vos {quota} demandes du jour. Elles reviennent à minuit."),
        )
    })?;

    let envoyee = Utc::now();
    let demande = join_requests::ActiveModel {
        id: Set(cuid2::create_id()),
        plan_id: Set(plan.id.clone()),
        author_id: Set(compte.id.clone()),
        message: Set(message),
        state: Set("envoyee".to_string()),
        sent_at: Set(envoyee.naive_utc()),
        ..Default::default()
    }
    .insert(&state.db)
    .await;

    let demande = match demande {
        Ok(ligne) => ligne,
        Err(erreur) => {
            // L'écriture a échoué : la demande prélevée n'a servi à rien.
            cache::rendre_demande(&state.cache, &compte.id, &jour).await;
            return Err(erreur.into());
        }
    };

    // Le fil du demandeur change : le plan demandé n'a plus à y figurer.
    if let Err(erreur) = cache::oublier(&state.cache, &cache::cles::fil(&compte.id)).await {
        tracing::warn!(erreur = %erreur, "fil non invalidé");
    }

    // L'auteur du plan voit la demande arriver sur son écran verrouillé ; c'est
    // le seul moment où Weave démarre une Live Activity de lui-même.
    if let Err(erreur) = live_activity::demarrer_pour(&state, &plan.author_id).await {
        tracing::warn!(erreur = %erreur, "Live Activity non démarrée");
    }
    live_activity::publier_au_mieux(&state, &compte.id).await;

    Ok(Json(json!({
        "id": demande.id,
        "sentAt": iso8601(demande.sent_at.and_utc()),
        "requestsLeftToday": restantes,
    })))
}

/// Accepter une demande : ouvre une conversation à deux.
///
/// Quand la dernière place part, le plan passe « complet » et les demandes
/// encore en attente sont closes — leurs auteurs n'ont plus à attendre une
/// réponse qui ne viendrait pas.
async fn accepter(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let demande = join_requests::Entity::find_by_id(id.clone())
        .one(&state.db)
        .await?
        .ok_or_else(|| introuvable("Demande introuvable."))?;

    let plan = plans::Entity::find_by_id(demande.plan_id.clone())
        .one(&state.db)
        .await?
        .ok_or_else(|| introuvable("Demande introuvable."))?;

    if plan.author_id != compte.id {
        return Err(AppError::new(Code::Forbidden, "Ce plan n'est pas le vôtre."));
    }
    if demande.state != "envoyee" {
        return Err(invalide("Cette demande est déjà tranchée."));
    }
    if plan.state != "ouvert" {
        return Err(AppError::new(
            Code::PlanClosed,
            "Ce plan n'accepte plus de demandes.",
        ));
    }

    let transaction = state.db.begin().await?;

    // Le verrou de la place, et il doit être pris AVANT de compter.
    //
    // Compter hors transaction ne protégeait que deux acceptations de la MÊME
    // demande — le filtre sur l'état s'en chargeait. Deux acceptations de
    // demandes DIFFÉRENTES sur la dernière place lisaient toutes deux le même
    // compte, mettaient chacune à jour sa propre ligne, et passaient toutes
    // deux : un plan pour une personne en accueillait deux.
    //
    // Écrire sur la ligne du plan prend son verrou pour la durée de la
    // transaction. La seconde acceptation attend donc la première, puis
    // réévalue `state = 'ouvert'` : si la première a rempli le plan, elle ne
    // touche aucune ligne et repart. Sinon, le compte qui suit voit la place
    // déjà prise.
    let ouvert = plans::Entity::update_many()
        .col_expr(
            plans::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(Utc::now().naive_utc()),
        )
        .filter(plans::Column::Id.eq(plan.id.as_str()))
        .filter(plans::Column::State.eq("ouvert"))
        .exec(&transaction)
        .await?;
    if ouvert.rows_affected != 1 {
        transaction.rollback().await?;
        return Err(AppError::new(
            Code::PlanClosed,
            "Toutes les places sont prises.",
        ));
    }

    let acceptees = join_requests::Entity::find()
        .filter(join_requests::Column::PlanId.eq(plan.id.as_str()))
        .filter(join_requests::Column::State.eq("acceptee"))
        .count(&transaction)
        .await?;
    if acceptees >= plan.capacity as u64 {
        transaction.rollback().await?;
        return Err(AppError::new(
            Code::PlanClosed,
            "Toutes les places sont prises.",
        ));
    }
    let restant_apres = plan.capacity as u64 - acceptees - 1;

    // Le filtre sur l'état écarte une seconde acceptation de la même demande.
    let accepte = join_requests::Entity::update_many()
        .col_expr(
            join_requests::Column::State,
            sea_orm::sea_query::Expr::value("acceptee"),
        )
        .col_expr(
            join_requests::Column::DecidedAt,
            sea_orm::sea_query::Expr::value(Utc::now().naive_utc()),
        )
        .filter(join_requests::Column::Id.eq(id.as_str()))
        .filter(join_requests::Column::State.eq("envoyee"))
        .exec(&transaction)
        .await?;
    if accepte.rows_affected != 1 {
        transaction.rollback().await?;
        return Err(invalide("Cette demande est déjà tranchée."));
    }

    if restant_apres == 0 {
        let mut complet: plans::ActiveModel = plan.clone().into();
        complet.state = Set("complet".to_string());
        complet.updated_at = Set(Utc::now().naive_utc());
        complet.update(&transaction).await?;

        join_requests::Entity::update_many()
            .col_expr(
                join_requests::Column::State,
                sea_orm::sea_query::Expr::value("expiree"),
            )
            .col_expr(
                join_requests::Column::DecidedAt,
                sea_orm::sea_query::Expr::value(Utc::now().naive_utc()),
            )
            .filter(join_requests::Column::PlanId.eq(plan.id.as_str()))
            .filter(join_requests::Column::State.eq("envoyee"))
            .exec(&transaction)
            .await?;
    }

    let conversation = conversations::ActiveModel {
        id: Set(cuid2::create_id()),
        plan_id: Set(demande.plan_id.clone()),
        request_id: Set(demande.id.clone()),
        host_id: Set(compte.id.clone()),
        guest_id: Set(demande.author_id.clone()),
        ..Default::default()
    }
    .insert(&transaction)
    .await?;

    transaction.commit().await?;

    for compte_id in [&compte.id, &demande.author_id] {
        if let Err(erreur) = cache::oublier(&state.cache, &cache::cles::fil(compte_id)).await {
            tracing::warn!(erreur = %erreur, "fil non invalidé");
        }
    }

    // Celui qui avait demandé apprend qu'il est attendu : là aussi, la bannière
    // doit pouvoir apparaître sans que l'application ait été lancée.
    if let Err(erreur) = live_activity::demarrer_pour(&state, &demande.author_id).await {
        tracing::warn!(erreur = %erreur, "Live Activity non démarrée");
    }
    live_activity::publier_au_mieux(&state, &compte.id).await;

    Ok(Json(json!({ "ok": true, "conversationId": conversation.id })))
}
