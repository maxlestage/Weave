//! Le « Bilan » : ce qui attire, ce qui tombe à plat.
//!
//! ## Ce qui manquait
//!
//! « Bilan » se vend 2,99 € dans le catalogue, avec sa description — « Un
//! retour ponctuel sur vos plans : ce qui attire, ce qui tombe à plat. » Le
//! crédit était accordé à l'achat et **rien ne le consommait**. Aucune route
//! ne le mentionnait, aucune ligne ne le lisait : on vendait un produit qui
//! n'existait pas.
//!
//! ## Ce que le bilan dit, et ce qu'il ne dit pas
//!
//! Tout sort des plans de la personne et des demandes qu'ils ont reçues. Rien
//! n'est inventé, rien n'est comparé aux autres : Weave ne classe pas les
//! gens, et un bilan qui dirait « vous recevez moins que la moyenne » ferait
//! exactement cela.
//!
//! Les messages des demandes ne sont pas lus. Leur nombre suffit à dire ce qui
//! attire ; leur contenu appartient à qui les a écrits.
//!
//! ## Le refus quand il n'y a rien à dire
//!
//! En dessous de quelques plans, un bilan ne distingue rien — il rendrait des
//! moyennes sur deux points. Il est alors refusé **avant** que le crédit ne
//! soit dépensé : faire payer 2,99 € pour un rapport vide serait pire que de
//! ne rien vendre du tout.

use crate::{
    auth::Authentifie,
    droits::exiger_credit,
    entities::{join_requests, plans},
    error::{invalide, AppError},
    temps::iso8601,
    AppState,
};
use axum::{extract::State, routing::post, Json, Router};
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// En dessous de ce nombre de plans passés, il n'y a rien à conclure.
const PLANS_MINIMUM: usize = 3;

/// Combien de plans le bilan cite nommément, dans un sens comme dans l'autre.
const CITES_MAX: usize = 3;

pub fn routes() -> Router<AppState> {
    Router::new().route("/v1/me/bilan", post(etablir))
}

/// Ce qu'on retient d'un plan pour le bilan.
struct Ligne {
    titre: String,
    categorie: String,
    demandes: usize,
    acceptees: usize,
    /// Jours entre la publication et le rendez-vous.
    delai_jours: i64,
}

async fn etablir(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
) -> Result<Json<Value>, AppError> {
    let maintenant = Utc::now();

    // Seuls les plans dont l'heure est passée entrent au bilan : un plan à
    // venir n'a pas fini de recevoir des demandes, et le compter maintenant
    // le ferait passer pour un échec.
    let passes = plans::Entity::find()
        .filter(plans::Column::AuthorId.eq(compte.id.as_str()))
        .filter(plans::Column::StartsAt.lt(maintenant.naive_utc()))
        .order_by_asc(plans::Column::StartsAt)
        .all(&state.db)
        .await?;

    if passes.len() < PLANS_MINIMUM {
        // Refusé AVANT de dépenser le crédit.
        return Err(invalide(&format!(
            "Il faut au moins {PLANS_MINIMUM} plans passés pour qu'un bilan dise quelque chose. \
             Vous en avez {}. Votre crédit n'a pas été utilisé.",
            passes.len()
        )));
    }

    let mut lignes = Vec::with_capacity(passes.len());
    for plan in &passes {
        let demandes = join_requests::Entity::find()
            .filter(join_requests::Column::PlanId.eq(plan.id.as_str()))
            .all(&state.db)
            .await?;

        lignes.push(Ligne {
            titre: plan.title.clone(),
            categorie: plan.category.clone(),
            demandes: demandes.len(),
            acceptees: demandes.iter().filter(|d| d.state == "acceptee").count(),
            delai_jours: (plan.starts_at - plan.created_at).num_days().max(0),
        });
    }

    // Le crédit se dépense une fois qu'on sait qu'il y a un bilan à rendre.
    exiger_credit(&state, &compte.id, "bilan", "Bilan").await?;

    let total_demandes: usize = lignes.iter().map(|l| l.demandes).sum();
    let total_acceptees: usize = lignes.iter().map(|l| l.acceptees).sum();
    let sans_echo: Vec<&Ligne> = lignes.iter().filter(|l| l.demandes == 0).collect();

    Ok(Json(json!({
        "etabliLe": iso8601(maintenant),
        "periode": {
            "duPremierPlan": iso8601(passes[0].starts_at.and_utc()),
            "auDernier": iso8601(passes[passes.len() - 1].starts_at.and_utc()),
        },
        "plansPasses": lignes.len(),
        "demandesRecues": total_demandes,
        "demandesAcceptees": total_acceptees,
        "plansSansAucuneDemande": sans_echo.len(),
        "parCategorie": par_categorie(&lignes),
        "cequiAttire": cites(&lignes, true),
        "ceQuiTombeAPlat": cites(&lignes, false),
        "delai": delai(&lignes),
    })))
}

