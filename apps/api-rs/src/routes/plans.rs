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
    crypto::signer_url_media,
    droits::{droits_pour, exiger_credit},
    live_activity,
    entities::{accounts, join_requests, plans, profiles},
    error::{invalide, introuvable, AppError, Code},
    limitation::{consommer, regles},
    temps::{age_depuis, iso8601},
    AppState,
};
use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};
use serde::Deserialize;
use serde_json::{json, Value};

/// Au plus trois plans ouverts à la fois. C'est l'invariant central : il ne
/// s'achète pas, et aucun palier ne le desserre.
pub const MAX_PLANS_OUVERTS: u64 = 3;

/// Un plan se publie au moins une heure à l'avance : le temps que quelqu'un le
/// voie et demande à venir.
pub(crate) const DELAI_MINIMUM_MINUTES: i64 = 60;

pub(crate) const CAPACITE_SOLO: i32 = 1;
pub(crate) const CAPACITE_GROUPE_MAX: i32 = 4;
pub(crate) const TITRE_MIN: usize = 8;
pub(crate) const TITRE_MAX: usize = 80;
pub(crate) const NOTE_MAX: usize = 280;

pub(crate) const CATEGORIES: [&str; 8] = [
    "sortie", "sport", "culture", "repas", "musique", "jeux", "balade", "benevolat",
];

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/plans", post(publier))
        // « mine » avant « {id} » : axum choisit la route littérale, mais
        // l'ordre rend l'intention lisible.
        .route("/v1/plans/mine", get(les_miens))
        .route("/v1/plans/{id}", delete(annuler))
        .route("/v1/plans/{id}/requests", get(demandes_recues))
}

/// Le plafond de plans rendus. Personne n'en a cinquante ouverts : c'est une
/// borne contre une requête qui dériverait, pas une pagination.
const PLANS_RENDUS_MAX: u64 = 50;

/// Mes plans — ceux que j'ai publiés, à venir comme passés.
async fn les_miens(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
) -> Result<Json<Value>, AppError> {
    let lignes = plans::Entity::find()
        .filter(plans::Column::AuthorId.eq(compte.id.as_str()))
        .order_by_asc(plans::Column::StartsAt)
        .limit(PLANS_RENDUS_MAX)
        .all(&state.db)
        .await?;

    let maintenant = Utc::now().naive_utc();
    let mut rendus = Vec::with_capacity(lignes.len());

    for ligne in lignes {
        let acceptees = join_requests::Entity::find()
            .filter(join_requests::Column::PlanId.eq(ligne.id.as_str()))
            .filter(join_requests::Column::State.eq("acceptee"))
            .count(&state.db)
            .await? as i32;
        let en_attente = join_requests::Entity::find()
            .filter(join_requests::Column::PlanId.eq(ligne.id.as_str()))
            .filter(join_requests::Column::State.eq("envoyee"))
            .count(&state.db)
            .await?;

        rendus.push(json!({
            "id": ligne.id,
            "title": ligne.title,
            "note": ligne.note,
            "category": ligne.category,
            "startsAt": iso8601(ligne.starts_at.and_utc()),
            "city": ligne.city,
            "capacity": ligne.capacity,
            "seatsLeft": (ligne.capacity - acceptees).max(0),
            // Un plan dont l'heure est passée est « passé », quel que soit son
            // état en base : c'est ce que voit l'auteur, et rien ne le réécrit.
            "state": if ligne.starts_at < maintenant { "passe" } else { ligne.state.as_str() },
            "pendingRequests": en_attente,
        }));
    }

    Ok(Json(Value::Array(rendus)))
}

