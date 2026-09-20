//! Son propre compte : sa fiche, ses critères, sa photo.
//!
//! La fiche est volontairement maigre — une ville, un genre, une phrase. Dans
//! Weave, ce n'est pas la fiche qui donne envie, c'est le plan. On ne remplit
//! pas un formulaire pour se rendre désirable ; on écrit ce qu'on compte faire.

use super::consentements;
use crate::messages::Msg;
use crate::{
    AppState,
    auth::{Authentifie, oublier_compte},
    cache,
    crypto::signer_url_media,
    crypto::{code_otp, hacher_secret, hash_email, normaliser_email, verifier_secret},
    droits::{
        Critere, credits_pour, demandes_restantes, exiger_credit_dans, filtre_autorise,
        quota_journalier,
    },
    entities::{accounts, otp_challenges, preferences, profiles},
    error::{AppError, Code, introuvable, invalide, non_autorise},
    limitation::{consommer, regles},
    temps::{age_depuis, iso8601},
};
use axum::{
    Json, Router,
    extract::State,
    routing::{get, post, put},
};
use chrono::{Duration, Utc};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Une phrase, pas une biographie.
pub(crate) const BIO_MAX_CARACTERES: usize = 160;

/// Les genres acceptés, pour la fiche comme pour les critères.
///
/// `packages/contracts/src/invariants.ts` fait foi ; le test du contrat tient
/// l'accord. Un vocabulaire fixe est ce qui rend la correspondance possible :
/// le fil retient un plan quand le genre de son auteur figure parmi ceux que
/// le lecteur cherche, comparés caractère par caractère. La colonne acceptait
/// n'importe quelle chaîne de quarante caractères — deux orthographes d'une
/// même chose ne se seraient jamais rencontrées.
pub(crate) const GENRES: [&str; 4] = ["femme", "homme", "non_binaire", "autre"];
/// Weave est réservé aux majeurs ; au-delà de 99 ans, le critère n'a plus de
/// sens comme filtre.
const AGE_MINIMUM: i32 = 18;
const AGE_MAXIMUM: i32 = 99;
/// Le rayon du fil, en kilomètres. `packages/contracts` fait foi.
pub const RAYON_MAXIMUM_KM: i32 = 100;
/// Les catégories de plans, telles que `packages/contracts` les fixe.
const CATEGORIES: [&str; 8] = [
    "sortie",
    "sport",
    "culture",
    "repas",
    "musique",
    "jeux",
    "balade",
    "benevolat",
];

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/me", get(lire).patch(modifier))
        .route("/v1/me/profile", put(deposer_fiche))
        .route(
            "/v1/me/preferences",
            get(lire_criteres).patch(ajuster_criteres),
        )
        .route("/v1/me/escale", post(ouvrir_escale).delete(fermer_escale))
        .route("/v1/me/email", post(demander_changement_email))
        .route("/v1/me/email/verify", post(changer_email))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModificationCompte {
    display_name: Option<String>,
    timezone: Option<String>,
    locale: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NouvelleAdresse {
    email: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfirmationAdresse {
    email: String,
    code: String,
}

/// Demande un code pour changer d'adresse e-mail.
///
/// ## Ce qui manquait, et ce que cela coûtait
///
/// L'adresse EST le compte : il n'y a ni mot de passe, ni question de secours,
/// et l'on se connecte par un code envoyé dans sa boîte. Rien ne permettait
/// d'en changer.
///
/// Perdre l'accès à cette boîte — un changement d'employeur, un fournisseur
/// qui ferme, une faute de frappe à l'inscription — rendait donc le compte
/// définitivement injoignable. Avec ses conversations, son abonnement en
/// cours et ses achats. On ne pouvait même plus le supprimer, puisque
/// supprimer demande d'être connecté.
///
/// ## Le code part à la NOUVELLE adresse
///
/// C'est ce qui prouve qu'on la contrôle. L'envoyer à l'ancienne ne
/// prouverait rien sur la nouvelle, et laisserait déplacer son compte vers
/// une adresse saisie de travers — ce qui est précisément l'un des cas qu'on
/// veut réparer.
///
/// ## Cette route ne dit jamais si une adresse est déjà prise
///
/// La tentation est de refuser tout de suite « cette adresse est utilisée ».
/// Ce serait un moyen de savoir, pour n'importe quelle adresse, si elle a un
/// compte sur Weave — sur une application de rencontre, c'est une information
/// qu'on ne donne pas.
///
/// L'unicité est donc vérifiée à la CONFIRMATION, une fois le contrôle de la
/// boîte prouvé. Qui possède les deux adresses l'apprendra alors, et c'est
/// sans conséquence : elles sont à lui.
async fn demander_changement_email(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Json(corps): Json<NouvelleAdresse>,
) -> Result<Json<Value>, AppError> {
    let email = normaliser_email(&corps.email);
    if !email.contains('@') || email.len() > 320 {
        return Err(invalide(Msg::AdresseEmailInvalide));
    }
    let empreinte = hash_email(&email);

    // Deux compteurs, comme à la connexion : le compte borne les rafales, et
    // l'empreinte protège une boîte donnée. Sans le second, un compte pourrait
    // arroser de codes une adresse qui n'est pas la sienne.
    consommer(&state, regles::DEMANDE_OTP, &compte.id).await?;
    consommer(&state, regles::DEMANDE_OTP, &empreinte).await?;

    let code = code_otp();
    let code_hache = hacher_secret(&code).map_err(|erreur| {
        tracing::error!(erreur = %erreur, "hachage du code impossible");
        AppError::new(Code::Internal, Msg::ErreurInterne.t())
    })?;

    otp_challenges::ActiveModel {
        id: Set(cuid2::create_id()),
        email_hash: Set(empreinte.clone()),
        code_hash: Set(code_hache),
        attempts: Set(0),
        consumed_at: Set(None),
        expires_at: Set((Utc::now()
            + chrono::Duration::minutes(crate::routes::auth::OTP_TTL_MINUTES))
        .naive_utc()),
        created_at: Set(Utc::now().naive_utc()),
    }
    .insert(&state.db)
    .await?;

    tracing::info!(email_hash = %empreinte, "Code de changement d'adresse émis");

    Ok(Json(json!({
        "sent": true,
        "expiresInSeconds": crate::routes::auth::OTP_TTL_MINUTES * 60,
        "devCode": (!state.config.is_production()).then_some(code.clone()),
    })))
}

/// Confirme le changement d'adresse avec le code reçu.
///
/// Les sessions ouvertes ne sont PAS révoquées, et c'est un choix. Le cas
/// qu'une révocation viserait — un appareil volé dont on déplace le compte —
/// n'est pas servi par elle : elle déconnecterait la victime de ses autres
/// appareils, pas le voleur du sien. Elle coûterait donc à qui change
/// légitimement d'adresse, sans rien retirer à personne d'autre.
async fn changer_email(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Json(corps): Json<ConfirmationAdresse>,
) -> Result<Json<Value>, AppError> {
    consommer(&state, regles::VERIF_OTP, &compte.id).await?;

    let email = normaliser_email(&corps.email);
    let empreinte = hash_email(&email);

    let actuel = accounts::Entity::find_by_id(compte.id.as_str())
        .one(&state.db)
        .await?
        .ok_or_else(|| introuvable(Msg::CompteIntrouvable))?;

    // Changer pour l'adresse qu'on a déjà : rien à faire, et ce n'est pas une
    // erreur. Le dire autrement obligerait l'écran à connaître l'adresse
    // courante pour savoir s'il a le droit de demander.
    if empreinte == actuel.email_hash {
        return Ok(Json(json!({ "ok": true, "email": email })));
    }

    let defi = otp_challenges::Entity::find()
        .filter(otp_challenges::Column::EmailHash.eq(empreinte.as_str()))
        .filter(otp_challenges::Column::ConsumedAt.is_null())
        .filter(otp_challenges::Column::ExpiresAt.gt(Utc::now().naive_utc()))
        .order_by_desc(otp_challenges::Column::CreatedAt)
        .one(&state.db)
        .await?
        .ok_or_else(|| non_autorise(Msg::CodeExpire))?;

    use sea_orm::sea_query::ExprTrait;

    // Une tentative se paie d'avance, et en une seule écriture — la même
    // leçon qu'à la connexion : lire le compteur puis l'incrémenter laisse N
    // essais simultanés n'en consommer qu'un, et un code à six chiffres ne
    // résiste pas à cela.
    let tentative = otp_challenges::Entity::update_many()
        .col_expr(
            otp_challenges::Column::Attempts,
            sea_orm::sea_query::Expr::col(otp_challenges::Column::Attempts).add(1),
        )
        .filter(otp_challenges::Column::Id.eq(defi.id.as_str()))
        .filter(otp_challenges::Column::Attempts.lt(crate::routes::auth::OTP_MAX_TENTATIVES))
        .exec(&state.db)
        .await?;
    if tentative.rows_affected != 1 {
        return Err(non_autorise(Msg::TropDeTentativesSurCeCode));
    }

    if !verifier_secret(&corps.code, &defi.code_hash) {
        return Err(non_autorise(Msg::CodeIncorrect));
    }

    // L'unicité, ici seulement : le contrôle de la boîte est prouvé, et
    // l'apprendre maintenant ne renseigne que celui qui possède l'adresse.
    if accounts::Entity::find()
        .filter(accounts::Column::EmailHash.eq(empreinte.as_str()))
        .one(&state.db)
        .await?
        .is_some()
    {
        return Err(invalide(Msg::AdresseDejaUtilisee));
    }

    // Le code est consommé dans la même transaction que le changement : s'il
    // l'était avant, un échec d'écriture laisserait un code brûlé et une
    // adresse inchangée, et il faudrait tout recommencer sans savoir pourquoi.
    let transaction = state.db.begin().await?;

    let mut modifie: accounts::ActiveModel = actuel.into();
    modifie.email = Set(email.clone());
    modifie.email_hash = Set(empreinte.clone());
    modifie.updated_at = Set(Utc::now().naive_utc());
    modifie.update(&transaction).await?;

    let mut consomme: otp_challenges::ActiveModel = defi.into();
    consomme.consumed_at = Set(Some(Utc::now().naive_utc()));
    consomme.update(&transaction).await?;

    transaction.commit().await?;

    tracing::info!(compte = %compte.id, "adresse de connexion changée");
    Ok(Json(json!({ "ok": true, "email": email })))
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
            return Err(invalide(Msg::NomAfficheTropLong { maximum: 40 }));
        }
    }
    if corps
        .timezone
        .as_ref()
        .is_some_and(|f| f.chars().count() > 64)
    {
        return Err(invalide(Msg::FuseauHoraireInvalide));
    }
    if corps
        .locale
        .as_ref()
        .is_some_and(|l| l.chars().count() > 10)
    {
        return Err(invalide(Msg::LangueInvalide));
    }

    let ligne = accounts::Entity::find_by_id(compte.id.as_str())
        .one(&state.db)
        .await?
        .ok_or_else(|| introuvable(Msg::CompteIntrouvable))?;

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

/// Durée d'une « Escale », telle qu'elle est vendue.
///
/// « Publier depuis une autre ville pendant sept jours » — c'est le texte du
/// catalogue, et donc ce que la personne a payé.
const ESCALE_JOURS: i64 = 7;

#[derive(Deserialize)]
struct NouvelleEscale {
    city: String,
}

/// Ouvre une « Escale » : le fil se compose autour d'une autre ville.
///
/// ## Ce qui manquait
///
/// « Escale » se vend 3,99 € dans le catalogue, avec sa description. Le crédit
/// était bien accordé à l'achat, et le fil honorait déjà l'escale — il
/// remplace le filtre géographique par une ville dès que `escaleCity` est
/// posé. Mais **aucune ligne n'a jamais écrit `escaleCity`.** Personne ne
/// pouvait déclencher ce qu'il venait d'acheter : le crédit s'accumulait sans
/// usage.
///
/// Le mécanisme était donc complet à une route près, et c'est cette route.
async fn ouvrir_escale(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Json(corps): Json<NouvelleEscale>,
) -> Result<Json<Value>, AppError> {
    let ville = corps.city.trim().to_string();
    if ville.is_empty() || ville.chars().count() > 80 {
        return Err(invalide(Msg::IndiquezUneVille));
    }

    let fin = Utc::now() + Duration::days(ESCALE_JOURS);

    // Ouvrir et payer dans la MÊME transaction, l'écriture conditionnelle
    // d'abord.
    //
    // Une escale en cours ne se remplace pas : la seconde ferait payer un
    // crédit pour raccourcir la première, ce que personne ne veut acheter. Ce
    // contrôle se lisait puis s'écrivait en deux temps — deux appels
    // concurrents le franchissaient ensemble et DÉPENSAIENT TOUS DEUX UN
    // CRÉDIT pour une seule escale. Un double appui coûtait 5,99 € de trop.
    //
    // La condition est maintenant dans la requête : une seule des deux la
    // gagne. Et le crédit se dépense dans la même transaction — si le solde
    // est vide, tout est annulé, l'escale comprise. L'ordre inverse ouvrirait
    // une escale que personne n'a payée.
    let transaction = state.db.begin().await?;

    let ouverte = preferences::Entity::update_many()
        .col_expr(preferences::Column::EscaleCity, Expr::value(ville.clone()))
        .col_expr(
            preferences::Column::EscaleUntil,
            Expr::value(fin.naive_utc()),
        )
        .col_expr(
            preferences::Column::UpdatedAt,
            Expr::value(Utc::now().naive_utc()),
        )
        .filter(preferences::Column::AccountId.eq(compte.id.as_str()))
        .filter(
            sea_orm::Condition::any()
                .add(preferences::Column::EscaleUntil.is_null())
                .add(preferences::Column::EscaleUntil.lte(Utc::now().naive_utc())),
        )
        .exec(&transaction)
        .await?;

    if ouverte.rows_affected == 0 {
        transaction.rollback().await?;
        // Distinguer « pas de fiche » d'« escale déjà en cours » : les deux
        // rendent zéro ligne, et la seconde est la seule qui s'explique.
        let existe = preferences::Entity::find()
            .filter(preferences::Column::AccountId.eq(compte.id.as_str()))
            .one(&state.db)
            .await?
            .is_some();
        return Err(if existe {
            invalide(Msg::EscaleDejaEnCours)
        } else {
            introuvable(Msg::CriteresIntrouvables)
        });
    }

    exiger_credit_dans(&transaction, &compte.id, "escale", "Escale").await?;
    transaction.commit().await?;

    // Le fil est mis en cache : sans cet oubli, l'escale ne prendrait effet
    // qu'à l'expiration du cache, et l'achat semblerait sans effet.
    if let Err(erreur) = cache::oublier(&state.cache, &cache::cles::fil(&compte.id)).await {
        tracing::warn!(erreur = %erreur, "fil non invalidé après ouverture d'escale");
    }

    Ok(Json(json!({
        "ok": true,
        "escaleCity": ville,
        "escaleUntil": iso8601(fin),
    })))
}

/// Ferme une escale avant son terme.
///
/// Le crédit n'est pas rendu : il a été dépensé, et l'escale a servi. Fermer
/// sert à revenir chez soi plus tôt, pas à annuler un achat.
async fn fermer_escale(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
) -> Result<Json<Value>, AppError> {
    let ligne = preferences::Entity::find()
        .filter(preferences::Column::AccountId.eq(compte.id.as_str()))
        .one(&state.db)
        .await?
        .ok_or_else(|| introuvable(Msg::CriteresIntrouvables))?;

    let mut maj: preferences::ActiveModel = ligne.into();
    maj.escale_city = Set(None);
    maj.escale_until = Set(None);
    maj.updated_at = Set(Utc::now().naive_utc());
    maj.update(&state.db).await?;

    if let Err(erreur) = cache::oublier(&state.cache, &cache::cles::fil(&compte.id)).await {
        tracing::warn!(erreur = %erreur, "fil non invalidé après fermeture d'escale");
    }

    Ok(Json(json!({ "ok": true })))
}

/// Déposer sa fiche : une ville, un genre, une phrase.
async fn deposer_fiche(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Json(corps): Json<Fiche>,
) -> Result<Json<Value>, AppError> {
    let ville = corps.city.trim().to_string();
    if ville.is_empty() || ville.chars().count() > 80 {
        return Err(invalide(Msg::IndiquezUneVille));
    }
    if !GENRES.contains(&corps.gender.as_str()) {
        return Err(invalide(Msg::GenreInconnu {
            valeur: corps.gender.clone(),
            acceptees: GENRES.join(", "),
        }));
    }
    if !(-90.0..=90.0).contains(&corps.latitude) || !(-180.0..=180.0).contains(&corps.longitude) {
        return Err(invalide(Msg::CoordonneesInvalides));
    }
    if let Some(bio) = &corps.bio {
        if bio.chars().count() > BIO_MAX_CARACTERES {
            return Err(invalide(Msg::PhraseTropLongue {
                maximum: BIO_MAX_CARACTERES as i64,
            }));
        }
    }

    // Les coordonnées sont arrondies au dépôt, et non à la lecture : ce qui
    // n'est pas enregistré ne peut pas fuir.
    //
    // C'est le SECOND arrondi. L'application arrondit déjà sur l'appareil, sur
    // la même grille, parce que la politique de confidentialité le promet :
    // « une donnée que nous n'avons pas ». Celui-ci ne la contredit pas — il
    // ne déplace rien d'une valeur déjà sur la grille — et couvre ce que le
    // client ne garantit jamais : une version ancienne, un client réécrit, un
    // appel fait à la main.
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
            .ok_or_else(|| introuvable(Msg::CompteIntrouvable))?;
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
        .ok_or_else(|| introuvable(Msg::CriteresIntrouvables))?;

    Ok(Json(json!({
        "minAge": pref.min_age,
        "maxAge": pref.max_age,
        "maxDistanceKm": pref.max_distance_km,
        // Le rayon réellement appliqué au fil. Sans « critères précis », il est
        // rabattu sur un cran — et l'afficher évite de montrer « 27 km » à
        // quelqu'un dont le fil en retient 25 sans le lui dire.
        "effectiveDistanceKm": crate::droits::rayon_effectif(&compte.tier, pref.max_distance_km),
        "seeking": liste_json(&pref.seeking_json),
        "categories": liste_json(&pref.categories_json),
        "days": jours_json(&pref.days_json),
        "escaleCity": pref.escale_city,
        "escaleUntil": pref.escale_until.map(|d| iso8601(d.and_utc())),
        "remindersOn": pref.reminders_on,
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
    /// Jours de la semaine retenus, au sens ISO : 1 pour lundi, 7 pour
    /// dimanche. Vide ou absent signifie « tous ».
    days: Option<Vec<i32>>,
    /// Le rappel avant le rendez-vous.
    ///
    /// Il ne dépend d'aucun palier : oublier un rendez-vous n'est pas un
    /// confort qu'on vend, et faire payer pour être prévenu ferait du plancher
    /// gratuit un produit qui laisse quelqu'un attendre seul.
    reminders_on: Option<bool>,
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
                return Err(invalide(Msg::AgeHorsBornes {
                    nom: nom.to_string(),
                    minimum: AGE_MINIMUM.into(),
                    maximum: AGE_MAXIMUM.into(),
                }));
            }
        }
    }
    if let (Some(min), Some(max)) = (corps.min_age, corps.max_age) {
        if min > max {
            return Err(invalide(Msg::AgeMinSuperieurAuMax));
        }
    }
    if let Some(rayon) = corps.max_distance_km {
        if !(1..=RAYON_MAXIMUM_KM).contains(&rayon) {
            return Err(invalide(Msg::RayonHorsBornes {
                maximum: RAYON_MAXIMUM_KM.into(),
            }));
        }
    }
    if let Some(recherche) = &corps.seeking {
        if recherche.len() > GENRES.len() {
            return Err(invalide(Msg::ChoixTropNombreux {
                maximum: GENRES.len() as i64,
            }));
        }
        // Un genre hors vocabulaire ne correspondrait à personne : la
        // comparaison du fil est exacte. Mieux vaut le refuser que de laisser
        // quelqu'un chercher dans le vide sans jamais comprendre pourquoi son
        // fil est resté vide.
        if let Some(inconnu) = recherche.iter().find(|g| !GENRES.contains(&g.as_str())) {
            return Err(invalide(Msg::GenreInconnuSimple {
                valeur: inconnu.clone(),
            }));
        }
    }
    if let Some(categories) = &corps.categories {
        if categories.len() > 8 {
            return Err(invalide(Msg::HuitCategoriesAuPlus));
        }
        if let Some(inconnue) = categories
            .iter()
            .find(|c| !CATEGORIES.contains(&c.as_str()))
        {
            return Err(invalide(Msg::CategorieInconnueNommee {
                valeur: inconnue.clone(),
            }));
        }
    }

    if let Some(jours) = &corps.days {
        if jours.len() > 7 {
            return Err(invalide(Msg::SeptJoursAuPlus));
        }
        if let Some(hors) = jours.iter().find(|j| !(1..=7).contains(*j)) {
            return Err(invalide(Msg::JourInconnu {
                valeur: hors.to_string(),
            }));
        }
    }

    // Chaque critère vendu par palier est refusé à qui ne l'a pas.
    //
    // Le refus ne porte que sur les critères RENSEIGNÉS : vider une liste
    // reste possible pour tout le monde, sinon quelqu'un dont l'abonnement
    // expire ne pourrait plus défaire ce qu'il avait posé.
    // La liste est exhaustive, y compris pour l'âge et la distance que tous
    // les paliers autorisent : ainsi, restreindre l'un d'eux un jour tiendra
    // dans `filtre_autorise` seul, sans qu'il faille penser à le brancher ici.
    for (critere, renseigne) in [
        (
            Critere::Age,
            corps.min_age.is_some() || corps.max_age.is_some(),
        ),
        (Critere::Distance, corps.max_distance_km.is_some()),
        (
            Critere::Genre,
            corps.seeking.as_ref().is_some_and(|l| !l.is_empty()),
        ),
        (
            Critere::Categorie,
            corps.categories.as_ref().is_some_and(|l| !l.is_empty()),
        ),
        (
            Critere::Jour,
            corps.days.as_ref().is_some_and(|l| !l.is_empty()),
        ),
    ] {
        if renseigne && !filtre_autorise(&compte.tier, critere) {
            return Err(AppError::new(
                crate::error::Code::EntitlementRequired,
                format!(
                    "Filtrer par {} demande une offre supérieure.",
                    critere.nom()
                ),
            ));
        }
    }

    // Le genre recherché relève de l'article 9 : il ne s'enregistre que sur un
    // consentement explicite, distinct et en cours de validité.
    //
    // Le refus ne porte, là encore, que sur une liste RENSEIGNÉE : vider la
    // sienne reste possible sans consentement — c'est même ce que fait le
    // retrait, et il ne doit pas buter sur son propre effet.
    if corps.seeking.as_ref().is_some_and(|l| !l.is_empty())
        && !consentements::sensibles_autorisees(&state.db, &compte.id).await?
    {
        return Err(AppError::new(
            crate::error::Code::Forbidden,
            "Chercher par genre demande votre consentement aux données sensibles, \
             à donner dans Réglages › Confidentialité."
                .to_string(),
        ));
    }

    let pref = preferences::Entity::find()
        .filter(preferences::Column::AccountId.eq(compte.id.as_str()))
        .one(&state.db)
        .await?
        .ok_or_else(|| introuvable(Msg::CriteresIntrouvables))?;

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
    if let Some(liste) = corps.days {
        ajuste.days_json = Set(serde_json::to_string(&liste).unwrap_or_else(|_| "[]".into()));
    }
    if let Some(rappels) = corps.reminders_on {
        ajuste.reminders_on = Set(rappels);
    }
    ajuste.updated_at = Set(Utc::now().naive_utc());
    ajuste.update(&state.db).await?;

    // Les critères décident de ce que le fil retient : le cache qu'ils
    // gouvernent ne vaut plus rien.
    oublier_fil(&state, &compte.id).await;
    Ok(Json(json!({ "ok": true })))
}

/// Oublie le fil composé pour ce compte.
///
/// Partagée : les critères, la modération et les consentements changent tous
/// ce que le fil doit retenir, et trois copies de ces trois lignes auraient
/// fini par ne plus oublier la même clé.
pub(crate) async fn oublier_fil(state: &AppState, compte_id: &str) {
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
        .ok_or_else(|| introuvable(Msg::CompteIntrouvable))?;

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
        photo_url: profil
            .as_ref()
            .and_then(|p| p.photo_key.as_ref())
            .map(|cle| {
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

/// Relit la liste des jours, stockée en texte JSON comme les autres.
///
/// Une colonne illisible vaut « aucun » : un critère abîmé ne doit pas rendre
/// le fil inaccessible.
pub(crate) fn jours_json(brut: &str) -> Vec<i32> {
    serde_json::from_str(brut).unwrap_or_default()
}
