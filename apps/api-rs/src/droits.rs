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
    error::{AppError, Code},
    temps::jour_local,
    AppState,
};
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::json;
use std::collections::BTreeMap;

/// Ce qu'un « Renfort » ajoute au quota du jour.
pub const RENFORT_GRANT: i64 = 5;

/// Les unités achetables à l'unité. L'ordre est celui des contrats partagés.
/// Jusqu'où un crédit « Horizon » ouvre la publication.
///
/// Il n'ouvrait RIEN DU TOUT — c'est-à-dire tout. La règle était « au-delà de
/// l'horizon du palier, dépense un crédit », sans borne supérieure : un compte
/// gratuit muni d'un seul crédit publiait un plan pour 2050.
///
/// Trois choses en découlaient. Le produit ne faisait pas ce qu'il annonce —
/// « jusqu'à soixante jours à l'avance » est écrit dans le catalogue, donc sur
/// la page des offres. Un crédit à l'unité donnait plus que l'abonnement le
/// plus cher, qui s'arrête à quatre-vingt-dix jours. Et un plan pouvait être
/// posé si loin que personne n'en verrait jamais l'échéance.
///
/// `packages/contracts` fait foi : `UNIT_PRODUCTS.horizon`.
pub const HORIZON_CREDIT_JOURS: i64 = 60;

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

/// Consomme un crédit, ou explique comment l'obtenir — par un palier ou à
/// l'unité. Le client n'a rien à deviner ni à coder en dur.
pub async fn exiger_credit(
    state: &AppState,
    compte_id: &str,
    sku: &str,
    nom: &str,
) -> Result<(), AppError> {
    exiger_credit_dans(&state.db, compte_id, sku, nom).await
}

/// Le même, sur une connexion quelconque — une transaction, notamment.
///
/// Dépenser un crédit hors de la transaction qui pose ce qu'il achète laisse
/// les deux se désynchroniser : le crédit part, l'écriture perd la course, et
/// personne ne rend rien. Ouvrir une escale est le cas où cela coûte le plus
/// cher — un double appui valait deux crédits pour une seule escale.
pub async fn exiger_credit_dans<C: sea_orm::ConnectionTrait>(
    db: &C,
    compte_id: &str,
    sku: &str,
    nom: &str,
) -> Result<(), AppError> {
    use sea_orm::sea_query::ExprTrait;

    // Décrément conditionnel : la clause `balance >= 1` est dans la requête,
    // sans quoi deux achats simultanés pourraient dépenser le même crédit.
    let resultat = credit_balances::Entity::update_many()
        .col_expr(
            credit_balances::Column::Balance,
            sea_orm::sea_query::Expr::col(credit_balances::Column::Balance).sub(1),
        )
        .filter(credit_balances::Column::AccountId.eq(compte_id))
        .filter(credit_balances::Column::Sku.eq(sku))
        .filter(credit_balances::Column::Balance.gte(1))
        .exec(db)
        .await?;

    if resultat.rows_affected == 1 {
        return Ok(());
    }

    Err(AppError::new(
        Code::EntitlementRequired,
        format!("« {nom} » n'est pas compris dans votre offre."),
    )
    .avec_details(json!({ "sku": sku })))
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

/// Ce que chaque profondeur de filtre autorise.
///
/// ## Pourquoi cette table existe
///
/// Le catalogue vend une profondeur de filtres par palier — « base »,
/// « étendus », « précis » — et le champ n'était lu nulle part : les critères
/// du fil acceptaient les mêmes réglages à tous les paliers. Quelqu'un payant
/// pour des « critères précis » avait exactement ce que le socle gratuit
/// offrait déjà.
///
/// ## Ce que le catalogue dit, et ce qu'il ne dit pas
///
/// « Escapade » nomme ce que « précis » recouvre : « catégorie, jour, distance
/// fine ». Les deux premiers sont des critères identifiables et sont traités
/// ici. La « distance fine » n'est définie nulle part — aucun palier ne dit
/// quelle granularité serait grossière —, elle n'est donc pas restreinte :
/// inventer une limite reviendrait à retirer quelque chose au nom d'une
/// promesse que personne n'a formulée.
///
/// « Étendus » n'est explicité par aucun palier. Le genre recherché lui est
/// attribué ici parce qu'il faut bien que « étendus » veuille dire quelque
/// chose entre « base » et « précis ».
///
/// ## Comment revenir en arrière
///
/// Toute la règle tient dans cette fonction. La rendre permissive pour tous —
/// `true` partout — rétablit le comportement d'avant sans toucher à rien
/// d'autre, et c'est un arbitrage commercial, pas une correction.
pub fn filtre_autorise(palier: &str, critere: Critere) -> bool {
    let profondeur = droits_pour(palier).filtres;
    match critere {
        // L'âge et la distance sont le socle : tout le monde y a droit.
        Critere::Age | Critere::Distance => true,
        Critere::Genre => matches!(profondeur, "etendus" | "precis"),
        Critere::Categorie | Critere::Jour => profondeur == "precis",
    }
}

/// Les rayons disponibles sans « critères précis ».
/// `packages/contracts` fait foi : `RADIUS_STEPS_KM`.
pub const CRANS_RAYON_KM: [i32; 4] = [10, 25, 50, 100];

/// Le rayon effectivement appliqué au fil, selon le palier.
///
/// Le catalogue vend « Critères précis : catégorie, jour, DISTANCE FINE » à
/// partir de l'Escapade. La catégorie et le jour se limitaient bien par
/// palier ; la distance, elle, se réglait au kilomètre près partout — y
/// compris au palier gratuit. « Distance fine » était vendue trois fois sans
/// rien désigner qui n'existât déjà.
///
/// Le réglage choisi n'est jamais écrasé en base : il est seulement rabattu à
/// la lecture, comme les autres critères vendus. Un abonnement qui s'interrompt
/// ne fait donc pas perdre ce qu'on avait réglé, et le rayon exact revient dès
/// que l'offre le permet.
///
/// Le cran le plus proche, et jamais moins que le plus petit : rabattre vers le
/// bas viderait le fil de quelqu'un qui n'a rien demandé.
pub fn rayon_effectif(palier: &str, km: i32) -> i32 {
    if droits_pour(palier).filtres == "precis" {
        return km;
    }
    CRANS_RAYON_KM
        .into_iter()
        .min_by_key(|cran| (cran - km).abs())
        .unwrap_or(km)
}

/// Les critères du fil, pour la table ci-dessus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Critere {
    Age,
    Distance,
    Genre,
    Categorie,
    Jour,
}

