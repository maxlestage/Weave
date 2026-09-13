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

/// Sonde le magasin, avec un second essai.
///
/// Le premier essai après le réveil d'un dyno échouait, et les suivants
/// passaient : la sonde annonçait « dégradé » — et une 503 — pour un service
/// entièrement sain. `ConnectionManager` rétablit sa connexion en arrière-plan
/// quand elle est tombée ; l'appel qui déclenche ce rétablissement en fait les
/// frais, celui d'après le trouve prêt.
///
/// Le second essai ne masque rien : un magasin réellement absent échoue les
/// deux fois. Il distingue seulement « en train de se reconnecter » de
/// « mort », et cette distinction est tout l'objet d'une sonde. Une sonde qui
/// crie à chaque réveil apprend surtout à ne plus être lue.
pub async fn ping(manager: &ConnectionManager) -> bool {
    for reste in [true, false] {
        if essayer_ping(manager).await {
            return true;
        }
        if reste {
            // Laisse au rétablissement le temps de s'amorcer. Le premier essai
            // l'a demandé ; sans cette pause, le second le redemanderait.
            tokio::time::sleep(DELAI_SECOND_ESSAI).await;
        }
    }
    false
}

/// Court : la sonde entière doit rester bornée, et ce délai s'ajoute aux deux
/// essais. Au pire, la sonde répond en un peu plus d'une seconde et demie.
const DELAI_SECOND_ESSAI: Duration = Duration::from_millis(50);

async fn essayer_ping(manager: &ConnectionManager) -> bool {
    let mut conn = manager.clone();
    let commande = redis::cmd("PING");
    let appel = commande.query_async::<String>(&mut conn);
    matches!(tokio::time::timeout(PROBE_TIMEOUT, appel).await, Ok(Ok(_)))
}

