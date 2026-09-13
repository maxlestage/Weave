//! Point d'entrée de l'API Weave.
//!
//! Pile : Rust + Axum + SeaORM + Redis.

mod alerte;
mod apns;
mod auth;
mod cache;
mod crypto;
mod db;
mod droits;
mod error;
mod limitation;
mod live_activity;
mod migrations;
mod purge;
mod routes;
mod temps;

#[cfg(test)]
mod tests;
mod env;

// Les 18 entités sont engendrées d'un bloc depuis la base ; celles qu'aucune
// route ne consomme encore signaleraient du code mort à chaque compilation.
#[allow(dead_code)]
mod entities;

use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use redis::aio::ConnectionManager;
use sea_orm::{ConnectionTrait, DatabaseConnection};
use serde_json::json;
use std::{sync::Arc, time::Duration};
use tower_http::cors::{AllowOrigin, CorsLayer};

#[derive(Clone)]
struct AppState {
    db: DatabaseConnection,
    cache: ConnectionManager,
    config: Arc<env::Env>,
    /// Partagé : le jeton d'autorisation APNs vit dans le client, et en forger
    /// un par notification coûterait une signature ES256 à chaque fois.
    apns: Arc<apns::ClientApns>,
}

/// Choisit le fournisseur cryptographique de rustls, avant tout usage de TLS.
///
/// Trois dépendances tirent rustls — `redis` pour Heroku Redis, `sqlx` pour
/// PostgreSQL, `reqwest` pour APNs — et elles n'activent pas le même
/// fournisseur : `aws-lc-rs` d'un côté, `ring` de l'autre. rustls voit les deux
/// features actives, refuse de trancher à notre place, et **panique** au
/// premier handshake :
///
/// ```text
/// Could not automatically determine the process-level CryptoProvider
/// from Rustls crate features.
/// ```
///
/// Rien ne le révèle en développement : en local, Redis et PostgreSQL se
/// joignent en clair, aucun handshake n'a lieu et le code ne s'exécute jamais.
/// Sur Heroku, Redis est en `rediss://` — le dyno s'arrêtait donc au démarrage,
/// sur un code 101 que rien ne relie à la configuration.
///
/// L'appel doit précéder la première connexion chiffrée, d'où sa place en tête
/// de `main`. Il échoue seulement si un fournisseur est déjà installé, ce qui
/// n'a rien d'un problème : l'objectif est atteint.
fn installer_fournisseur_tls() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    installer_fournisseur_tls();
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt().with_target(false).json().init();

    let config = match env::charger() {
        Ok(c) => c,
        Err(rapport) => {
            // La configuration dit elle-même ce qui manque, et le remède de
            // chaque ligne. Rien à ajouter ici.
            eprint!("{rapport}");
            std::process::exit(1);
        }
    };

    let db = db::connecter(&config.db).await?;

    // `weave-api migrate` : la phase de publication d'Heroku l'appelle avant
    // que la nouvelle version ne reçoive du trafic. Si elle échoue, la version
    // précédente reste en ligne — d'où un processus séparé, qui ne démarre ni
    // le serveur ni le cache.
    if std::env::args().nth(1).as_deref() == Some("migrate") {
        let posees = migrations::appliquer(&db).await?;
        if posees.is_empty() {
            println!("  • aucune migration à appliquer");
        } else {
            for nom in posees {
                println!("  • appliquée : {nom}");
            }
        }
        return Ok(());
    }

    // `weave-api purge` : même logique que `migrate` — un processus séparé,
    // qui ne démarre ni serveur ni cache, et qu'un planificateur appelle.
    if std::env::args().nth(1).as_deref() == Some("purge") {
        let bilan = purge::executer(&db).await?;
        println!("  • messages effacés  : {}", bilan.messages_effaces);
        println!("  • activités effacées : {}", bilan.activites_effacees);
        println!("  • comptes effacés   : {}", bilan.comptes_effaces);
        if bilan.comptes_differes > 0 {
            println!(
                "  • comptes différés : {} (signalement encore ouvert)",
                bilan.comptes_differes
            );
        }
        return Ok(());
    }

    let cache = cache::connecter(&config.cache).await?;

    let origine = if config.is_production() {
        AllowOrigin::exact(config.web_origin.parse()?)
    } else {
        AllowOrigin::any()
    };

    let port = config.port;
    let driver = config.db.driver.as_str();

    let state = AppState {
        db,
        cache,
        config: Arc::new(config),
        apns: Arc::new(apns::ClientApns::new()),
    };

    // La purge tourne depuis le service, faute de planificateur externe. Un
    // verrou dans le cache garantit un seul passage par jour, quel que soit le
    // nombre de dynos.
    purge::planifier(state.clone());

    let app = construire_routeur(state).layer(CorsLayer::new().allow_origin(origine));

    let ecoute = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!(port, database = driver, "API Weave démarrée");

    // `into_make_service_with_connect_info` est requis par les routes qui
    // limitent le débit par adresse : sans lui, l'extracteur `ConnectInfo`
    // échoue à l'exécution, pas à la compilation.
    axum::serve(
        ecoute,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(arret_demande())
    .await?;

    Ok(())
}

