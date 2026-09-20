//! Le rappel avant le rendez-vous.
//!
//! ## Pourquoi
//!
//! Un plan se publie des jours à l'avance, et rien ne le remettait en mémoire.
//! Or l'absence n'est presque jamais un renoncement : c'est un oubli. Et c'est
//! ce qui coûte le plus cher à un produit qui fait se rencontrer des gens,
//! parce que la personne restée seule au café, elle, ne revient pas.
//!
//! Le désistement, ajouté par ailleurs, traite celui qui SAIT qu'il ne viendra
//! pas. Le rappel traite celui qui l'a oublié. Les deux manquaient, et ils ne
//! se remplacent pas l'un l'autre.
//!
//! ## Pourquoi une marque en base plutôt qu'une fenêtre de temps
//!
//! La tentation est de chercher « les plans qui commencent dans deux heures,
//! à quinze minutes près ». Ce serait un rendez-vous à tenir : un dyno endormi
//! une heure manquerait la fenêtre, et le rappel ne partirait JAMAIS. Une
//! notification qui n'arrive pas est pire qu'une notification en retard.
//!
//! `plans.remindedAt` inverse la logique. On cherche les plans à venir qui
//! n'ont pas encore été rappelés, et on les marque. Un réveil tardif rattrape
//! donc au lieu de manquer, un réveil rapproché ne double pas, et deux dynos
//! qui balaient ensemble sont inoffensifs l'un pour l'autre — la marque est
//! posée par une écriture conditionnée, et seul celui qui l'a réellement posée
//! pousse quoi que ce soit.
//!
//! ## Qui reçoit
//!
//! L'auteur et les personnes acceptées : ceux qui ont un rendez-vous. Pas les
//! demandes encore en attente — on ne rappelle pas un rendez-vous qu'on n'a
//! pas. Et pas ceux qui ont coupé le réglage.

use crate::{
    AppState, alerte, cache,
    entities::{join_requests, plans, preferences},
};
use chrono::{Duration, Utc};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};

/// Combien de temps avant le rendez-vous le rappel part.
///
/// Deux heures : assez pour se remettre en route, changer de chemise ou
/// prévenir qu'on ne viendra pas — trop tard pour qu'on l'ait oublié à
/// nouveau. Un rappel la veille serait oublié comme le plan l'a été.
pub const AVANCE: Duration = Duration::hours(2);

/// Entre deux balayages.
const INTERVALLE: Duration = Duration::minutes(10);

/// Le verrou tient moins longtemps que l'intervalle ne l'espace : un dyno qui
/// meurt en plein balayage ne bloque pas le suivant plus d'un tour.
const VERROU_SECONDES: u64 = 540;

/// Fait tourner le rappel depuis le service, comme la purge.
pub fn planifier(state: AppState) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(INTERVALLE.to_std().unwrap_or_default()).await;
            passer_si_c_est_notre_tour(&state).await;
        }
    });
}

async fn passer_si_c_est_notre_tour(state: &AppState) {
    let cle = cache::cles::verrou("rappels");
    match cache::prendre_verrou(&state.cache, &cle, VERROU_SECONDES).await {
        Ok(false) => return,
        Err(erreur) => {
            // Sans verrou, on ne balaie pas : deux dynos pousseraient chacun
            // leur rappel du même plan. Le tour suivant est dans dix minutes,
            // et la marque en base fait qu'on ne perd rien à attendre.
            tracing::warn!(erreur = %erreur, "verrou de rappel indisponible, passage reporté");
            return;
        }
        Ok(true) => {}
    }

    match executer(state).await {
        Ok(bilan) => {
            if bilan.plans > 0 {
                tracing::info!(
                    plans = bilan.plans,
                    personnes = bilan.personnes,
                    "rappels envoyés"
                );
            }
        }
        Err(erreur) => tracing::error!(erreur = %erreur, "rappels en échec"),
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Bilan {
    pub plans: usize,
    pub personnes: usize,
}

/// Un passage : rappelle ce qui approche et n'a pas encore été rappelé.
pub async fn executer(state: &AppState) -> Result<Bilan, sea_orm::DbErr> {
    let maintenant = Utc::now().naive_utc();
    let horizon = maintenant + AVANCE;

    // Les plans qui approchent, jamais rappelés, et qui auront bien lieu.
    //
    // La borne basse est le présent : un plan déjà commencé ne se rappelle
    // pas. Sans elle, un plan passé inaperçu resterait éligible pour
    // toujours, et le rappel arriverait après le rendez-vous.
    let a_rappeler = plans::Entity::find()
        .filter(plans::Column::RemindedAt.is_null())
        .filter(plans::Column::StartsAt.gt(maintenant))
        .filter(plans::Column::StartsAt.lte(horizon))
        .filter(plans::Column::State.is_in(["ouvert", "complet"]))
        .limit(200)
        .all(&state.db)
        .await?;

    let mut bilan = Bilan::default();

    for plan in a_rappeler {
        // La marque d'abord, et conditionnée sur son absence.
        //
        // Poser la marque APRÈS avoir poussé laisserait deux balayages
        // concurrents pousser tous les deux. Ici, celui qui ne touche aucune
        // ligne sait qu'un autre s'en charge, et passe.
        //
        // Le prix de cet ordre est connu : si la poussée échoue ensuite, le
        // rappel est perdu plutôt que doublé. C'est le bon sens de l'erreur —
        // une notification manquée est un oubli de plus, une notification
        // doublée est une raison de couper les notifications.
        let marque = plans::Entity::update_many()
            .col_expr(
                plans::Column::RemindedAt,
                sea_orm::sea_query::Expr::value(maintenant),
            )
            .filter(plans::Column::Id.eq(plan.id.as_str()))
            .filter(plans::Column::RemindedAt.is_null())
            .exec(&state.db)
            .await?;
        if marque.rows_affected == 0 {
            continue;
        }

        bilan.plans += 1;
        for personne in convies(state, &plan).await? {
            if veut_etre_rappele(state, &personne).await? {
                alerte::prevenir_rendez_vous(state, &personne).await;
                bilan.personnes += 1;
            }
        }
    }

    Ok(bilan)
}

/// Ceux qui ont rendez-vous : l'auteur, et les personnes acceptées.
///
/// Pas les demandes en attente : on ne rappelle pas un rendez-vous qu'on n'a
/// pas encore. Recevoir « c'est bientôt » pour un plan dont on attend toujours
/// la réponse serait une fausse joie, et une de trop.
async fn convies(state: &AppState, plan: &plans::Model) -> Result<Vec<String>, sea_orm::DbErr> {
    let mut convies = vec![plan.author_id.clone()];
    convies.extend(
        join_requests::Entity::find()
            .filter(join_requests::Column::PlanId.eq(plan.id.as_str()))
            .filter(join_requests::Column::State.eq("acceptee"))
            .all(&state.db)
            .await?
            .into_iter()
            .map(|demande| demande.author_id),
    );
    Ok(convies)
}

/// Le réglage, et ce qu'on fait quand il n'y a pas de ligne de critères.
///
/// Absente, on rappelle : la colonne vaut `true` par défaut, et un compte sans
/// critères enregistrés n'a pas dit non. Lire l'absence comme un refus
/// priverait du rappel exactement ceux qui n'ont jamais ouvert les réglages.
async fn veut_etre_rappele(state: &AppState, compte_id: &str) -> Result<bool, sea_orm::DbErr> {
    Ok(preferences::Entity::find()
        .filter(preferences::Column::AccountId.eq(compte_id))
        .one(&state.db)
        .await?
        .is_none_or(|criteres| criteres.reminders_on))
}