impl Critere {
    /// Le nom qu'on rend à qui se voit refuser le critère.
    pub fn nom(self) -> &'static str {
        match self {
            Critere::Age => "l'âge",
            Critere::Distance => "la distance",
            Critere::Genre => "le genre recherché",
            Critere::Categorie => "la catégorie",
            Critere::Jour => "le jour",
        }
    }
}

#[cfg(test)]
mod tests_filtres {
    use super::*;

    /// Le socle gratuit garde l'âge et la distance.
    ///
    /// Restreindre ces deux-là viderait le fil de tout réglage utile et ferait
    /// du palier gratuit une démonstration plutôt qu'un service.
    #[test]
    fn le_socle_garde_l_age_et_la_distance() {
        assert!(filtre_autorise("depart", Critere::Age));
        assert!(filtre_autorise("depart", Critere::Distance));
    }

    /// Et il n'a pas ce que les paliers payants vendent.
    #[test]
    fn le_socle_n_a_pas_les_criteres_vendus() {
        assert!(!filtre_autorise("depart", Critere::Genre));
        assert!(!filtre_autorise("depart", Critere::Categorie));
        assert!(!filtre_autorise("depart", Critere::Jour));
    }

    /// « Escapade » nomme catégorie et jour : elle doit les avoir.
    #[test]
    fn escapade_a_ce_que_son_argumentaire_nomme() {
        for critere in [Critere::Genre, Critere::Categorie, Critere::Jour] {
            assert!(
                filtre_autorise("escapade", critere),
                "« Escapade » vend « critères précis : catégorie, jour » : {critere:?}"
            );
        }
    }

    /// Un palier inconnu retombe sur le socle, jamais sur le plus généreux.
    #[test]
    fn un_palier_inconnu_retombe_sur_le_socle() {
        assert!(!filtre_autorise("inconnu", Critere::Categorie));
        assert!(filtre_autorise("inconnu", Critere::Age));
    }
}
