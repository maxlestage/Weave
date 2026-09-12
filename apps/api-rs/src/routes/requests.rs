//! Demander à venir.
//!
//! Le message est obligatoire, et d'une longueur minimale : c'est ce qui
//! distingue une demande d'un geste. Chaque demande consomme une unité du
//! quota journalier — à tous les paliers, sans exception. C'est l'invariant
//! qui empêche d'arroser.

use crate::{
    auth::Authentifie,
    entities::conversations,
    cache,
    droits::quota_journalier,
    entities::{accounts, blocks, join_requests, plans},
    error::{introuvable, invalide, AppError, Code},
    limitation::{consommer, regles},
    temps::{iso8601, jour_local, secondes_avant_minuit},
    AppState,
};
use axum::{
    extract::{Path, State},
    routing::post,
    Json, Router,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, Set,
    TransactionTrait,
};
use serde::Deserialize;
use serde_json::{json, Value};

/// Vingt caractères : assez pour dire pourquoi on veut venir, trop pour un
/// « salut » envoyé à la chaîne.
const MESSAGE_MIN: usize = 20;
const MESSAGE_MAX: usize = 600;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/requests", post(demander))
        .route("/v1/requests/{id}/accept", post(accepter))
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

    let acceptees = join_requests::Entity::find()
        .filter(join_requests::Column::PlanId.eq(plan.id.as_str()))
        .filter(join_requests::Column::State.eq("acceptee"))
        .count(&state.db)
        .await?;
    if acceptees >= plan.capacity as u64 {
        return Err(AppError::new(
            Code::PlanClosed,
            "Toutes les places sont prises.",
        ));
    }
    let restant_apres = plan.capacity as u64 - acceptees - 1;

    let transaction = state.db.begin().await?;

    // Le filtre sur l'état porte l'invariant de capacité : de deux
    // acceptations concurrentes, une seule voit une ligne « envoyee » et passe.
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

    Ok(Json(json!({ "ok": true, "conversationId": conversation.id })))
}
