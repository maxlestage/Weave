//! Les notifications d'alerte — celles qu'on reçoit quand l'application est
//! fermée et qu'aucune bannière n'est en cours.
//!
//! ## Pourquoi ce module existe
//!
//! La Live Activity porte déjà un signal : elle démarre d'elle-même quand
//! quelqu'un demande à venir, et son démarrage fait vibrer le téléphone. Mais
//! son état ne compte que des plans et des demandes — jamais des messages. Un
//! message arrivait donc sans que personne ne l'apprenne, sauf à ouvrir
//! l'application.
//!
//! La mécanique était pourtant là, entière et débranchée : `TypeEnvoi::Alerte`
//! écrit mais jamais construit, et `devices.apns_token` collecté à chaque
//! enregistrement d'appareil sans qu'aucune ligne ne le relise. Garder un
//! identifiant d'appareil pour un usage qui n'existe pas est exactement ce que
//! la politique de confidentialité s'interdit — il fallait soit s'en servir,
//! soit cesser de le demander.
//!
//! ## Ce qu'une alerte a le droit de dire
//!
//! Rien de la conversation. Ni le message, ni le nom de qui l'a écrit, ni même
//! son prénom. C'est la règle déjà tenue par la Live Activity, et elle vaut
//! d'autant plus ici : une notification s'affiche sur un écran verrouillé, que
//! n'importe qui peut lire par-dessus une épaule.
//!
//! Une alerte dit donc qu'il s'est passé quelque chose, et rien d'autre. C'est
//! suffisant : elle sert à faire ouvrir l'application, pas à la remplacer.

