//! Limitation de débit adossée au magasin clé-valeur.
//!
//! Fenêtre fixe, compteur par sujet (compte authentifié, sinon adresse IP). Le
//! cache étant déjà une dépendance dure du produit, aucune bibliothèque
//! supplémentaire n'est nécessaire.

use crate::{cache, error::AppError, AppState};

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
    pub const DEMANDE_OTP: Regle = Regle { seau: "otp-request", limite: 5, fenetre_secondes: 15 * 60 };
    /// Vérification du code : ralentit une attaque par force brute.
    pub const VERIF_OTP: Regle = Regle { seau: "otp-verify", limite: 10, fenetre_secondes: 15 * 60 };
    /// Publication d'un plan. Le plafond de plans ouverts fait le vrai travail.
    pub const PUBLICATION: Regle = Regle { seau: "publish", limite: 20, fenetre_secondes: 60 * 60 };
    /// Demandes de participation. Le quota journalier du palier est
    /// l'invariant ; cette règle ne sert qu'à borner les rafales.
    pub const DEMANDE: Regle = Regle { seau: "join", limite: 40, fenetre_secondes: 60 * 60 };
}

pub struct Etat {
    pub restant: i64,
    #[allow(dead_code)]
    pub remise_a_zero_dans: i64,
}

/// Incrémente le compteur et refuse la requête si le seuil est franchi.
///
/// Un cache indisponible ne doit pas fermer le service : la limitation est une
/// protection, pas une condition de fonctionnement. On laisse alors passer, en
/// le signalant — refuser toutes les connexions parce que Redis hoquette
/// serait un remède pire que le mal.
pub async fn consommer(
    state: &AppState,
    regle: Regle,
    sujet: &str,
) -> Result<Etat, AppError> {
    let cle = cache::cles::limitation(regle.seau, sujet);
    let mut conn = state.cache.clone();

    let compte: i64 = match redis::cmd("INCR").arg(&cle).query_async(&mut conn).await {
        Ok(v) => v,
        Err(erreur) => {
            tracing::warn!(erreur = %erreur, seau = regle.seau, "limitation de débit indisponible");
            return Ok(Etat { restant: regle.limite, remise_a_zero_dans: regle.fenetre_secondes });
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
        return Err(crate::error::trop_de_requetes(&format!(
            "Limite atteinte pour « {} ». Réessayez dans {remise_a_zero_dans} s.",
            regle.seau
        )));
    }

    Ok(Etat {
        restant: (regle.limite - compte).max(0),
        remise_a_zero_dans,
    })
}