/// Le rendement par catégorie, de la plus sollicitée à la moins.
fn par_categorie(lignes: &[Ligne]) -> Vec<Value> {
    let mut par: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for ligne in lignes {
        let entree = par.entry(ligne.categorie.as_str()).or_insert((0, 0));
        entree.0 += 1;
        entree.1 += ligne.demandes;
    }

    let mut rendu: Vec<(f64, Value)> = par
        .into_iter()
        .map(|(categorie, (plans, demandes))| {
            let moyenne = demandes as f64 / plans as f64;
            (
                moyenne,
                json!({
                    "categorie": categorie,
                    "plans": plans,
                    "demandes": demandes,
                    "demandesParPlan": arrondir(moyenne),
                }),
            )
        })
        .collect();

    // Décroissant. `total_cmp` plutôt que `partial_cmp` : un flottant ne se
    // compare pas partiellement ici, et on ne veut pas d'un `unwrap`.
    rendu.sort_by(|a, b| b.0.total_cmp(&a.0));
    rendu.into_iter().map(|(_, v)| v).collect()
}

/// Les plans les plus — ou les moins — sollicités, nommés.
fn cites(lignes: &[Ligne], meilleurs: bool) -> Vec<Value> {
    let mut triees: Vec<&Ligne> = lignes.iter().collect();
    if meilleurs {
        triees.sort_by(|a, b| b.demandes.cmp(&a.demandes));
    } else {
        triees.sort_by(|a, b| a.demandes.cmp(&b.demandes));
    }

    triees
        .into_iter()
        .take(CITES_MAX)
        .map(|l| {
            json!({
                "titre": l.titre,
                "categorie": l.categorie,
                "demandes": l.demandes,
                "publieJoursAvant": l.delai_jours,
            })
        })
        .collect()
}

/// Le délai de publication, comparé entre ce qui a pris et ce qui n'a rien eu.
///
/// C'est la seule vraie conclusion que ces données permettent : publier plus
/// tôt laisse plus de temps pour être vu. Rendue en deux moyennes plutôt qu'en
/// conseil — le chiffre se lit, un conseil s'impose.
fn delai(lignes: &[Ligne]) -> Value {
    let moyenne = |choisies: Vec<&Ligne>| -> Option<f64> {
        if choisies.is_empty() {
            return None;
        }
        let somme: i64 = choisies.iter().map(|l| l.delai_jours).sum();
        Some(somme as f64 / choisies.len() as f64)
    };

    let avec = moyenne(lignes.iter().filter(|l| l.demandes > 0).collect());
    let sans = moyenne(lignes.iter().filter(|l| l.demandes == 0).collect());

    json!({
        "joursAvantQuandCaPrend": avec.map(arrondir),
        "joursAvantQuandCaNePrendPas": sans.map(arrondir),
    })
}

/// Une décimale : deux chiffres après la virgule sur une moyenne de trois
/// plans laisseraient croire à une précision qui n'existe pas.
fn arrondir(valeur: f64) -> f64 {
    (valeur * 10.0).round() / 10.0
}