use crate::{
    apns::{Envoi, TypeEnvoi},
    entities::devices,
    AppState,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

/// Une alerte à pousser : ce qui s'affiche, et rien de plus.
struct Texte {
    titre: &'static str,
    corps: &'static str,
    /// Regroupe les alertes d'une même nature dans le centre de notifications.
    fil: &'static str,
}

const NOUVEAU_MESSAGE: Texte = Texte {
    titre: "Nouveau message",
    corps: "Ouvrez Weave pour le lire.",
    fil: "messages",
};

/// Prévient quelqu'un qu'un message l'attend.
///
/// N'échoue jamais : une notification perdue ne doit pas faire échouer l'envoi
/// du message qui l'a déclenchée. Le message est écrit, c'est ce qui compte ;
/// l'alerte est un confort.
pub async fn prevenir_message(state: &AppState, destinataire: &str) {
    if let Err(erreur) = pousser(state, destinataire, NOUVEAU_MESSAGE).await {
        tracing::warn!(erreur = %erreur, compte = destinataire, "alerte non poussée");
    }
}

async fn pousser(state: &AppState, compte_id: &str, texte: Texte) -> Result<u32, sea_orm::DbErr> {
    let appareils = devices::Entity::find()
        .filter(devices::Column::AccountId.eq(compte_id))
        .filter(devices::Column::Platform.eq("ios"))
        .filter(devices::Column::ApnsToken.is_not_null())
        .all(&state.db)
        .await?;

    let mut poussees = 0;

    for appareil in appareils {
        let Some(jeton) = appareil.apns_token.clone() else {
            continue;
        };

        let resultat = state
            .apns
            .envoyer(
                &state.config,
                Envoi {
                    jeton_appareil: &jeton,
                    type_envoi: TypeEnvoi::Alerte,
                    // Le suffixe de sujet est propre à ActivityKit : une alerte
                    // ordinaire s'adresse au paquet lui-même.
                    suffixe_sujet: None,
                    // Une alerte n'est pas urgente au sens d'Apple : la
                    // priorité 5 laisse le système la grouper et ménager la
                    // batterie. La 10 est réservée à ce qui doit arriver
                    // maintenant, et un message n'en est pas.
                    priorite: 5,
                    // Deux messages rapprochés ne doivent pas empiler deux
                    // bannières : la seconde remplace la première.
                    collapse_id: Some(format!("{}-{}", texte.fil, compte_id)),
                    charge: serde_json::json!({
                        "aps": {
                            "alert": { "title": texte.titre, "body": texte.corps },
                            "sound": "default",
                            "thread-id": texte.fil,
                        }
                    }),
                },
            )
            .await;

        if resultat.ok {
            poussees += 1;
        } else if resultat.jeton_mort() {
            // Apple dit que ce jeton ne vaut plus rien : le garder ferait
            // pousser dans le vide à chaque message, et c'est une donnée
            // d'appareil qu'on n'a plus de raison de conserver.
            tracing::info!(
                compte = compte_id,
                statut = resultat.statut,
                raison = resultat.raison.as_deref().unwrap_or("—"),
                "jeton d'alerte abandonné"
            );
            let mut maj: devices::ActiveModel = appareil.into();
            maj.apns_token = Set(None);
            maj.update(&state.db).await?;
        } else {
            tracing::warn!(
                compte = compte_id,
                statut = resultat.statut,
                raison = resultat.raison.as_deref().unwrap_or("—"),
                "alerte refusée par APNs"
            );
        }
    }

    Ok(poussees)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::Service;

    /// La règle de cette page : une alerte ne dit jamais qui écrit, ni quoi.
    ///
    /// Le test porte sur le texte constant plutôt que sur un envoi : c'est le
    /// texte qui pourrait dériver au fil des retouches, et c'est lui qui
    /// s'affiche sur un écran verrouillé.
    #[test]
    fn une_alerte_ne_nomme_personne_et_ne_cite_rien() {
        let assemble = format!("{} {}", NOUVEAU_MESSAGE.titre, NOUVEAU_MESSAGE.corps);
        for interdit in ["de ", "message :", "«", "\""] {
            assert!(
                !assemble.contains(interdit),
                "« {interdit} » laisse penser qu'un contenu ou un nom est cité"
            );
        }
        assert!(assemble.contains("Weave"), "l'alerte doit dire où aller");
    }

    /// Sans appareil enregistré, pousser ne doit rien tenter ni rien casser.
    #[tokio::test]
    async fn sans_appareil_il_n_y_a_rien_a_pousser() {
        let service = Service::monter().await;
        let compte = service.compte("solo", "depart").await;
        let etat = pousser(&service.etat, &compte, NOUVEAU_MESSAGE).await;
        assert_eq!(etat.expect("poussée"), 0);
    }

    /// Un appareil sans jeton d'alerte est ignoré : il n'y a rien à viser.
    ///
    /// La ligne est insérée directement plutôt que par la route : c'est le
    /// filtre de `pousser` qu'on éprouve, pas l'enregistrement d'un appareil.
    #[tokio::test]
    async fn un_appareil_sans_jeton_est_ignore() {
        use chrono::Utc;
        use sea_orm::Set;

        let service = Service::monter().await;
        let compte = service.compte("silencieux", "depart").await;

        devices::ActiveModel {
            id: Set(format!("d-{compte}")),
            account_id: Set(compte.clone()),
            platform: Set("ios".to_string()),
            vendor_id: Set(format!("v-{compte}")),
            model: Set(None),
            os_version: Set(None),
            app_version: Set(None),
            // Le point du test : pas de jeton d'alerte.
            apns_token: Set(None),
            push_to_start_token: Set(Some("pts".to_string())),
            apns_environment: Set("sandbox".to_string()),
            last_seen_at: Set(Utc::now().naive_utc()),
            created_at: Set(Utc::now().naive_utc()),
        }
        .insert(&service.db)
        .await
        .expect("appareil inséré");

        let poussees = pousser(&service.etat, &compte, NOUVEAU_MESSAGE)
            .await
            .expect("poussée");
        assert_eq!(poussees, 0, "un appareil sans jeton d'alerte ne reçoit rien");
    }
}
