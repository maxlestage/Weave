//! L'envoi des e-mails : le code de connexion, seul courrier du service.
//!
//! ## Ce qui manquait
//!
//! Le code était créé et haché, la réponse disait `sent: true`, l'application
//! affichait « un code vient de partir »… et rien ne partait. L'envoi « par le
//! fournisseur d'e-mail transactionnel » était écrit dans un commentaire, nulle
//! part ailleurs. Personne ne pouvait se connecter.
//!
//! ## Ce qui se passe sans configuration
//!
//! En production, la demande de code ÉCHOUE, et le dit : mieux vaut « le code
//! n'a pas pu être envoyé » qu'une attente devant une boîte vide. Hors
//! production, l'envoi est simulé — le code revient alors dans la réponse
//! (`devCode`), et les tests relisent ce qui serait parti.

use crate::{env::Env, messages::Msg};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, header::ContentType},
    transport::smtp::authentication::Credentials,
};
use std::time::Duration;

/// Pourquoi un courrier n'est pas parti.
#[derive(Debug)]
pub enum EchecEnvoi {
    /// Aucun serveur de courrier n'est configuré, en production.
    NonConfigure,
    /// L'adresse, l'expéditeur ou le serveur ont refusé.
    Refuse(String),
}

impl std::fmt::Display for EchecEnvoi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EchecEnvoi::NonConfigure => f.write_str("aucun serveur de courrier configuré"),
            EchecEnvoi::Refuse(raison) => write!(f, "refusé : {raison}"),
        }
    }
}

pub struct ClientCourriel {
    /// Le serveur et l'adresse d'expédition, s'ils sont configurés.
    relais: Option<(AsyncSmtpTransport<Tokio1Executor>, Mailbox)>,
    /// Ce qui aurait été envoyé, pour que les tests puissent le relire. Le
    /// champ n'existe qu'à la compilation des tests.
    #[cfg(test)]
    traces: std::sync::Arc<std::sync::Mutex<Vec<Trace>>>,
}

/// Un courrier simulé, tel qu'un test le relit.
#[cfg(test)]
#[derive(Clone, Debug)]
pub struct Trace {
    pub destinataire: String,
    pub sujet: String,
    pub corps: String,
}

impl ClientCourriel {
    pub fn new(config: &Env) -> Self {
        let relais = Self::relais(config).unwrap_or_else(|raison| {
            if config.courriel.configure {
                tracing::error!(raison = %raison, "serveur de courrier inutilisable");
            }
            None
        });
        Self {
            relais,
            #[cfg(test)]
            traces: Default::default(),
        }
    }

    fn relais(
        config: &Env,
    ) -> Result<Option<(AsyncSmtpTransport<Tokio1Executor>, Mailbox)>, String> {
        let courriel = &config.courriel;
        let (Some(utilisateur), Some(mot_de_passe)) =
            (&courriel.utilisateur, &courriel.mot_de_passe)
        else {
            return Ok(None);
        };
        let adresse = courriel.expediteur.as_deref().unwrap_or(utilisateur);
        let expediteur = format!("Weave <{adresse}>")
            .parse::<Mailbox>()
            .map_err(|erreur| format!("adresse d'expédition invalide : {erreur}"))?;

        // STARTTLS sur 587 : c'est ce qu'iCloud, comme la plupart des
        // fournisseurs, attend d'un client qui s'authentifie.
        let transport = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&courriel.hote)
            .map_err(|erreur| format!("serveur {} : {erreur}", courriel.hote))?
            .port(courriel.port)
            .credentials(Credentials::new(utilisateur.clone(), mot_de_passe.clone()))
            // Une demande de code ne doit pas pendre une minute : passé ce
            // délai, mieux vaut dire que l'envoi a échoué.
            .timeout(Some(Duration::from_secs(20)))
            .build();
        Ok(Some((transport, expediteur)))
    }

    /// Ce qui aurait été envoyé depuis le début du test, dans l'ordre.
    #[cfg(test)]
    pub fn traces(&self) -> Vec<Trace> {
        self.traces.lock().map(|t| t.clone()).unwrap_or_default()
    }

    /// Envoie le code de connexion, dans la langue de la requête en cours.
    pub async fn envoyer_code(
        &self,
        config: &Env,
        destinataire: &str,
        code: &str,
        minutes: i64,
    ) -> Result<(), EchecEnvoi> {
        let sujet = Msg::SujetDuCode {
            code: code.to_string(),
        }
        .t();
        let corps = Msg::CorpsDuCode {
            code: code.to_string(),
            minutes,
        }
        .t();

        let Some((transport, expediteur)) = &self.relais else {
            if config.is_production() {
                tracing::error!(
                    "Aucun serveur de courrier configuré (SMTP_USER, SMTP_PASSWORD) : \
                     les codes de connexion ne peuvent pas partir."
                );
                return Err(EchecEnvoi::NonConfigure);
            }
            #[cfg(test)]
            if let Ok(mut traces) = self.traces.lock() {
                traces.push(Trace {
                    destinataire: destinataire.to_string(),
                    sujet,
                    corps,
                });
            }
            return Ok(());
        };

        let courrier = Message::builder()
            .from(expediteur.clone())
            .to(destinataire
                .parse::<Mailbox>()
                .map_err(|erreur| EchecEnvoi::Refuse(erreur.to_string()))?)
            .subject(sujet)
            .header(ContentType::TEXT_PLAIN)
            .body(corps)
            .map_err(|erreur| EchecEnvoi::Refuse(erreur.to_string()))?;

        transport
            .send(courrier)
            .await
            .map(|_| ())
            .map_err(|erreur| EchecEnvoi::Refuse(erreur.to_string()))
    }
}
