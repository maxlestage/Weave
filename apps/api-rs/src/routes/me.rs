//! Son propre compte : sa fiche, ses critères, sa photo.
//!
//! La fiche est volontairement maigre — une ville, un genre, une phrase. Dans
//! Weave, ce n'est pas la fiche qui donne envie, c'est le plan. On ne remplit
//! pas un formulaire pour se rendre désirable ; on écrit ce qu'on compte faire.

use crate::{
    auth::{oublier_compte, Authentifie},
    cache,
    crypto::signer_url_media,
    droits::{credits_pour, demandes_restantes, quota_journalier},
    entities::{accounts, preferences, profiles},
    error::{introuvable, invalide, AppError},
    temps::{age_depuis, iso8601},
    AppState,
};
use axum::{
    extract::State,
    routing::{get, patch, put},
    Json, Router,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Une phrase, pas une biographie.
const BIO_MAX_CARACTERES: usize = 160;
/// Weave est réservé aux majeurs ; au-delà de 99 ans, le critère n'a plus de
/// sens comme filtre.
const AGE_MINIMUM: i32 = 18;
const AGE_MAXIMUM: i32 = 99;
/// Le rayon du fil, en kilomètres. `packages/contracts` fait foi.
const RAYON_MAXIMUM_KM: i32 = 100;
/// Les catégories de plans, telles que `packages/contracts` les fixe.
const CATEGORIES: [&str; 8] = [
    "sortie", "sport", "culture", "repas", "musique", "jeux", "balade", "benevolat",
];

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/me", get(lire).patch(modifier))
        .route("/v1/me/profile", put(deposer_fiche))
        .route("/v1/me/preferences", get(lire_criteres).patch(ajuster_criteres))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModificationCompte {
    display_name: Option<String>,
    timezone: Option<String>,
    locale: Option<String>,
}

/// Modifier son compte : le nom affiché, le fuseau, la langue.
async fn modifier(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Json(corps): Json<ModificationCompte>,
) -> Result<Json<Value>, AppError> {
    if let Some(nom) = &corps.display_name {
        let taille = nom.chars().count();
        if taille == 0 || taille > 40 {
            return Err(invalide("Le nom affiché tient en 40 caractères."));
        }
    }
    if corps.timezone.as_ref().is_some_and(|f| f.chars().count() > 64) {
        return Err(invalide("Fuseau horaire invalide."));
    }
    if corps.locale.as_ref().is_some_and(|l| l.chars().count() > 10) {
        return Err(invalide("Langue invalide."));
    }

    let ligne = accounts::Entity::find_by_id(compte.id.as_str())
        .one(&state.db)
        .await?
        .ok_or_else(|| introuvable("Compte introuvable."))?;

    let mut modifie: accounts::ActiveModel = ligne.into();
    if let Some(nom) = corps.display_name {
        modifie.display_name = Set(nom);
    }
    if let Some(fuseau) = corps.timezone {
        modifie.timezone = Set(fuseau);
    }
    if let Some(langue) = corps.locale {
        modifie.locale = Set(langue);
    }
    modifie.updated_at = Set(Utc::now().naive_utc());
    modifie.update(&state.db).await?;

    oublier_compte(&state, &compte.id).await;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fiche {
    city: String,
    latitude: f64,
    longitude: f64,
    gender: String,
    bio: Option<String>,
}

/// Déposer sa fiche : une ville, un genre, une phrase.
async fn deposer_fiche(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Json(corps): Json<Fiche>,
) -> Result<Json<Value>, AppError> {
    let ville = corps.city.trim().to_string();
    if ville.is_empty() || ville.chars().count() > 80 {
        return Err(invalide("Indiquez une ville."));
    }
    if corps.gender.chars().count() > 40 {
        return Err(invalide("Genre invalide."));
    }
    if !(-90.0..=90.0).contains(&corps.latitude) || !(-180.0..=180.0).contains(&corps.longitude) {
        return Err(invalide("Coordonnées invalides."));
    }
    if let Some(bio) = &corps.bio {
        if bio.chars().count() > BIO_MAX_CARACTERES {
            return Err(invalide(&format!(
                "La phrase tient en {BIO_MAX_CARACTERES} caractères."
            )));
        }
    }

    // Les coordonnées sont arrondies au dépôt : Weave ne conserve jamais une
    // position plus précise que le kilomètre. L'arrondi est fait ici, pas à la
    // lecture — ce qui n'est pas enregistré ne peut pas fuir.
    let lat = (corps.latitude * 100.0).round() / 100.0;
    let lon = (corps.longitude * 100.0).round() / 100.0;

    let existante = profiles::Entity::find()
        .filter(profiles::Column::AccountId.eq(compte.id.as_str()))
        .one(&state.db)
        .await?;

    match existante {
        Some(ligne) => {
            let mut fiche: profiles::ActiveModel = ligne.into();
            fiche.city = Set(ville);
            fiche.lat_rounded = Set(lat);
            fiche.lon_rounded = Set(lon);
            fiche.gender = Set(corps.gender);
            if let Some(bio) = corps.bio {
                fiche.bio = Set(bio);
            }
            fiche.updated_at = Set(Utc::now().naive_utc());
            fiche.update(&state.db).await?;
        }
        None => {
            profiles::ActiveModel {
                id: Set(cuid2::create_id()),
                account_id: Set(compte.id.clone()),
                city: Set(ville),
                lat_rounded: Set(lat),
                lon_rounded: Set(lon),
                gender: Set(corps.gender),
                bio: Set(corps.bio.unwrap_or_default()),
                photo_key: Set(None),
                photo_reviewed_at: Set(None),
                created_at: Set(Utc::now().naive_utc()),
                updated_at: Set(Utc::now().naive_utc()),
            }
            .insert(&state.db)
            .await?;
        }
    }

    // Une ville et un genre suffisent pour publier et pour demander : c'est
    // tout ce qui manque avant d'être actif.
    if compte.status == "onboarding" {
        let ligne = accounts::Entity::find_by_id(compte.id.as_str())
            .one(&state.db)
            .await?
            .ok_or_else(|| introuvable("Compte introuvable."))?;
        if ligne.status == "onboarding" {
            let mut actif: accounts::ActiveModel = ligne.into();
            actif.status = Set("active".to_string());
            actif.updated_at = Set(Utc::now().naive_utc());
            actif.update(&state.db).await?;
        }
    }

    oublier_compte(&state, &compte.id).await;
    oublier_fil(&state, &compte.id).await;
    Ok(Json(json!({ "ok": true })))
}

/// Lire les critères du fil.
async fn lire_criteres(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
) -> Result<Json<Value>, AppError> {
    let pref = preferences::Entity::find()
        .filter(preferences::Column::AccountId.eq(compte.id.as_str()))
        .one(&state.db)
        .await?
        .ok_or_else(|| introuvable("Critères introuvables."))?;

    Ok(Json(json!({
        "minAge": pref.min_age,
        "maxAge": pref.max_age,
        "maxDistanceKm": pref.max_distance_km,
        "seeking": liste_json(&pref.seeking_json),
        "categories": liste_json(&pref.categories_json),
        "escaleCity": pref.escale_city,
        "escaleUntil": pref.escale_until.map(|d| iso8601(d.and_utc())),
    })))
}

/// Les listes sont stockées en JSON dans une colonne texte — le schéma est
/// partagé avec SQLite, qui n'a pas de type tableau. Une colonne illisible ne
/// doit pas faire échouer la lecture des critères : elle vaut « aucun ».
fn liste_json(brut: &str) -> Vec<String> {
    serde_json::from_str(brut).unwrap_or_default()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AjustementCriteres {
    min_age: Option<i32>,
    max_age: Option<i32>,
    max_distance_km: Option<i32>,
    seeking: Option<Vec<String>>,
    categories: Option<Vec<String>>,
}

/// Ajuster les critères du fil.
async fn ajuster_criteres(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Json(corps): Json<AjustementCriteres>,
) -> Result<Json<Value>, AppError> {
    for (valeur, nom) in [(corps.min_age, "minimum"), (corps.max_age, "maximum")] {
        if let Some(age) = valeur {
            if !(AGE_MINIMUM..=AGE_MAXIMUM).contains(&age) {
                return Err(invalide(&format!(
                    "L'âge {nom} doit être compris entre {AGE_MINIMUM} et {AGE_MAXIMUM} ans."
                )));
            }
        }
    }
    if let (Some(min), Some(max)) = (corps.min_age, corps.max_age) {
        if min > max {
            return Err(invalide("L'âge minimum ne peut pas dépasser l'âge maximum."));
        }
    }
    if let Some(rayon) = corps.max_distance_km {
        if !(1..=RAYON_MAXIMUM_KM).contains(&rayon) {
            return Err(invalide(&format!(
                "Le rayon va de 1 à {RAYON_MAXIMUM_KM} kilomètres."
            )));
        }
    }
    if corps.seeking.as_ref().is_some_and(|l| l.len() > 6) {
        return Err(invalide("Six choix au plus."));
    }
    if let Some(categories) = &corps.categories {
        if categories.len() > 8 {
            return Err(invalide("Huit catégories au plus."));
        }
        if let Some(inconnue) = categories.iter().find(|c| !CATEGORIES.contains(&c.as_str())) {
            return Err(invalide(&format!("Catégorie inconnue : {inconnue}.")));
        }
    }

    let pref = preferences::Entity::find()
        .filter(preferences::Column::AccountId.eq(compte.id.as_str()))
        .one(&state.db)
        .await?
        .ok_or_else(|| introuvable("Critères introuvables."))?;

    let mut ajuste: preferences::ActiveModel = pref.into();
    if let Some(age) = corps.min_age {
        ajuste.min_age = Set(age);
    }
    if let Some(age) = corps.max_age {
        ajuste.max_age = Set(age);
    }
    if let Some(rayon) = corps.max_distance_km {
        ajuste.max_distance_km = Set(rayon);
    }
    if let Some(liste) = corps.seeking {
        ajuste.seeking_json = Set(serde_json::to_string(&liste).unwrap_or_else(|_| "[]".into()));
    }
    if let Some(liste) = corps.categories {
        ajuste.categories_json = Set(serde_json::to_string(&liste).unwrap_or_else(|_| "[]".into()));
    }
    ajuste.updated_at = Set(Utc::now().naive_utc());
    ajuste.update(&state.db).await?;

    // Les critères décident de ce que le fil retient : le cache qu'ils
    // gouvernent ne vaut plus rien.
    oublier_fil(&state, &compte.id).await;
    Ok(Json(json!({ "ok": true })))
}

async fn oublier_fil(state: &AppState, compte_id: &str) {
    if let Err(erreur) = cache::oublier(&state.cache, &cache::cles::fil(compte_id)).await {
        tracing::warn!(erreur = %erreur, "fil non invalidé");
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Me {
    pub id: String,
    pub handle: String,
    pub display_name: String,
    pub age: i32,
    pub status: String,
    pub tier: String,
    pub city: String,
    pub bio: String,
    pub photo_url: Option<String>,
    pub verified: bool,
    pub requests_left_today: i64,
    pub credits: Value,
    pub created_at: String,
}

async fn lire(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
) -> Result<Json<Me>, AppError> {
    let ligne = accounts::Entity::find_by_id(compte.id.clone())
        .one(&state.db)
        .await?
        .ok_or_else(|| introuvable("Compte introuvable."))?;

    let profil = profiles::Entity::find()
        .filter(profiles::Column::AccountId.eq(compte.id.as_str()))
        .one(&state.db)
        .await?;

    let quota = quota_journalier(&state, &compte).await;

    Ok(Json(Me {
        id: ligne.id,
        handle: ligne.handle,
        display_name: ligne.display_name,
        age: age_depuis(ligne.birth_date.and_utc(), Utc::now()),
        status: ligne.status,
        tier: compte.tier.clone(),
        city: profil.as_ref().map(|p| p.city.clone()).unwrap_or_default(),
        bio: profil.as_ref().map(|p| p.bio.clone()).unwrap_or_default(),
        // La photo n'est jamais servie par une URL devinable : chaque lecture
        // passe par une signature à durée limitée.
        photo_url: profil.as_ref().and_then(|p| p.photo_key.as_ref()).map(|cle| {
            signer_url_media(
                &state.config.media.base_url,
                &state.config.media.signing_secret,
                cle,
                state.config.media.ttl_url_signee_secondes,
                0,
            )
        }),
        verified: ligne.verified,
        requests_left_today: demandes_restantes(&state, &compte, quota).await,
        credits: serde_json::to_value(credits_pour(&state, &compte.id).await?)
            .unwrap_or_else(|_| Value::Object(Default::default())),
        // `toISOString()` de JavaScript rend « ...517Z », pas « ...517+00:00 ».
        // Le site et l'application iOS lisent ce champ tel quel.
        created_at: iso8601(ligne.created_at.and_utc()),
    }))
}
