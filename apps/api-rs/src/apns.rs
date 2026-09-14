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
    /// Jusqu'à quand APNs doit garder la notification, en secondes UNIX.
    ///
    /// Sans cet en-tête, rien n'est gardé. La documentation d'Apple est
    /// explicite : « If the value is nonzero, APNs stores the notification and
    /// tries to deliver it at least once, repeating the attempt as needed
    /// until the specified date. If the value is `0`, APNs attempts to deliver
    /// the notification only once and doesn't store it. » Et plus haut :
    /// APNs garde une notification « for 30 days or less, depending on the
    /// date you specify in the `apns-expiration` header ».
    ///
    /// L'en-tête n'était pas envoyé du tout. Une alerte partie vers un
    /// téléphone éteint ou hors réseau était donc perdue, sans reprise —
    /// « quelqu'un veut venir » compris, qui est le seul message du produit
    /// qui attende une réponse.
    ///
    /// Obligatoire, et non facultatif : il n'existe aucun envoi de Weave qu'on
    /// accepterait de perdre, et un champ qu'on peut laisser vide finit par
    /// l'être. Le compilateur tient l'invariant à chaque nouvel appel.
    pub peremption: i64,
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
const JETONS_MORTS: [&str; 3] = ["BadDeviceToken", "Unregistered", "ExpiredToken"];

/// `DeviceTokenNotForTopic` n'est pas de ceux-là, et l'y compter coûtait cher.
///
/// Cette raison dit que le jeton n'est pas pour CE sujet. Elle ne dit pas
/// lequel des deux est en cause — un jeton périmé, ou notre `apns-topic`. Le
/// serveur ne peut pas trancher, et les deux lectures ne se paient pas pareil.
///
/// Si le jeton est mort, le garder coûte quelques envois dans le vide jusqu'à
/// ce que l'appareil se réinscrive. Si c'est `APNS_BUNDLE_ID` qui est faux,
/// l'effacer vide **toutes** les inscriptions à la fois — jetons d'alerte,
/// jetons de démarrage, sessions closes — et plus rien ne part tant que chaque
/// personne n'a pas rouvert l'application. Une faute d'un caractère dans une
/// variable de configuration, au lancement, c'est-à-dire au pire moment.
///
/// On garde donc le jeton, et on le dit assez fort pour qu'on aille vérifier.
const SUJET_SUSPECT: &str = "DeviceTokenNotForTopic";

impl Resultat {
    pub fn jeton_mort(&self) -> bool {
        self.raison
            .as_deref()
            .is_some_and(|r| JETONS_MORTS.contains(&r))
    }

    /// Le refus met-il en cause notre configuration plutôt que le jeton ?
    pub fn sujet_suspect(&self) -> bool {
        self.raison.as_deref() == Some(SUJET_SUSPECT)
    }
}

#[derive(Serialize)]
struct Claims {
    iss: String,
    iat: u64,
}

/// L'adresse à laquelle APNs attend la notification.
fn adresse(config: &Env, jeton_appareil: &str) -> String {
    let hote = if config.apns.environnement == "production" {
        HOTE_PRODUCTION
    } else {
        HOTE_BAC_A_SABLE
    };
    format!("{hote}/3/device/{jeton_appareil}")
}