/// Assemble toutes les routes du service.
///
/// Extrait de `main` pour que les tests puissent monter le service entier —
/// et non chaque gestionnaire isolément. Un gestionnaire juste derrière un
/// routage faux ne rend toujours pas le bon service.
fn construire_routeur(state: AppState) -> Router {
    let vitrine = state.config.web_dist.clone();
    let api = Router::new()
        .route("/health", get(health))
        .merge(routes::auth::routes())
        .merge(routes::me::routes())
        .merge(routes::plans::routes())
        .merge(routes::requests::routes())
        .merge(routes::conversations::routes())
        .merge(routes::moderation::routes())
        .merge(routes::devices::routes())
        .merge(routes::export::routes())
        .merge(routes::media::routes())
        .merge(routes::fil::routes())
        .merge(routes::billing::routes())
        .with_state(state);

    monter_vitrine(api, vitrine.as_deref())
}

/// Monte le site vitrine sous les routes de l'API.
///
/// Le site est statique et peu visité : lui dédier un second dyno doublerait
/// la facture sans rien apporter. Il passe en recours, jamais devant l'API —
/// un fichier nommé `health` dans `dist` ne doit pas éteindre la sonde.
///
/// En développement il n'y a pas de `dist` : `WEB_DIST_PATH` n'est pas défini
/// et aucune route n'est ajoutée. Un site absent là où on l'attendait est
/// signalé, mais n'empêche pas l'API de démarrer : l'application iOS en
/// dépend, et elle n'a que faire de la vitrine.
fn monter_vitrine(routeur: Router, chemin: Option<&str>) -> Router {
    let Some(chemin) = chemin else {
        return routeur;
    };

    if !std::path::Path::new(chemin).join("index.html").is_file() {
        tracing::warn!(chemin, "site vitrine introuvable, l'API démarre sans lui");
        return routeur;
    }

    let fichiers = tower_http::services::ServeDir::new(chemin)
        // Sans cela, `/` — un répertoire — rend une 404 : c'est l'adresse du
        // service elle-même qui serait vide.
        .append_index_html_on_directories(true);

    let vitrine = Router::new()
        .fallback_service(fichiers)
        .layer(axum::middleware::from_fn(servir_vitrine));

    routeur.fallback_service(vitrine)
}

/// Ce que le recours ajoute autour des fichiers : la méthode, puis le cache.
///
/// La construction appose une empreinte au nom des scripts, des styles et des
/// images : leur contenu ne change jamais sous un même nom, ils se gardent
/// indéfiniment. `index.html`, lui, garde les noms des autres : le mettre en
/// cache une heure servirait l'ancien site une heure après chaque publication.
async fn servir_vitrine(requete: Request, suite: Next) -> Response {
    // Un POST sur une adresse d'API mal orthographiée arrive ici, et le
    // service de fichiers répondrait « méthode interdite » : un contresens,
    // qui laisse croire que la ressource existe. Elle n'existe pas.
    if !matches!(*requete.method(), axum::http::Method::GET | axum::http::Method::HEAD) {
        return StatusCode::NOT_FOUND.into_response();
    }

    let chemin = requete.uri().path();
    let html = chemin == "/" || chemin.ends_with('/') || chemin.ends_with(".html");

    let mut reponse = suite.run(requete).await;
    if reponse.status().is_success() {
        reponse.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static(if html {
                "no-cache"
            } else {
                "public, max-age=31536000, immutable"
            }),
        );
    }
    reponse
}

/// Sonde de santé.
///
/// Les deux sondes sont bornées : une dépendance qui ne répond pas doit être
/// rapportée comme telle, jamais faire attendre la réponse.
async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let sonde_db = async {
        matches!(
            tokio::time::timeout(
                cache::PROBE_TIMEOUT,
                state.db.execute_unprepared("SELECT 1"),
            )
            .await,
            Ok(Ok(_))
        )
    };

    let (db_ok, cache_ok) = tokio::join!(sonde_db, cache::ping(&state.cache));
    let sain = db_ok && cache_ok;

    let corps = json!({
        "status": if sain { "ok" } else { "degraded" },
        "database": { "driver": state.config.db.driver.as_str(), "ok": db_ok },
        // Le cache n'est pas un confort dans Weave : le quota de demandes n'y
        // vit qu'ici, et sans lui l'invariant central ne tient plus.
        "cache": { "ok": cache_ok, "required": true },
        "version": "0.1.0",
    });

    let code = if sain {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (code, Json(corps))
}

async fn arret_demande() {
    let ctrl_c = async { tokio::signal::ctrl_c().await.ok() };
    #[cfg(unix)]
    let sigterm = async {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
                Some(())
            }
            Err(_) => None,
        }
    };
    #[cfg(not(unix))]
    let sigterm = std::future::pending::<Option<()>>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = sigterm => {},
    }
    tracing::info!("Arrêt demandé");
    // Laisse aux requêtes en vol le temps de se terminer.
    tokio::time::sleep(Duration::from_millis(50)).await;
}
