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

/* ------------------------------------------------------------------ */
/* Clés                                                                */
/* ------------------------------------------------------------------ */

/// Espace de noms, repris tel quel de l'API TypeScript : les entrées déjà
/// posées par le service en place doivent rester lisibles pendant la bascule.
const NS: &str = "weave:v2";

pub mod cles {
    use super::NS;

    /// Fil composé pour un compte.
    pub fn fil(compte: &str) -> String {
        format!("{NS}:feed:{compte}")
    }
    /// Demandes déjà envoyées aujourd'hui. Expire à minuit, heure locale.
    pub fn demandes_utilisees(compte: &str, jour: &str) -> String {
        format!("{NS}:req:{compte}:{jour}")
    }
    /// « Renforts » déjà appliqués aujourd'hui. Expire à minuit, heure locale.
    pub fn renforts(compte: &str, jour: &str) -> String {
        format!("{NS}:renfort:{compte}:{jour}")
    }
    /// Identité résumée, pour éviter un aller-retour base à chaque requête.
    pub fn identite(compte: &str) -> String {
        format!("{NS}:me:{compte}")
    }
    /// Compteur de limitation de débit.
    pub fn limitation(seau: &str, sujet: &str) -> String {
        format!("{NS}:rl:{seau}:{sujet}")
    }
}

/// Lit une valeur JSON. Une entrée illisible est traitée comme absente : le
/// cache n'est pas une source de vérité, et un format qui a changé ne doit pas
/// faire échouer la requête.
pub async fn lire_json<T: serde::de::DeserializeOwned>(
    manager: &ConnectionManager,
    cle: &str,
) -> Option<T> {
    let mut conn = manager.clone();
    let brut: Option<String> = redis::cmd("GET")
        .arg(cle)
        .query_async(&mut conn)
        .await
        .ok()?;
    serde_json::from_str(&brut?).ok()
}

pub async fn ecrire_json<T: serde::Serialize>(
    manager: &ConnectionManager,
    cle: &str,
    valeur: &T,
    ttl_secondes: u64,
) -> Result<(), crate::error::AppError> {
    let mut conn = manager.clone();
    let charge = serde_json::to_string(valeur).map_err(|erreur| {
        tracing::error!(erreur = %erreur, cle, "valeur non sérialisable pour le cache");
        crate::error::AppError::new(
            crate::error::Code::Internal,
            "Une erreur interne est survenue.",
        )
    })?;
    redis::cmd("SET")
        .arg(cle)
        .arg(charge)
        .arg("EX")
        .arg(ttl_secondes)
        .query_async::<()>(&mut conn)
        .await?;
    Ok(())
}

pub async fn oublier(manager: &ConnectionManager, cle: &str) -> redis::RedisResult<()> {
    let mut conn = manager.clone();
    redis::cmd("DEL").arg(cle).query_async::<()>(&mut conn).await
}

/// Lit un compteur entier. Une clé absente vaut zéro, une valeur illisible
/// aussi : ces compteurs se reconstruisent, ils ne justifient pas un échec.
pub async fn compteur(manager: &ConnectionManager, cle: &str) -> i64 {
    let mut conn = manager.clone();
    redis::cmd("GET")
        .arg(cle)
        .query_async::<Option<String>>(&mut conn)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// Consomme une demande du quota du jour, de façon atomique.
///
/// Le script incrémente puis compare : si le plafond est dépassé, il annule son
/// propre incrément et renvoie -1. Une vérification suivie d'une écriture en
/// deux temps laisserait passer deux demandes concurrentes sur la dernière
/// place — et l'invariant central du produit ne tiendrait plus.
const CONSOMMER_DEMANDE_LUA: &str = r#"
local plafond = tonumber(ARGV[1])
local expiration = tonumber(ARGV[2])

local utilisees = redis.call('INCR', KEYS[1])
if utilisees == 1 then
  redis.call('EXPIRE', KEYS[1], expiration)
end

if utilisees > plafond then
  redis.call('DECR', KEYS[1])
  return -1
end

return plafond - utilisees
"#;

/// Renvoie le nombre de demandes restantes, ou `None` si le quota est épuisé.
pub async fn consommer_demande(
    manager: &ConnectionManager,
    compte: &str,
    jour: &str,
    quota: i64,
    secondes_avant_minuit: i64,
) -> Result<Option<i64>, crate::error::AppError> {
    let mut conn = manager.clone();
    // Au moins une minute d'expiration : une clé qui expirerait à l'instant
    // rendrait le quota au mauvais moment.
    let expiration = secondes_avant_minuit.max(60);

    let restantes: i64 = redis::cmd("EVAL")
        .arg(CONSOMMER_DEMANDE_LUA)
        .arg(1)
        .arg(cles::demandes_utilisees(compte, jour))
        .arg(quota)
        .arg(expiration)
        .query_async(&mut conn)
        .await?;

    Ok((restantes >= 0).then_some(restantes))
}

/// Rend une demande au quota — lorsque l'écriture qui la suivait a échoué.
/// Ne descend jamais sous zéro.
pub async fn rendre_demande(manager: &ConnectionManager, compte: &str, jour: &str) {
    let cle = cles::demandes_utilisees(compte, jour);
    let mut conn = manager.clone();
    let utilisees: i64 = redis::cmd("GET")
        .arg(&cle)
        .query_async::<Option<String>>(&mut conn)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    if utilisees > 0 {
        let _ = redis::cmd("DECR").arg(&cle).query_async::<i64>(&mut conn).await;
    }
}