/// Les en-têtes d'un envoi, hors autorisation.
///
/// Fonction à part, et c'est le point : ces cinq valeurs décident si la
/// notification arrive, et aucune ne se voit dans un journal. Un sujet sans le
/// suffixe d'ActivityKit, un type d'envoi qui ne correspond pas, une
/// péremption oubliée — APNs refuse, ou accepte puis ne livre rien. Écrites
/// dans le constructeur de requête, elles n'étaient éprouvables que par un
/// vrai appel à Apple ; ici, elles se lisent.
fn en_tetes(config: &Env, envoi: &Envoi<'_>) -> Vec<(&'static str, String)> {
    let sujet = match envoi.suffixe_sujet {
        Some(suffixe) => format!("{}{suffixe}", config.apns.bundle_id),
        None => config.apns.bundle_id.clone(),
    };

    let mut en_tetes = vec![
        ("apns-topic", sujet),
        ("apns-push-type", envoi.type_envoi.en_tete().to_string()),
        ("apns-priority", envoi.priorite.to_string()),
        ("apns-expiration", envoi.peremption.to_string()),
    ];
    if let Some(cle) = &envoi.collapse_id {
        en_tetes.push(("apns-collapse-id", cle.clone()));
    }
    en_tetes
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

        let en_tetes = en_tetes(config, &envoi);
        let sujet_journal = en_tetes
            .iter()
            .find(|(nom, _)| *nom == "apns-topic")
            .map(|(_, valeur)| valeur.clone())
            .unwrap_or_default();

        let mut requete = self
            .http
            .post(adresse(config, envoi.jeton_appareil))
            .header("authorization", format!("bearer {jeton}"));
        for (nom, valeur) in &en_tetes {
            requete = requete.header(*nom, valeur.as_str());
        }

        let reponse = requete.json(&envoi.charge).send().await;

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
                let resultat = Resultat {
                    ok: false,
                    statut,
                    raison,
                };
                if resultat.sujet_suspect() {
                    // En erreur, pas en avertissement : c'est une configuration
                    // à corriger, et rien ne partira tant qu'elle ne l'est pas.
                    tracing::error!(
                        statut,
                        sujet = %sujet_journal,
                        "APNs refuse le sujet : vérifiez APNS_BUNDLE_ID et le sujet \
                         de l'application. Les jetons sont conservés."
                    );
                } else {
                    tracing::warn!(
                        statut,
                        raison = ?resultat.raison,
                        "APNs a refusé l'envoi"
                    );
                }
                resultat
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
                if maintenant.saturating_sub(*emis_le) < DUREE_JETON_SECONDES {
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

    fn config(environnement: &str, bundle: &str) -> crate::env::Env {
        crate::env::Env {
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
                bundle_id: bundle.to_string(),
                environnement: environnement.to_string(),
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
        }
    }

    fn envoi<'a>(type_envoi: TypeEnvoi, suffixe: Option<&'a str>) -> Envoi<'a> {
        Envoi {
            jeton_appareil: "jeton",
            type_envoi,
            suffixe_sujet: suffixe,
            priorite: 5,
            collapse_id: Some("fil-1".to_string()),
            peremption: 1_789_500_000,
            charge: serde_json::json!({}),
        }
    }

    fn valeur<'a>(en_tetes: &'a [(&'static str, String)], nom: &str) -> Option<&'a str> {
        en_tetes
            .iter()
            .find(|(n, _)| *n == nom)
            .map(|(_, v)| v.as_str())
    }

    /// Les cinq en-têtes qui décident si une notification arrive.
    ///
    /// Aucun ne se voit dans un journal, et aucun n'était éprouvé : un sujet
    /// sans le suffixe d'ActivityKit, un type d'envoi qui ne correspond pas,
    /// et APNs refuse — ou accepte, puis ne livre rien.
    #[test]
    fn une_alerte_porte_les_en_tetes_qu_apple_attend() {
        let config = config("sandbox", "com.weave.app");
        let en_tetes = en_tetes(&config, &envoi(TypeEnvoi::Alerte, None));

        assert_eq!(valeur(&en_tetes, "apns-topic"), Some("com.weave.app"));
        assert_eq!(valeur(&en_tetes, "apns-push-type"), Some("alert"));
        assert_eq!(valeur(&en_tetes, "apns-priority"), Some("5"));
        assert_eq!(valeur(&en_tetes, "apns-collapse-id"), Some("fil-1"));
    }

    /// ActivityKit exige son suffixe de sujet et son propre type d'envoi.
    #[test]
    fn une_live_activity_porte_son_suffixe_de_sujet() {
        let config = config("sandbox", "com.weave.app");
        let en_tetes = en_tetes(
            &config,
            &envoi(TypeEnvoi::LiveActivity, Some(".push-type.liveactivity")),
        );

        assert_eq!(
            valeur(&en_tetes, "apns-topic"),
            Some("com.weave.app.push-type.liveactivity"),
        );
        assert_eq!(valeur(&en_tetes, "apns-push-type"), Some("liveactivity"));
    }

    /// Sans péremption, APNs ne garde rien.
    ///
    /// L'en-tête n'était pas envoyé du tout. La documentation d'Apple est
    /// explicite : « If the value is `0`, APNs attempts to deliver the
    /// notification only once and doesn't store it. » Une alerte partie vers
    /// un téléphone éteint était donc perdue sans reprise — « quelqu'un veut
    /// venir » compris, le seul message du produit qui attende une réponse.
    #[test]
    fn chaque_envoi_dit_jusqu_a_quand_le_garder() {
        let config = config("sandbox", "com.weave.app");
        for (type_envoi, suffixe) in [
            (TypeEnvoi::Alerte, None),
            (TypeEnvoi::LiveActivity, Some(".push-type.liveactivity")),
        ] {
            let en_tetes = en_tetes(&config, &envoi(type_envoi, suffixe));
            let date = valeur(&en_tetes, "apns-expiration")
                .expect("« apns-expiration » absent : APNs ne gardera rien")
                .parse::<i64>()
                .expect("une date en secondes UNIX");
            assert!(date > 0, "une péremption nulle vaut « ne garde rien »");
        }
    }

    /// Le bac à sable et la production ne se joignent pas à la même adresse.
    #[test]
    fn l_adresse_suit_l_environnement() {
        assert!(
            adresse(&config("production", "com.weave.app"), "jeton")
                .starts_with("https://api.push.apple.com/3/device/")
        );
        assert!(
            adresse(&config("sandbox", "com.weave.app"), "jeton")
                .starts_with("https://api.sandbox.push.apple.com/3/device/")
        );
    }

    fn refus(raison: &str) -> Resultat {
        Resultat {
            ok: false,
            statut: 400,
            raison: Some(raison.to_string()),
        }
    }

    /// Un jeton mort s'efface ; un sujet refusé, non.
    ///
    /// `DeviceTokenNotForTopic` figurait parmi les jetons morts. La raison ne
    /// dit pourtant pas lequel des deux est en cause — le jeton, ou notre
    /// `apns-topic` — et les deux lectures ne se paient pas pareil. Garder un
    /// jeton réellement mort coûte quelques envois dans le vide. Effacer sur
    /// une erreur de sujet vide **toutes** les inscriptions à la fois, sur une
    /// faute d'un caractère dans `APNS_BUNDLE_ID`, au lancement.
    #[test]
    fn un_sujet_refuse_ne_fait_pas_effacer_les_jetons() {
        for raison in ["BadDeviceToken", "Unregistered", "ExpiredToken"] {
            assert!(
                refus(raison).jeton_mort(),
                "« {raison} » devrait faire abandonner le jeton"
            );
        }

        let sujet = refus("DeviceTokenNotForTopic");
        assert!(
            !sujet.jeton_mort(),
            "un sujet refusé efface encore les jetons : une erreur de \
             configuration viderait toutes les inscriptions"
        );
        assert!(
            sujet.sujet_suspect(),
            "un sujet refusé passe inaperçu : personne n'ira vérifier la configuration"
        );

        // Un refus quelconque n'est ni l'un ni l'autre.
        let autre = refus("PayloadTooLarge");
        assert!(!autre.jeton_mort());
        assert!(!autre.sujet_suspect());
    }

    /// Sans configuration APNs, l'envoi est simulé plutôt que d'échouer. Une
    /// notification perdue ne doit jamais faire échouer la requête qui l'a
    /// déclenchée : accepter une demande reste utile même si l'écran verrouillé
    /// de l'autre ne s'allume pas.
    #[tokio::test]
    async fn sans_configuration_l_envoi_est_simule() {
        let config = config("sandbox", "com.weave.app");
        let resultat = ClientApns::new()
            .envoyer(
                &config,
                Envoi {
                    jeton_appareil: "jeton",
                    type_envoi: TypeEnvoi::Alerte,
                    suffixe_sujet: None,
                    priorite: 10,
                    collapse_id: None,
                    peremption: 0,
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
