//! Client APNs — notifications distantes et Live Activities.
//!
//! Trois types d'envoi sont utilisés par Weave :
//!
//!  • `liveactivity` avec `event: "start"`  → démarre à distance la Live
//!    Activity « Métier » sur l'iPhone, grâce au jeton « push-to-start »
//!    d'ActivityKit ;
//!  • `liveactivity` avec `event: "update"` → met à jour le compte à rebours
//!    d'un fil, sans réveiller l'application ;
//!  • `alert`                               → notification classique.
//!
//! Le jeton d'autorisation est signé en ES256 avec la clé .p8 d'Apple, et la
//! requête part en HTTP/2 — Apple ne parle pas autre chose.

use crate::env::Env;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::Serialize;
use serde_json::Value;
use std::{
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

/// Apple accepte un jeton d'autorisation pendant une heure ; on le renouvelle
/// avant, pour ne jamais présenter un jeton qui vient d'expirer.
const DUREE_JETON_SECONDES: u64 = 45 * 60;

const HOTE_PRODUCTION: &str = "https://api.push.apple.com";
const HOTE_BAC_A_SABLE: &str = "https://api.sandbox.push.apple.com";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeEnvoi {
    LiveActivity,
    Alerte,
}

impl TypeEnvoi {
    fn en_tete(self) -> &'static str {
        match self {
            TypeEnvoi::LiveActivity => "liveactivity",
            TypeEnvoi::Alerte => "alert",
        }
    }
}

pub struct Envoi<'a> {
    pub jeton_appareil: &'a str,
    pub type_envoi: TypeEnvoi,
    /// Suffixe de sujet : ActivityKit exige « .push-type.liveactivity ».
    pub suffixe_sujet: Option<&'a str>,
    pub priorite: u8,
    /// Clé de dédoublonnage : un seul état en vol par fil. Sans elle, deux
    /// mises à jour rapprochées feraient clignoter la bannière.
    pub collapse_id: Option<String>,
    pub charge: Value,
}

#[derive(Debug)]
pub struct Resultat {
    pub ok: bool,
    pub statut: u16,
    pub raison: Option<String>,
}

/// Les raisons pour lesquelles Apple dit qu'un jeton ne vaut plus rien.
/// Continuer à pousser dessus n'aboutira jamais : la ligne se ferme.
const JETONS_MORTS: [&str; 4] = [
    "BadDeviceToken",
    "Unregistered",
    "ExpiredToken",
    "DeviceTokenNotForTopic",
];

impl Resultat {
    pub fn jeton_mort(&self) -> bool {
        self.raison
            .as_deref()
            .is_some_and(|r| JETONS_MORTS.contains(&r))
    }
}

#[derive(Serialize)]
struct Claims {
    iss: String,
    iat: u64,
}

/// Client APNs. Le jeton d'autorisation est partagé entre les envois : en
/// forger un par notification coûterait une signature ES256 à chaque fois.
pub struct ClientApns {
    http: reqwest::Client,
    jeton: Arc<RwLock<Option<(String, u64)>>>,
}

