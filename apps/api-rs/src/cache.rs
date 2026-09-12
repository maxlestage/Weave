//! Magasin clé-valeur.
//!
//! Le cache n'est pas un confort dans Weave : le quota de demandes n'y vit
//! qu'ici, et sans lui l'invariant central ne tient plus. C'est pourquoi la
//! sonde de santé le déclare « requis ».

use crate::env::Cache;
use redis::aio::ConnectionManager;
use std::time::Duration;

/// Borne des sondes. Une dépendance qui ne répond pas doit être rapportée
/// comme telle, jamais faire attendre la réponse : vu de l'extérieur, un
/// service qui ne répond pas est indiscernable d'un service mort.
pub const PROBE_TIMEOUT: Duration = Duration::from_millis(750);

pub async fn connecter(cfg: &Cache) -> redis::RedisResult<ConnectionManager> {
    let client = redis::Client::open(url_effective(cfg))?;
    ConnectionManager::new(client).await
}

pub async fn ping(manager: &ConnectionManager) -> bool {
    let mut conn = manager.clone();
    let commande = redis::cmd("PING");
    let appel = commande.query_async::<String>(&mut conn);
    matches!(tokio::time::timeout(PROBE_TIMEOUT, appel).await, Ok(Ok(_)))
}

/// Même situation que pour la base : certificat auto-signé sur le réseau
/// interne de l'hébergeur.
fn url_effective(cfg: &Cache) -> String {
    if !cfg.tls_insecure || !cfg.url.starts_with("rediss://") {
        return cfg.url.clone();
    }
    if cfg.url.contains("insecure") {
        return cfg.url.clone();
    }
    let separateur = if cfg.url.contains('?') { '&' } else { '?' };
    format!("{}{}insecure=true", cfg.url, separateur)
}
