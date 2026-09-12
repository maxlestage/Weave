//! Les plans : ce que l'on compte faire, et quand.
//!
//! Weave ne publie pas des profils mais des plans. Deux règles gouvernent ce
//! module, et aucune n'est négociable contre de l'argent :
//!
//!   - au plus trois plans ouverts à la fois, quel que soit le palier ;
//!   - aucun classement payant — ni palier ni achat ne fait remonter un plan
//!     devant celui de quelqu'un d'autre.
//!
//! Ce qui se vend, c'est l'horizon de publication et les plans de groupe.

use crate::{
    auth::Authentifie,
    droits::droits_pour,
    entities::{join_requests, plans, profiles},
    error::{invalide, introuvable, AppError, Code},
    limitation::{consommer, regles},
    temps::iso8601,
    AppState,
};
use axum::{
    extract::{Path, State},
    routing::{delete, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set,
    TransactionTrait,
};
use serde::Deserialize;
use serde_json::{json, Value};

/// Au plus trois plans ouverts à la fois. C'est l'invariant central : il ne
/// s'achète pas, et aucun palier ne le desserre.
pub const MAX_PLANS_OUVERTS: u64 = 3;

/// Un plan se publie au moins une heure à l'avance : le temps que quelqu'un le
/// voie et demande à venir.
const DELAI_MINIMUM_MINUTES: i64 = 60;

const CAPACITE_SOLO: i32 = 1;
const CAPACITE_GROUPE_MAX: i32 = 4;
const TITRE_MIN: usize = 8;
const TITRE_MAX: usize = 80;
const NOTE_MAX: usize = 280;

const CATEGORIES: [&str; 8] = [
    "sortie", "sport", "culture", "repas", "musique", "jeux", "balade", "benevolat",
];

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/plans", post(publier))
        .route("/v1/plans/{id}", delete(annuler))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NouveauPlan {
    title: String,
    note: Option<String>,
    category: String,
    /// Date et heure du rendez-vous, ISO 8601.
    starts_at: String,
    city: Option<String>,
    capacity: Option<i32>,
}

async fn publier(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Json(corps): Json<NouveauPlan>,
) -> Result<Json<Value>, AppError> {
    consommer(&state, regles::PUBLICATION, &compte.id).await?;
    let droits = droits_pour(&compte.tier);

    let titre = corps.title.trim().to_string();
    if titre.chars().count() < TITRE_MIN || titre.chars().count() > TITRE_MAX {
        return Err(invalide(&format!(
            "Le titre doit faire entre {TITRE_MIN} et {TITRE_MAX} caractères."
        )));
    }
    let note = corps.note.unwrap_or_default().trim().to_string();
    if note.chars().count() > NOTE_MAX {
        return Err(invalide(&format!(
            "La note ne peut pas dépasser {NOTE_MAX} caractères."
        )));
    }
    if !CATEGORIES.contains(&corps.category.as_str()) {
        return Err(invalide("Catégorie inconnue."));
    }

    let debut = DateTime::parse_from_rfc3339(&corps.starts_at)
        .map_err(|_| invalide("Date de rendez-vous illisible."))?
        .with_timezone(&Utc);

    if debut < Utc::now() + Duration::minutes(DELAI_MINIMUM_MINUTES) {
        return Err(invalide(&format!(
            "Un plan se publie au moins {DELAI_MINIMUM_MINUTES} minutes à l'avance."
        )));
    }

    // Le palier borne l'horizon ; un crédit « Horizon » l'ouvre une fois.
    if debut > Utc::now() + Duration::days(droits.jours_a_l_avance) {
        exiger_credit(&state, &compte.id, "horizon", "Horizon").await?;
    }

    let capacite = corps.capacity.unwrap_or(CAPACITE_SOLO);
    if !(CAPACITE_SOLO..=CAPACITE_GROUPE_MAX).contains(&capacite) {
        return Err(invalide(&format!(
            "La capacité doit être comprise entre {CAPACITE_SOLO} et {CAPACITE_GROUPE_MAX}."
        )));
    }
    if capacite > CAPACITE_SOLO && !droits.plans_de_groupe {
        exiger_credit(&state, &compte.id, "tablee", "Tablée").await?;
    }

    // L'invariant : trois plans ouverts, pas un de plus. On ne compte que les
    // plans à venir — un plan passé n'occupe plus de place.
    let ouverts = plans::Entity::find()
        .filter(plans::Column::AuthorId.eq(compte.id.as_str()))
        .filter(plans::Column::State.eq("ouvert"))
        .filter(plans::Column::StartsAt.gt(Utc::now().naive_utc()))
        .count(&state.db)
        .await?;
    if ouverts >= MAX_PLANS_OUVERTS {
        return Err(AppError::new(
            Code::TooManyPlans,
            format!(
                "Vous avez déjà {MAX_PLANS_OUVERTS} plans ouverts. Annulez-en un pour en publier un autre."
            ),
        ));
    }

    let profil = profiles::Entity::find()
        .filter(profiles::Column::AccountId.eq(compte.id.as_str()))
        .one(&state.db)
        .await?
        .ok_or_else(|| invalide("Renseignez d'abord votre ville."))?;

    let plan = plans::ActiveModel {
        id: Set(cuid2::create_id()),
        author_id: Set(compte.id.clone()),
        title: Set(titre),
        note: Set(note),
        category: Set(corps.category),
        starts_at: Set(debut.naive_utc()),
        city: Set(corps.city.unwrap_or(profil.city)),
        // La position est arrondie au kilomètre : Weave n'expose jamais de
        // position précise, pas même celle de l'auteur du plan.
        lat_rounded: Set(profil.lat_rounded),
        lon_rounded: Set(profil.lon_rounded),
        capacity: Set(capacite),
        state: Set("ouvert".to_string()),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&state.db)
    .await?;

    Ok(Json(json!({
        "id": plan.id,
        "title": plan.title,
        "startsAt": iso8601(plan.starts_at.and_utc()),
    })))
}

async fn annuler(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let plan = plans::Entity::find_by_id(id.clone())
        .one(&state.db)
        .await?
        .ok_or_else(|| introuvable("Plan introuvable."))?;

    if plan.author_id != compte.id {
        return Err(AppError::new(
            Code::Forbidden,
            "Ce plan n'est pas le vôtre.",
        ));
    }

    // Les deux écritures vont ensemble : un plan annulé dont les demandes
    // resteraient « envoyée » laisserait leurs auteurs attendre une réponse
    // qui ne viendra jamais.
    let transaction = state.db.begin().await?;

    let mut annule: plans::ActiveModel = plan.into();
    annule.state = Set("annule".to_string());
    annule.cancelled_at = Set(Some(Utc::now().naive_utc()));
    annule.updated_at = Set(Utc::now().naive_utc());
    annule.update(&transaction).await?;

    let en_attente = join_requests::Entity::find()
        .filter(join_requests::Column::PlanId.eq(id.as_str()))
        .filter(join_requests::Column::State.eq("envoyee"))
        .all(&transaction)
        .await?;

    for demande in en_attente {
        let mut close: join_requests::ActiveModel = demande.into();
        close.state = Set("expiree".to_string());
        close.decided_at = Set(Some(Utc::now().naive_utc()));
        close.update(&transaction).await?;
    }

    transaction.commit().await?;

    Ok(Json(json!({ "ok": true })))
}

/// Consomme un crédit, ou explique comment l'obtenir — par un palier ou à
/// l'unité. Le client n'a rien à deviner ni à coder en dur.
async fn exiger_credit(
    state: &AppState,
    compte_id: &str,
    sku: &str,
    nom: &str,
) -> Result<(), AppError> {
    use crate::entities::credit_balances;
    use sea_orm::sea_query::ExprTrait;

    // Décrément conditionnel : la clause `balance >= 1` est dans la requête,
    // sans quoi deux achats simultanés pourraient dépenser le même crédit.
    let resultat = credit_balances::Entity::update_many()
        .col_expr(
            credit_balances::Column::Balance,
            sea_orm::sea_query::Expr::col(credit_balances::Column::Balance).sub(1),
        )
        .filter(credit_balances::Column::AccountId.eq(compte_id))
        .filter(credit_balances::Column::Sku.eq(sku))
        .filter(credit_balances::Column::Balance.gte(1))
        .exec(&state.db)
        .await?;

    if resultat.rows_affected == 1 {
        return Ok(());
    }

    Err(AppError::new(
        Code::EntitlementRequired,
        format!("« {nom} » n'est pas compris dans votre offre."),
    )
    .avec_details(json!({ "sku": sku })))
}
