//! Droits attachés au palier, et compteurs qui en dépendent.
//!
//! Règle de conception du catalogue : aucun palier n'achète de visibilité.
//! Payer ne fait jamais remonter un plan devant celui de quelqu'un d'autre —
//! ce qui se vend, c'est la finesse des critères, l'horizon de publication et
//! les plans de groupe.

use crate::{
    auth::CompteAuthentifie,
    cache,
    entities::credit_balances,
    error::AppError,
    temps::jour_local,
    AppState,
};
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::collections::BTreeMap;

/// Ce qu'un « Renfort » ajoute au quota du jour.
pub const RENFORT_GRANT: i64 = 5;

/// Les unités achetables à l'unité. L'ordre est celui des contrats partagés.
pub const UNIT_SKUS: [&str; 5] = ["renfort", "horizon", "tablee", "escale", "bilan"];

#[derive(Debug, Clone, Copy)]
pub struct Droits {
    /// Demandes envoyables par jour. Bornée à tous les paliers, par conception.
    pub demandes_par_jour: i64,
    /// Jusqu'à combien de jours à l'avance un plan peut être publié.
    pub jours_a_l_avance: i64,
    pub filtres: &'static str,
    pub plans_de_groupe: bool,
    pub escales_par_mois: i64,
    pub bilan: bool,
    pub assistance_prioritaire: bool,
}

/// Le palier inconnu retombe sur « départ » : jamais sur un palier payant.
pub fn droits_pour(palier: &str) -> Droits {
    match palier {
        "viree" => Droits { demandes_par_jour: 12, jours_a_l_avance: 14, filtres: "etendus", plans_de_groupe: true, escales_par_mois: 0, bilan: false, assistance_prioritaire: false },
        "escapade" => Droits { demandes_par_jour: 25, jours_a_l_avance: 30, filtres: "precis", plans_de_groupe: true, escales_par_mois: 1, bilan: false, assistance_prioritaire: false },
        "expedition" => Droits { demandes_par_jour: 40, jours_a_l_avance: 60, filtres: "precis", plans_de_groupe: true, escales_par_mois: 2, bilan: true, assistance_prioritaire: false },
        "grandtour" => Droits { demandes_par_jour: 60, jours_a_l_avance: 90, filtres: "precis", plans_de_groupe: true, escales_par_mois: 4, bilan: true, assistance_prioritaire: true },
        _ => Droits { demandes_par_jour: 5, jours_a_l_avance: 7, filtres: "base", plans_de_groupe: false, escales_par_mois: 0, bilan: false, assistance_prioritaire: false },
    }
}

/// Quota de demandes pour la journée en cours : celui du palier, augmenté des
/// « Renforts » déjà appliqués aujourd'hui.
///
/// Toutes les lectures du quota passent par ici. Calculer `demandes_par_jour`
/// seul quelque part afficherait un compteur faux à qui vient d'acheter un
/// renfort.
pub async fn quota_journalier(state: &AppState, compte: &CompteAuthentifie) -> i64 {
    let jour = jour_local(&compte.timezone, Utc::now());
    let renforts = cache::compteur(&state.cache, &cache::cles::renforts(&compte.id, &jour)).await;
    droits_pour(&compte.tier).demandes_par_jour + renforts * RENFORT_GRANT
}

/// Demandes restantes aujourd'hui. Ne descend jamais sous zéro.
pub async fn demandes_restantes(state: &AppState, compte: &CompteAuthentifie, quota: i64) -> i64 {
    let jour = jour_local(&compte.timezone, Utc::now());
    let utilisees =
        cache::compteur(&state.cache, &cache::cles::demandes_utilisees(&compte.id, &jour)).await;
    (quota - utilisees).max(0)
}

/// Soldes d'unités, tous les SKU présents même à zéro : l'application affiche
/// la liste entière, et une clé absente s'y lirait comme une erreur.
pub async fn credits_pour(
    state: &AppState,
    compte_id: &str,
) -> Result<BTreeMap<String, i64>, AppError> {
    let mut credits: BTreeMap<String, i64> =
        UNIT_SKUS.iter().map(|sku| (sku.to_string(), 0)).collect();

    let lignes = credit_balances::Entity::find()
        .filter(credit_balances::Column::AccountId.eq(compte_id))
        .all(&state.db)
        .await?;

    for ligne in lignes {
        // Un SKU retiré du catalogue peut subsister en base : on l'ignore
        // plutôt que d'exposer une unité qui n'existe plus.
        if credits.contains_key(&ligne.sku) {
            credits.insert(ligne.sku, ligne.balance as i64);
        }
    }
    Ok(credits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_palier_inconnu_retombe_sur_depart_jamais_sur_un_palier_payant() {
        assert_eq!(droits_pour("inconnu").demandes_par_jour, 5);
        assert_eq!(droits_pour("").demandes_par_jour, 5);
        assert!(!droits_pour("inconnu").plans_de_groupe);
    }

    #[test]
    fn les_droits_sont_ceux_du_catalogue_partage() {
        // Valeurs tirées de packages/contracts/src/catalog.ts.
        assert_eq!(droits_pour("depart").demandes_par_jour, 5);
        assert_eq!(droits_pour("viree").demandes_par_jour, 12);
        assert_eq!(droits_pour("escapade").demandes_par_jour, 25);
        assert_eq!(droits_pour("expedition").demandes_par_jour, 40);
        assert_eq!(droits_pour("grandtour").demandes_par_jour, 60);
        assert_eq!(droits_pour("grandtour").jours_a_l_avance, 90);
        assert_eq!(droits_pour("depart").jours_a_l_avance, 7);
    }

    #[test]
    fn aucun_palier_n_achete_de_visibilite() {
        // Le catalogue ne porte aucun droit de mise en avant : si l'un venait
        // à en gagner un, ce test n'aurait plus de sens et devrait être revu
        // en même temps que la règle.
        for palier in ["depart", "viree", "escapade", "expedition", "grandtour"] {
            let d = droits_pour(palier);
            assert!(d.demandes_par_jour > 0, "{palier} doit pouvoir demander");
            assert!(d.jours_a_l_avance > 0, "{palier} doit pouvoir publier");
        }
    }
}
