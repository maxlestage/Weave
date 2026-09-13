//! Limitation de débit adossée au magasin clé-valeur.
//!
//! Fenêtre fixe, compteur par sujet (compte authentifié, sinon adresse IP). Le
//! cache étant déjà une dépendance dure du produit, aucune bibliothèque
//! supplémentaire n'est nécessaire.

use crate::{AppState, cache, error::AppError};

#[derive(Debug, Clone, Copy)]
pub struct Regle {
    /// Nom du seau : sert de préfixe de clé et de libellé de journalisation.
    pub seau: &'static str,
    /// Nombre d'actions autorisées par fenêtre.
    pub limite: i64,
    /// Durée de la fenêtre, en secondes.
    pub fenetre_secondes: i64,
}

/// Règles appliquées par le service.
pub mod regles {
    use super::Regle;

    /// Demande de code de connexion : protège la boîte mail et le coût d'envoi.
    pub const DEMANDE_OTP: Regle = Regle {
        seau: "otp-request",
        limite: 5,
        fenetre_secondes: 15 * 60,
    };
    /// Vérification du code : ralentit une attaque par force brute.
    pub const VERIF_OTP: Regle = Regle {
        seau: "otp-verify",
        limite: 10,
        fenetre_secondes: 15 * 60,
    };
    /// Publication d'un plan. Le plafond de plans ouverts fait le vrai travail.
    pub const PUBLICATION: Regle = Regle {
        seau: "publish",
        limite: 20,
        fenetre_secondes: 60 * 60,
    };
    /// Demandes de participation. Le quota journalier du palier est
    /// l'invariant ; cette règle ne sert qu'à borner les rafales.
    pub const DEMANDE: Regle = Regle {
        seau: "join",
        limite: 40,
        fenetre_secondes: 60 * 60,
    };

    /// Envoi de photo de profil.
    ///
    /// Chaque envoi est une transaction qui écrit jusqu'à deux mégaoctets et en
    /// efface autant : l'ancienne part avec la nouvelle, si bien que rien ne
    /// s'accumule — mais rien ne bornait non plus le rythme. Dix par heure
    /// laisse largement de quoi hésiter entre trois photos.
    pub const PHOTO: Regle = Regle {
        seau: "photo",
        limite: 10,
        fenetre_secondes: 60 * 60,
    };

    /// Export de ses données.
    ///
    /// C'est la lecture la plus lourde du service : tout le compte, plans,
    /// demandes, conversations, messages, achats, et les octets de la photo.
    /// Rien ne la bornait, et elle est accessible à tout compte connecté.
    ///
    /// La borne est volontairement large. Le droit d'accès ne se refuse pas :
    /// l'article 12 ne permet de s'opposer qu'aux demandes « manifestement
    /// infondées ou excessives, notamment en raison de leur caractère
    /// répétitif ». Cinq par jour ne gêne personne qui exerce son droit, et
    /// arrête une boucle.
    pub const EXPORT: Regle = Regle {
        seau: "export",
        limite: 5,
        fenetre_secondes: 24 * 60 * 60,
    };
}

/// Incrémente le compteur et refuse la requête si le seuil est franchi.
///
/// Un cache indisponible ne doit pas fermer le service : la limitation est une
/// protection, pas une condition de fonctionnement. On laisse alors passer, en
/// le signalant — refuser toutes les connexions parce que Redis hoquette
/// serait un remède pire que le mal.
pub async fn consommer(state: &AppState, regle: Regle, sujet: &str) -> Result<(), AppError> {
    let cle = cache::cles::limitation(regle.seau, sujet);
    let mut conn = state.cache.clone();

    let compte: i64 = match redis::cmd("INCR").arg(&cle).query_async(&mut conn).await {
        Ok(v) => v,
        Err(erreur) => {
            tracing::warn!(erreur = %erreur, seau = regle.seau, "limitation de débit indisponible");
            return Ok(());
        }
    };

    if compte == 1 {
        let _ = redis::cmd("EXPIRE")
            .arg(&cle)
            .arg(regle.fenetre_secondes)
            .query_async::<i64>(&mut conn)
            .await;
    }

    let ttl: i64 = redis::cmd("TTL")
        .arg(&cle)
        .query_async(&mut conn)
        .await
        .unwrap_or(regle.fenetre_secondes);
    let remise_a_zero_dans = if ttl > 0 { ttl } else { regle.fenetre_secondes };

    if compte > regle.limite {
        return Err(
            crate::error::trop_de_requetes(crate::messages::Msg::LimiteAtteinte {
                quoi: regle.seau.to_string(),
                secondes: remise_a_zero_dans,
            })
            .dans(remise_a_zero_dans),
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::Service;

    /// Un refus de débit doit dire QUAND réessayer, et pas seulement en
    /// français : le message est pour la personne, `Retry-After` est pour le
    /// programme qui doit temporiser au lieu de réessayer aussitôt — et de se
    /// faire refuser encore.
    #[tokio::test]
    async fn un_refus_porte_le_delai_avant_de_reessayer() {
        const SERRE: Regle = Regle {
            seau: "test-serre",
            limite: 1,
            fenetre_secondes: 30,
        };

        let service = Service::monter().await;
        let sujet = service.id("limite");

        consommer(&service.etat, SERRE, &sujet)
            .await
            .expect("le premier passage est accepté");

        let refus = consommer(&service.etat, SERRE, &sujet)
            .await
            .expect_err("le second doit être refusé");

        assert_eq!(refus.code, crate::error::Code::RateLimited);
        let delai = refus
            .retry_after
            .expect("le délai doit accompagner le refus");
        assert!(
            (1..=SERRE.fenetre_secondes).contains(&delai),
            "délai hors de la fenêtre : {delai} s"
        );
        assert!(
            refus.message.contains("Réessayez"),
            "le message reste lisible par une personne"
        );
    }
}