/// Les demandes reçues sur un de mes plans.
///
/// Seul l'auteur les voit : elles ne sont pas publiques, et le message qu'on
/// écrit pour rejoindre un plan n'est lu que par qui l'organise.
async fn demandes_recues(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Path(plan_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let plan = plans::Entity::find_by_id(plan_id.as_str())
        .one(&state.db)
        .await?
        .ok_or_else(|| introuvable("Plan introuvable."))?;

    if plan.author_id != compte.id {
        return Err(AppError::new(Code::Forbidden, "Ce plan n'est pas le vôtre."));
    }

    let demandes = join_requests::Entity::find()
        .filter(join_requests::Column::PlanId.eq(plan_id.as_str()))
        .filter(join_requests::Column::State.eq("envoyee"))
        .order_by_asc(join_requests::Column::SentAt)
        .all(&state.db)
        .await?;

    let mut rendues = Vec::with_capacity(demandes.len());
    for demande in demandes {
        let auteur = accounts::Entity::find_by_id(demande.author_id.as_str())
            .one(&state.db)
            .await?;
        // Un compte supprimé entre-temps ne doit pas faire échouer la lecture
        // de toute la liste : sa demande n'a simplement plus d'auteur à
        // montrer.
        let Some(auteur) = auteur else { continue };

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

        rendues.push(json!({
            "id": demande.id,
            "message": demande.message,
            "sentAt": iso8601(demande.sent_at.and_utc()),
            "author": {
                "id": auteur.id,
                "displayName": auteur.display_name,
                "age": age_depuis(auteur.birth_date.and_utc(), Utc::now()),
                "photoUrl": photo,
                "verified": auteur.verified,
            },
        }));
    }

    Ok(Json(Value::Array(rendues)))
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

    // Un premier comptage, avant de dépenser quoi que ce soit.
    //
    // Le comptage qui fait foi est plus bas, sous verrou. Celui-ci ne sert
    // qu'à ne pas prélever un crédit « Horizon » ou « Tablée » à quelqu'un
    // qu'on va refuser trois lignes plus loin : le crédit se consomme hors
    // transaction, la transaction ne le rendrait donc pas.
    if compter_ouverts(&state.db, &compte.id).await? >= MAX_PLANS_OUVERTS {
        return Err(trop_de_plans());
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

    let transaction = state.db.begin().await?;

    // Le verrou du compte, pris avant de compter.
    //
    // Compter puis insérer hors transaction laissait deux publications
    // simultanées lire le même total et passer toutes deux : à deux plans
    // ouverts, on en obtenait quatre. Écrire sur la ligne du compte sérialise
    // les publications d'une même personne — la seconde attend la première,
    // puis compte ce que celle-ci a inséré.
    accounts::Entity::update_many()
        .col_expr(
            accounts::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(Utc::now().naive_utc()),
        )
        .filter(accounts::Column::Id.eq(compte.id.as_str()))
        .exec(&transaction)
        .await?;

    // Le comptage qui fait foi, sous verrou.
    if compter_ouverts(&transaction, &compte.id).await? >= MAX_PLANS_OUVERTS {
        transaction.rollback().await?;
        return Err(trop_de_plans());
    }

    let profil = profiles::Entity::find()
        .filter(profiles::Column::AccountId.eq(compte.id.as_str()))
        .one(&transaction)
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
    .insert(&transaction)
    .await?;

    transaction.commit().await?;

    // Le prochain plan a peut-être changé : la bannière de l'écran verrouillé
    // doit le dire tout de suite.
    live_activity::publier_au_mieux(&state, &compte.id).await;

    Ok(Json(json!({
        "id": plan.id,
        "title": plan.title,
        "startsAt": iso8601(plan.starts_at.and_utc()),
    })))
}

/// Compte les plans ouverts à venir d'une personne.
///
/// Un plan passé n'occupe plus de place : seuls les plans dont l'heure de
/// rendez-vous est encore devant nous sont comptés.
///
/// Générique sur le connecteur pour qu'on puisse compter aussi bien sur la
/// base que dans une transaction — c'est exactement la différence entre le
/// comptage indicatif et celui qui fait foi.
async fn compter_ouverts<C: sea_orm::ConnectionTrait>(
    connexion: &C,
    compte_id: &str,
) -> Result<u64, sea_orm::DbErr> {
    plans::Entity::find()
        .filter(plans::Column::AuthorId.eq(compte_id))
        .filter(plans::Column::State.eq("ouvert"))
        .filter(plans::Column::StartsAt.gt(Utc::now().naive_utc()))
        .count(connexion)
        .await
}

/// Le refus, écrit une fois : les deux comptages doivent dire la même chose.
fn trop_de_plans() -> AppError {
    AppError::new(
        Code::TooManyPlans,
        format!(
            "Vous avez déjà {MAX_PLANS_OUVERTS} plans ouverts. Annulez-en un pour en publier un autre."
        ),
    )
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

    live_activity::publier_au_mieux(&state, &compte.id).await;

    Ok(Json(json!({ "ok": true })))
}
