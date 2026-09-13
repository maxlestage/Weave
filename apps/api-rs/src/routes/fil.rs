//! Le fil : les plans à venir autour de soi.
//!
//! Une règle gouverne ce module, et elle n'est pas négociable : **aucun
//! classement payant**. Ni palier ni achat ne fait remonter un plan devant
//! celui de quelqu'un d'autre. Le tri est l'imminence d'abord, la proximité
//! ensuite, et rien de plus.
//!
//! Ce que le palier change, c'est l'horizon de publication et la finesse des
//! critères — jamais une place dans la file.

use crate::routes::me::jours_json;
use crate::{
    auth::{Authentifie, CompteAuthentifie},
    cache,
    droits::{
        demandes_restantes, droits_pour, filtre_autorise, quota_journalier, rayon_effectif, Critere,
    },
    entities::{accounts, blocks, join_requests, plans, preferences, profiles},
    error::AppError,
    limitation::{consommer, Regle},
    temps::{age_depuis, boite_englobante, distance_km, iso8601, jour_local},
    AppState,
};
use axum::{extract::State, routing::get, Json, Router};
use chrono::{Datelike, Duration, Utc};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Lecture de cache, appelée à chaque ouverture de l'application, au retour
/// d'arrière-plan et par la montre : la limite est là contre l'emballement
/// d'un client, pas contre l'usage normal.
const REGLE_FIL: Regle = Regle {
    seau: "feed",
    limite: 240,
    fenetre_secondes: 60 * 60,
};

/// Combien de plans on lit en base avant de filtrer en mémoire.
const LIMITE_LECTURE: u64 = 300;
/// Combien on en rend.
const TAILLE_FIL: usize = 60;
/// Le fil se périme vite : il dépend de la position, des critères et de l'heure.
const TTL_FIL: u64 = 5 * 60;
/// Rayon retenu quand aucun n'est réglé.
const RAYON_DEFAUT_KM: f64 = 25.0;
/// Un plan reste au fil un moment après son heure : on peut arriver en retard.
const GRACE_MINUTES: i64 = 30;