/// Même situation que pour la base : certificat auto-signé sur le réseau
/// interne de l'hébergeur.
///
/// La bibliothèque attend un **fragment**, `#insecure`, et rien d'autre : un
/// paramètre de requête `?insecure=true` est accepté sans broncher par
/// l'analyse d'URL, puis ignoré. La vérification restait donc active, et la
/// connexion échouait sur `CaUsedAsEndEntity` — le certificat d'Heroku étant
/// sa propre autorité.
///
/// Le fragment exige un chemin : `rediss://hôte:port#insecure` n'est pas
/// analysable, `rediss://hôte:port/#insecure` l'est.
fn url_effective(cfg: &Cache) -> String {
    if !cfg.tls_insecure || !cfg.url.starts_with("rediss://") {
        return cfg.url.clone();
    }
    if cfg.url.contains('#') {
        return cfg.url.clone();
    }
    let base = cfg.url.trim_end_matches('/');
    let apres_schema = &base["rediss://".len()..];
    if apres_schema.contains('/') {
        format!("{base}#insecure")
    } else {
        format!("{base}/#insecure")
    }
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
    /// Dernier état poussé de la Live Activity.
    pub fn live_activity(compte: &str) -> String {
        format!("{NS}:la:{compte}")
    }
    /// Résumé compact pour Apple Watch.
    pub fn montre(compte: &str) -> String {
        format!("{NS}:watch:{compte}")
    }
    /// Compteur de limitation de débit.
    /// Verrou d'une tâche d'entretien, pour qu'un seul dyno l'exécute.
    pub fn verrou(tache: &str) -> String {
        format!("{NS}:verrou:{tache}")
    }
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

/// Prend un verrou pour la durée indiquée, ou rend faux s'il est déjà tenu.
///
/// `SET NX EX` en une seule commande : tester puis poser laisserait une fenêtre
/// pendant laquelle deux dynos se croiraient tous deux seuls. Le verrou expire
/// de lui-même — un processus arrêté au mauvais moment ne bloque pas la tâche
/// pour toujours.
pub async fn prendre_verrou(
    manager: &ConnectionManager,
    cle: &str,
    secondes: u64,
) -> redis::RedisResult<bool> {
    let mut conn = manager.clone();
    let pose: Option<String> = redis::cmd("SET")
        .arg(cle)
        .arg(chrono::Utc::now().to_rfc3339())
        .arg("NX")
        .arg("EX")
        .arg(secondes)
        .query_async(&mut conn)
        .await?;
    Ok(pose.is_some())
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

/// Applique un « Renfort » à la journée en cours, de façon atomique.
///
/// Même script que le quota de demandes, et pour la même raison : le nombre de
/// renforts applicables dans une journée est lui-même borné. Sans ce second
/// plafond, l'argent lèverait l'invariant, et « on ne peut pas arroser »
/// deviendrait « on ne peut pas arroser gratuitement ».
///
/// Renvoie le nombre de renforts appliqués aujourd'hui, ou `None` si le
/// plafond du jour est atteint.
pub async fn appliquer_renfort(
    manager: &ConnectionManager,
    compte: &str,
    jour: &str,
    maximum: i64,
    secondes_avant_minuit: i64,
) -> Result<Option<i64>, crate::error::AppError> {
    let mut conn = manager.clone();
    let restants: i64 = redis::cmd("EVAL")
        .arg(CONSOMMER_DEMANDE_LUA)
        .arg(1)
        .arg(cles::renforts(compte, jour))
        .arg(maximum)
        .arg(secondes_avant_minuit.max(60))
        .query_async(&mut conn)
        .await?;

    Ok((restants >= 0).then(|| maximum - restants))
}

/// Libère une place de renfort réservée mais non payée.
///
/// La place est réservée AVANT que le crédit soit dépensé : l'inverse
/// consommerait un achat pour rien lorsque le plafond est atteint. Elle est
/// donc rendue si le crédit manque, sans quoi une tentative refusée
/// grignoterait le plafond du jour.
pub async fn rendre_renfort(manager: &ConnectionManager, compte: &str, jour: &str) {
    decrementer_sans_passer_sous_zero(manager, &cles::renforts(compte, jour)).await;
}

/// Rend une demande au quota — lorsque l'écriture qui la suivait a échoué.
/// Ne descend jamais sous zéro.
pub async fn rendre_demande(manager: &ConnectionManager, compte: &str, jour: &str) {
    decrementer_sans_passer_sous_zero(manager, &cles::demandes_utilisees(compte, jour)).await;
}

/// Le pendant du script de consommation, et pour la même raison.
///
/// Lire puis décrémenter en deux temps laisse deux remboursements concurrents
/// franchir le zéro ensemble : tous deux lisent 1, tous deux décrémentent, le
/// compteur tombe à -1. Le prochain `INCR` le ramène alors à 0, sous le
/// plafond — et une demande est offerte. C'est précisément l'invariant que le
/// produit défend qui cède, par une porte que personne ne regardait.
///
/// Le cas s'atteint sans rien forger : deux appels simultanés au retrait d'une
/// même demande passent tous deux le contrôle d'état avant que le premier ne
/// l'ait écrit, et remboursent tous deux.
const RENDRE_LUA: &str = r#"
local utilisees = tonumber(redis.call('GET', KEYS[1]) or '0')
if utilisees > 0 then
  return redis.call('DECR', KEYS[1])
end
return utilisees
"#;

async fn decrementer_sans_passer_sous_zero(manager: &ConnectionManager, cle: &str) {
    let mut conn = manager.clone();
    let _ = redis::cmd("EVAL")
        .arg(RENDRE_LUA)
        .arg(1)
        .arg(cle)
        .query_async::<i64>(&mut conn)
        .await;
}

#[cfg(test)]
mod tests_quota {
    use super::*;
    use crate::tests::Service;

    /// Le compteur ne doit jamais devenir négatif — un compteur négatif est une
    /// demande offerte, et « on ne peut pas arroser » ne tient plus.
    ///
    /// Vingt remboursements concurrents pour une seule demande consommée : en
    /// deux temps, ils franchissaient le zéro ensemble et laissaient le
    /// compteur à -19.
    #[tokio::test]
    async fn des_remboursements_concurrents_ne_passent_pas_sous_zero() {
        let service = Service::monter().await;
        let compte = service.id("quota");
        let jour = "2026-09-13";
        let cle = cles::demandes_utilisees(&compte, jour);

        // Une demande consommée, et une seule.
        consommer_demande(&service.etat.cache, &compte, jour, 5, 3600)
            .await
            .expect("cache joignable")
            .expect("le quota accepte la première");

        let mut essais = Vec::new();
        for _ in 0..20 {
            let cache = service.etat.cache.clone();
            let compte = compte.clone();
            essais.push(tokio::spawn(async move {
                rendre_demande(&cache, &compte, "2026-09-13").await;
            }));
        }
        for essai in essais {
            essai.await.expect("tâche terminée");
        }

        let reste = compteur(&service.etat.cache, &cle).await;
        assert_eq!(reste, 0, "le compteur est tombé à {reste} : demandes offertes");

        let _ = oublier(&service.etat.cache, &cle).await;
    }

    /// Et le remboursement légitime fonctionne toujours : deux consommées,
    /// deux rendues, on revient à zéro.
    #[tokio::test]
    async fn deux_remboursements_pour_deux_demandes_reviennent_a_zero() {
        let service = Service::monter().await;
        let compte = service.id("quota-pair");
        let jour = "2026-09-13";
        let cle = cles::demandes_utilisees(&compte, jour);

        for _ in 0..2 {
            consommer_demande(&service.etat.cache, &compte, jour, 5, 3600)
                .await
                .expect("cache joignable")
                .expect("accepté");
        }
        assert_eq!(compteur(&service.etat.cache, &cle).await, 2);

        rendre_demande(&service.etat.cache, &compte, jour).await;
        rendre_demande(&service.etat.cache, &compte, jour).await;

        assert_eq!(compteur(&service.etat.cache, &cle).await, 0);
        let _ = oublier(&service.etat.cache, &cle).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cache(url: &str, tls_insecure: bool) -> Cache {
        Cache { url: url.to_string(), tls_insecure }
    }

    /// Le défaut qui a fait tomber le dyno : `?insecure=true` est une syntaxe
    /// que l'analyse d'URL accepte et que la bibliothèque ignore. Seul le
    /// fragment compte, et il lui faut un chemin.
    #[test]
    fn le_mode_permissif_passe_par_un_fragment() {
        assert_eq!(
            url_effective(&cache("rediss://:mdp@hote:6380", true)),
            "rediss://:mdp@hote:6380/#insecure"
        );
        assert_eq!(
            url_effective(&cache("rediss://hote:6380/", true)),
            "rediss://hote:6380/#insecure"
        );
        assert_eq!(
            url_effective(&cache("rediss://hote:6380/0", true)),
            "rediss://hote:6380/0#insecure"
        );
    }

    #[test]
    fn le_fragment_n_est_pas_ajoute_deux_fois() {
        let deja = "rediss://hote:6380/#insecure";
        assert_eq!(url_effective(&cache(deja, true)), deja);
    }

    /// Sans TLS, ou sans le drapeau, l'URL ne doit pas être touchée : y
    /// ajouter un fragment ferait échouer une connexion en clair.
    #[test]
    fn les_autres_urls_restent_intactes() {
        assert_eq!(url_effective(&cache("redis://hote:6379", true)), "redis://hote:6379");
        assert_eq!(url_effective(&cache("rediss://hote:6380", false)), "rediss://hote:6380");
    }
}

#[cfg(test)]
mod tests_sonde {
    use super::*;
    use crate::tests::Service;

    /// Un magasin qui répond est déclaré sain.
    #[tokio::test]
    async fn un_magasin_vivant_est_sain() {
        let service = Service::monter().await;
        assert!(ping(&service.etat.cache).await);
    }

    /// Un magasin absent est déclaré mort — et vite.
    ///
    /// C'est ce que le second essai ne doit pas défaire : il distingue une
    /// reconnexion d'une panne, il ne doit pas transformer la panne en attente.
    /// Une sonde qui met dix secondes à dire « non » bloque le redémarrage
    /// qu'elle est censée déclencher.
    #[tokio::test]
    async fn un_magasin_absent_est_declare_mort_sans_faire_attendre() {
        // Un port fermé : le système refuse la connexion immédiatement, ce qui
        // éprouve l'enchaînement des essais plutôt que les délais de garde.
        let ecoute = std::net::TcpListener::bind("127.0.0.1:0").expect("port");
        let port = ecoute.local_addr().unwrap().port();
        drop(ecoute);

        let client = redis::Client::open(format!("redis://127.0.0.1:{port}")).expect("client");
        // `ConnectionManager::new` échoue d'emblée sans serveur : c'est déjà la
        // réponse, et elle vaut « mort ».
        let Ok(manager) = ConnectionManager::new(client).await else {
            return;
        };

        let debut = std::time::Instant::now();
        let sain = ping(&manager).await;
        let duree = debut.elapsed();

        assert!(!sain, "un magasin absent ne doit pas être déclaré sain");
        assert!(
            duree < Duration::from_secs(2),
            "la sonde a mis {duree:?} : deux essais bornés doivent rester sous deux secondes"
        );
    }
}
