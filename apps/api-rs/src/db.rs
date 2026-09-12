//! Connexion à la base, par SeaORM.
//!
//! Deux moteurs, comme du temps de Prisma : PostgreSQL en production, SQLite
//! au développement. Le schéma est identique ; seule l'URL change.

use crate::env::{Db, Driver};
use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};
use std::time::Duration;

pub async fn connecter(cfg: &Db) -> Result<DatabaseConnection, DbErr> {
    let mut options = ConnectOptions::new(url_effective(cfg));
    options
        .max_connections(10)
        .connect_timeout(Duration::from_secs(10))
        .acquire_timeout(Duration::from_secs(10))
        .sqlx_logging(false);

    Database::connect(options).await
}

/// Heroku présente un certificat auto-signé sur son réseau interne. Le trafic
/// reste chiffré, mais l'identité du serveur n'est pas vérifiée — acceptable
/// parce que la base est jointe par le réseau privé de l'hébergeur, et
/// seulement pour cette raison.
fn url_effective(cfg: &Db) -> String {
    if cfg.driver != Driver::Postgres || !cfg.ssl_insecure {
        return cfg.url.clone();
    }
    if cfg.url.contains("sslmode=") {
        return cfg.url.clone();
    }
    let separateur = if cfg.url.contains('?') { '&' } else { '?' };
    format!("{}{}sslmode=require", cfg.url, separateur)
}
