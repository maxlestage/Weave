//! Point d'entrée de l'API Weave.
//!
//! Pile : Rust + Axum + SeaORM + Redis.

mod apns;
mod auth;
mod cache;
mod crypto;
mod db;
mod droits;
mod error;
mod limitation;
mod routes;
mod temps;

#[cfg(test)]
mod tests;
mod env;

// Les 18 entités sont engendrées d'un bloc depuis la base ; celles qu'aucune
// route ne consomme encore signaleraient du code mort à chaque compilation.
#[allow(dead_code)]
mod entities;

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
    };

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
    Router::new()
        .route("/health", get(health))
        .merge(routes::auth::routes())
        .merge(routes::me::routes())
        .merge(routes::plans::routes())
        .merge(routes::requests::routes())
        .merge(routes::conversations::routes())
        .merge(routes::moderation::routes())
        .merge(routes::devices::routes())
        .merge(routes::media::routes())
        .merge(routes::fil::routes())
        .merge(routes::billing::routes())
        .with_state(state)
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