impl ClientApns {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                // Apple n'accepte que HTTP/2.
                .http2_prior_knowledge()
                .build()
                .unwrap_or_default(),
            jeton: Arc::new(RwLock::new(None)),
        }
    }

    /// Envoie une notification, ou la simule si APNs n'est pas configuré.
    ///
    /// Une notification perdue ne doit jamais faire échouer la requête qui l'a
    /// déclenchée : accepter une demande reste utile même si l'écran verrouillé
    /// de l'autre ne s'allume pas.
    pub async fn envoyer(&self, config: &Env, envoi: Envoi<'_>) -> Resultat {
        if !config.apns.configure {
            tracing::debug!(
                type_envoi = envoi.type_envoi.en_tete(),
                "APNs non configuré, envoi simulé"
            );
            return Resultat {
                ok: true,
                statut: 200,
                raison: Some("simulated".to_string()),
            };
        }

        let jeton = match self.jeton_autorisation(config) {
            Ok(j) => j,
            Err(erreur) => {
                tracing::error!(erreur = %erreur, "jeton APNs non signé");
                return Resultat {
                    ok: false,
                    statut: 0,
                    raison: Some("jeton non signé".to_string()),
                };
            }
        };

        let hote = if config.apns.environnement == "production" {
            HOTE_PRODUCTION
        } else {
            HOTE_BAC_A_SABLE
        };
        let sujet = match envoi.suffixe_sujet {
            Some(suffixe) => format!("{}{suffixe}", config.apns.bundle_id),
            None => config.apns.bundle_id.clone(),
        };

        let requete = self
            .http
            .post(format!("{hote}/3/device/{}", envoi.jeton_appareil))
            .header("authorization", format!("bearer {jeton}"))
            .header("apns-topic", sujet)
            .header("apns-push-type", envoi.type_envoi.en_tete())
            .header("apns-priority", envoi.priorite.to_string());

        let requete = match &envoi.collapse_id {
            Some(cle) => requete.header("apns-collapse-id", cle.as_str()),
            None => requete,
        };

        let reponse = requete
            .json(&envoi.charge)
            .send()
            .await;

        match reponse {
            Ok(r) if r.status().is_success() => Resultat {
                ok: true,
                statut: r.status().as_u16(),
                raison: None,
            },
            Ok(r) => {
                let statut = r.status().as_u16();
                let raison = r
                    .json::<Value>()
                    .await
                    .ok()
                    .and_then(|v| v.get("reason").and_then(Value::as_str).map(str::to_string));
                tracing::warn!(statut, raison = ?raison, "APNs a refusé l'envoi");
                Resultat {
                    ok: false,
                    statut,
                    raison,
                }
            }
            Err(erreur) => {
                tracing::warn!(erreur = %erreur, "APNs injoignable");
                Resultat {
                    ok: false,
                    statut: 0,
                    raison: Some(erreur.to_string()),
                }
            }
        }
    }

    fn jeton_autorisation(&self, config: &Env) -> Result<String, String> {
        let maintenant = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();

        if let Ok(cache) = self.jeton.read() {
            if let Some((valeur, emis_le)) = cache.as_ref() {
                if maintenant - emis_le < DUREE_JETON_SECONDES {
                    return Ok(valeur.clone());
                }
            }
        }

        let chemin = config
            .apns
            .key_path
            .as_ref()
            .ok_or_else(|| "chemin de clé APNs absent".to_string())?;
        let pem = std::fs::read(chemin).map_err(|e| format!("clé APNs illisible : {e}"))?;

        let mut entete = Header::new(Algorithm::ES256);
        entete.kid = config.apns.key_id.clone();

        let claims = Claims {
            iss: config
                .apns
                .team_id
                .clone()
                .ok_or_else(|| "identifiant d'équipe APNs absent".to_string())?,
            iat: maintenant,
        };

        let cle = EncodingKey::from_ec_pem(&pem).map_err(|e| format!("clé APNs invalide : {e}"))?;
        let jeton = jsonwebtoken::encode(&entete, &claims, &cle)
            .map_err(|e| format!("signature APNs impossible : {e}"))?;

        if let Ok(mut cache) = self.jeton.write() {
            *cache = Some((jeton.clone(), maintenant));
        }
        Ok(jeton)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sans configuration APNs, l'envoi est simulé plutôt que d'échouer. Une
    /// notification perdue ne doit jamais faire échouer la requête qui l'a
    /// déclenchée : accepter une demande reste utile même si l'écran verrouillé
    /// de l'autre ne s'allume pas.
    #[tokio::test]
    async fn sans_configuration_l_envoi_est_simule() {
        let config = crate::env::Env {
            mode: crate::env::Mode::Development,
            port: 0,
            db: crate::env::Db {
                driver: crate::env::Driver::Sqlite,
                url: String::new(),
                ssl_insecure: false,
            },
            cache: crate::env::Cache {
                url: String::new(),
                tls_insecure: false,
            },
            media: crate::env::Media {
                signing_secret: String::new(),
                base_url: String::new(),
                ttl_url_signee_secondes: 600,
            },
            apns: crate::env::Apns {
                key_id: None,
                team_id: None,
                key_path: None,
                bundle_id: "com.weave.app".to_string(),
                environnement: "sandbox".to_string(),
                configure: false,
            },
            app_store: crate::env::AppStore {
                environnement: "sandbox".to_string(),
                configure: false,
            },
            auth: crate::env::Auth {
                jwt_secret: String::new(),
                access_ttl_secondes: 900,
                refresh_ttl_jours: 60,
            },
            web_origin: String::new(),
            web_dist: None,
        };

        let resultat = ClientApns::new()
            .envoyer(
                &config,
                Envoi {
                    jeton_appareil: "jeton",
                    type_envoi: TypeEnvoi::Alerte,
                    suffixe_sujet: None,
                    priorite: 10,
                    collapse_id: None,
                    charge: serde_json::json!({}),
                },
            )
            .await;

        assert!(resultat.ok);
        assert_eq!(resultat.raison.as_deref(), Some("simulated"));
    }

    #[test]
    fn le_type_d_envoi_porte_l_en_tete_attendu_par_apple() {
        assert_eq!(TypeEnvoi::LiveActivity.en_tete(), "liveactivity");
        assert_eq!(TypeEnvoi::Alerte.en_tete(), "alert");
    }
}