pub fn routes() -> Router<AppState> {
    Router::new().route("/v1/plans", get(lire_fil))
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PlanDuFil {
    id: String,
    title: String,
    note: String,
    category: String,
    starts_at: String,
    city: String,
    distance_km: f64,
    places_left: i32,
    author_name: String,
    author_age: i32,
    author_verified: bool,
    already_requested: bool,
}

async fn lire_fil(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
) -> Result<Json<Value>, AppError> {
    consommer(&state, REGLE_FIL, &compte.id).await?;

    let quota = quota_journalier(&state, &compte).await;
    let restantes = demandes_restantes(&state, &compte, quota).await;

    // Le fil en cache est rendu tel quel, mais jamais le quota : celui-ci se
    // dépense entre deux compositions, et l'afficher périmé donnerait un
    // compteur faux.
    let cle = cache::cles::fil(&compte.id);
    if let Some(en_cache) = cache::lire_json::<Value>(&state.cache, &cle).await {
        return Ok(Json(json!({
            "plans": en_cache.get("plans").cloned().unwrap_or_else(|| json!([])),
            "generatedAt": en_cache.get("generatedAt").cloned().unwrap_or(Value::Null),
            "requestsLeftToday": restantes,
            "fromCache": true,
        })));
    }

    let Some(contexte) = contexte_de(&state, &compte.id, &compte.tier).await? else {
        // Sans ville ni critères, il n'y a pas de fil à composer — mais le
        // service répond quand même : un compte en cours d'inscription n'est
        // pas une erreur.
        return Ok(Json(json!({
            "plans": [],
            "requestsLeftToday": restantes,
            "fromCache": false,
            "generatedAt": iso8601(Utc::now()),
        })));
    };

    let plans = composer(&state, &compte, &contexte).await?;
    let genere = iso8601(Utc::now());

    if let Err(erreur) = cache::ecrire_json(
        &state.cache,
        &cle,
        &json!({ "plans": plans, "generatedAt": genere }),
        TTL_FIL,
    )
    .await
    {
        tracing::warn!(erreur = %erreur, "fil non mis en cache");
    }

    Ok(Json(json!({
        "plans": plans,
        "requestsLeftToday": restantes,
        "fromCache": false,
        "generatedAt": genere,
    })))
}

struct Contexte {
    lat: f64,
    lon: f64,
    distance_max_km: f64,
    age_min: i32,
    age_max: i32,
    recherche: Vec<String>,
    /// Jours retenus, au sens ISO : 1 lundi, 7 dimanche. Vide = tous.
    jours: Vec<i32>,
    categories: Vec<String>,
    /// Ville d'« Escale » : le fil bascule sur une autre ville, sans distance.
    escale: Option<String>,
    /// Soi-même, et les blocages dans les deux sens.
    exclus: Vec<String>,
}

async fn contexte_de(
    state: &AppState,
    compte_id: &str,
    palier: &str,
) -> Result<Option<Contexte>, AppError> {
    let Some(profil) = profiles::Entity::find()
        .filter(profiles::Column::AccountId.eq(compte_id))
        .one(&state.db)
        .await?
    else {
        return Ok(None);
    };
    let Some(pref) = preferences::Entity::find()
        .filter(preferences::Column::AccountId.eq(compte_id))
        .one(&state.db)
        .await?
    else {
        return Ok(None);
    };

    let escale_active = pref
        .escale_until
        .map(|jusqua| jusqua.and_utc() > Utc::now())
        .unwrap_or(false);

    // Un blocage masque dans les deux sens : ni l'un ni l'autre ne doit revoir
    // les plans de celui qu'il a bloqué, ou qui l'a bloqué.
    let mut exclus = vec![compte_id.to_string()];
    for pose in blocks::Entity::find()
        .filter(blocks::Column::AuthorId.eq(compte_id))
        .all(&state.db)
        .await?
    {
        exclus.push(pose.target_id);
    }
    for subi in blocks::Entity::find()
        .filter(blocks::Column::TargetId.eq(compte_id))
        .all(&state.db)
        .await?
    {
        exclus.push(subi.author_id);
    }

    // Le genre recherché relève de l'article 9 : le fil ne filtre dessus que
    // tant que le consentement vaut.
    //
    // Le retrait efface déjà le critère, mais il n'est pas le seul chemin :
    // un changement substantiel de la politique périme les consentements
    // donnés sur la version précédente, et aucune écriture ne repasse alors
    // sur les lignes existantes. Relire ici est ce qui fait que le traitement
    // s'arrête vraiment, plutôt qu'à la prochaine fois que quelqu'un touche à
    // ses critères.
    let sensibles = super::consentements::sensibles_autorisees(&state.db, compte_id).await?;

    Ok(Some(Contexte {
        lat: profil.lat_rounded,
        lon: profil.lon_rounded,
        // La distance est rabattue sur un cran quand le palier n'achète pas la
        // distance fine — relue ici comme les autres critères vendus, et non
        // seulement contrôlée à l'écriture : un abonnement qui expire doit
        // cesser de donner ce qu'on ne paie plus.
        distance_max_km: if pref.max_distance_km > 0 {
            f64::from(rayon_effectif(palier, pref.max_distance_km))
        } else {
            RAYON_DEFAUT_KM
        },
        age_min: pref.min_age,
        age_max: pref.max_age,
        // Les critères vendus par palier sont relus à travers la règle, pas
        // seulement contrôlés à l'écriture.
        //
        // Sans cela, un abonnement qui expire laisserait en place les critères
        // posés du temps où il courait : on continuerait de bénéficier de ce
        // qu'on ne paie plus, et il aurait suffi de s'abonner un mois.
        recherche: if sensibles {
            si_autorise(palier, Critere::Genre, liste_json(&pref.seeking_json))
        } else {
            Vec::new()
        },
        jours: si_autorise(palier, Critere::Jour, jours_json(&pref.days_json)),
        categories: si_autorise(palier, Critere::Categorie, liste_json(&pref.categories_json)),
        escale: escale_active.then_some(pref.escale_city).flatten(),
        exclus,
    }))
}

/// Les listes de critères sont stockées en JSON : une valeur illisible vaut
/// « aucun critère », ce qui élargit le fil au lieu de le vider.
fn liste_json(brut: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(brut).unwrap_or_default()
}

async fn composer(
    state: &AppState,
    compte: &CompteAuthentifie,
    contexte: &Contexte,
) -> Result<Vec<PlanDuFil>, AppError> {
    let droits = droits_pour(&compte.tier);
    let maintenant = Utc::now();
    let plancher = maintenant - Duration::minutes(GRACE_MINUTES);
    let plafond = maintenant + Duration::days(droits.jours_a_l_avance);

    let mut requete = plans::Entity::find()
        .filter(plans::Column::State.eq("ouvert"))
        .filter(plans::Column::StartsAt.gte(plancher.naive_utc()))
        .filter(plans::Column::StartsAt.lte(plafond.naive_utc()))
        .filter(plans::Column::AuthorId.is_not_in(contexte.exclus.clone()));

    if !contexte.categories.is_empty() {
        requete = requete.filter(plans::Column::Category.is_in(contexte.categories.clone()));
    }

    // Une « Escale » remplace le filtre géographique par une ville : on ne
    // cherche plus autour de soi, mais là où l'on va.
    if let Some(ville) = &contexte.escale {
        requete = requete.filter(plans::Column::City.eq(ville.as_str()));
    } else {
        // Boîte englobante en SQL, distance exacte ensuite en mémoire : le
        // schéma doit rester identique sur PostgreSQL et SQLite, donc sans
        // extension géospatiale.
        let boite = boite_englobante(contexte.lat, contexte.lon, contexte.distance_max_km);
        requete = requete
            .filter(plans::Column::LatRounded.gte(boite.lat_min))
            .filter(plans::Column::LatRounded.lte(boite.lat_max))
            .filter(plans::Column::LonRounded.gte(boite.lon_min))
            .filter(plans::Column::LonRounded.lte(boite.lon_max));
    }

    let lignes = requete
        .order_by_asc(plans::Column::StartsAt)
        .limit(LIMITE_LECTURE)
        .all(&state.db)
        .await?;

    let mut retenus: Vec<(String, f64, PlanDuFil)> = Vec::new();

    for ligne in lignes {
        let Some(auteur) = accounts::Entity::find_by_id(ligne.author_id.clone())
            .one(&state.db)
            .await?
        else {
            continue;
        };

        if auteur.status != "active" || auteur.deletion_requested_at.is_some() {
            continue;
        }

        let age = age_depuis(auteur.birth_date.and_utc(), maintenant);
        if age < contexte.age_min || age > contexte.age_max {
            continue;
        }

        if !contexte.recherche.is_empty() {
            let genre = profiles::Entity::find()
                .filter(profiles::Column::AccountId.eq(auteur.id.as_str()))
                .one(&state.db)
                .await?
                .map(|p| p.gender);
            match genre {
                Some(g) if contexte.recherche.contains(&g) => {}
                _ => continue,
            }
        }

        // Le jour de la semaine se juge en mémoire : `strftime` de SQLite et
        // `EXTRACT` de PostgreSQL ne comptent pas les jours pareil, et le
        // schéma doit rester le même des deux côtés.
        if !contexte.jours.is_empty() {
            let jour = ligne.starts_at.and_utc().weekday().number_from_monday() as i32;
            if !contexte.jours.contains(&jour) {
                continue;
            }
        }

        let distance = if contexte.escale.is_some() {
            0.0
        } else {
            distance_km(contexte.lat, contexte.lon, ligne.lat_rounded, ligne.lon_rounded)
        };
        if contexte.escale.is_none() && distance > contexte.distance_max_km {
            continue;
        }

        let demandes = join_requests::Entity::find()
            .filter(join_requests::Column::PlanId.eq(ligne.id.as_str()))
            .filter(join_requests::Column::State.is_in(vec!["envoyee", "acceptee"]))
            .all(&state.db)
            .await?;

        let acceptees = demandes.iter().filter(|d| d.state == "acceptee").count() as i32;
        let places = (ligne.capacity - acceptees).max(0);
        let deja_demande = demandes.iter().any(|d| d.author_id == compte.id);

        // Un plan complet reste visible à qui y a déjà demandé sa place — le
        // masquer donnerait l'impression qu'il ne se passe rien, alors qu'il
        // s'y passe justement quelque chose.
        if places == 0 && !deja_demande {
            continue;
        }

        let jour = jour_local(&compte.timezone, ligne.starts_at.and_utc());
        retenus.push((
            jour,
            distance,
            PlanDuFil {
                id: ligne.id,
                title: ligne.title,
                note: ligne.note,
                category: ligne.category,
                starts_at: iso8601(ligne.starts_at.and_utc()),
                city: ligne.city,
                distance_km: distance,
                places_left: places,
                author_name: auteur.display_name,
                author_age: age,
                author_verified: auteur.verified,
                already_requested: deja_demande,
            },
        ));
    }

    // Imminence d'abord, proximité ensuite. Rien d'autre.
    //
    // Le tri porte sur un couple (jour, distance), pas sur un écart de temps
    // toléré : comparer « à moins de douze heures près » ne définit pas un
    // ordre total — A et B pourraient se comparer par la distance, B et C
    // aussi, et A et C par la date. Le résultat dépendrait alors de
    // l'algorithme de tri, ce qu'on ne peut pas se permettre sur un classement
    // qu'on promet explicable.
    retenus.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    });

    Ok(retenus
        .into_iter()
        .take(TAILLE_FIL)
        .map(|(_, _, plan)| plan)
        .collect())
}

/// Ne garde un critère que si le palier y donne droit.
///
/// Les critères sont contrôlés à l'écriture, mais un palier peut retomber
/// entre-temps : un abonnement qui expire laisserait sinon en place ce qui
/// avait été posé du temps où il courait. Il aurait suffi de s'abonner un
/// mois pour garder le bénéfice indéfiniment.
fn si_autorise<T>(palier: &str, critere: Critere, valeurs: Vec<T>) -> Vec<T> {
    if filtre_autorise(palier, critere) {
        valeurs
    } else {
        Vec::new()
    }
}
