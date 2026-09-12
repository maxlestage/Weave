//! Live Activity « Prochain plan » : le rendez-vous le plus proche sur l'écran
//! verrouillé et dans l'île dynamique, avec ce qui attend une réponse.
//!
//! L'état poussé est délibérément minuscule — un titre, une date, deux
//! compteurs. Aucun nom, aucune photo, aucun message ne transite par APNs : ce
//! qui s'affiche sur un écran verrouillé doit pouvoir être lu par quelqu'un
//! d'autre sans rien révéler de qui vous voyez.

use crate::{
    apns::{Envoi, TypeEnvoi},
    cache,
    entities::{devices, join_requests, live_activity_sessions, plans},
    error::AppError,
    temps::iso8601,
    AppState,
};
use chrono::{Duration, NaiveDateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Type d'attributs ActivityKit ; doit correspondre au nom Swift exact.
const TYPE_ATTRIBUTS: &str = "WeaveActivityAttributes";

/// Suffixe de sujet exigé par ActivityKit.
const SUFFIXE_SUJET: &str = ".push-type.liveactivity";

/// Durée de validité d'un état poussé avant que le système le marque périmé.
const PEREMPTION_SECONDES: i64 = 60 * 60;

/// Combien de temps le dernier état connu reste en cache.
const TTL_ETAT_SECONDES: u64 = 24 * 60 * 60;

/// Un plan reste « le prochain » un moment après son heure : on arrive en
/// retard, on ne veut pas voir la bannière disparaître pour autant.
const GRACE_PLAN_MINUTES: i64 = 30;

/// Combien de temps une Live Activity déclarée reste considérée en cours.
const ACTIVITE_MAX_HEURES: i64 = 8;

/// L'état affichable. Rien de plus ne doit s'y ajouter : chaque champ finit
/// sur un écran verrouillé.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Etat {
    pub plan_title: Option<String>,
    pub plan_starts_at: Option<String>,
    pub pending_requests: u64,
    pub awaiting_reply: u64,
    pub updated_at: String,
}

impl Etat {
    /// Vrai s'il n'y a plus rien à afficher : la Live Activity doit se terminer.
    fn vide(&self) -> bool {
        self.plan_title.is_none() && self.pending_requests == 0 && self.awaiting_reply == 0
    }

    /// Deux états sont équivalents si rien d'affichable n'a changé. `updatedAt`
    /// bouge à chaque calcul et ne compte donc pas : le comparer pousserait un
    /// envoi à chaque fois.
    fn meme_affichage(&self, autre: &Etat) -> bool {
        self.plan_title == autre.plan_title
            && self.plan_starts_at == autre.plan_starts_at
            && self.pending_requests == autre.pending_requests
            && self.awaiting_reply == autre.awaiting_reply
    }
}

/// Ce qu'il faut savoir pour composer un état.
struct Situation {
    prochain: Option<(String, NaiveDateTime, String)>,
    a_traiter: u64,
    sans_reponse: u64,
}

async fn situation_de(state: &AppState, compte_id: &str) -> Result<Situation, AppError> {
    let plancher = (Utc::now() - Duration::minutes(GRACE_PLAN_MINUTES)).naive_utc();
    // Un plan complet est toujours un rendez-vous : c'est même celui dont on a
    // le plus besoin sur l'écran verrouillé. Seuls « annule » et « passe »
    // sortent.
    let a_venir = ["ouvert", "complet"];

    // Mes propres plans à venir.
    let publie = plans::Entity::find()
        .filter(plans::Column::AuthorId.eq(compte_id))
        .filter(plans::Column::State.is_in(a_venir))
        .filter(plans::Column::StartsAt.gte(plancher))
        .order_by_asc(plans::Column::StartsAt)
        .one(&state.db)
        .await?
        .map(|p| (p.title, p.starts_at, p.city));

    // Les plans où ma demande a été acceptée.
    let mut rejoint = None;
    for demande in join_requests::Entity::find()
        .filter(join_requests::Column::AuthorId.eq(compte_id))
        .filter(join_requests::Column::State.eq("acceptee"))
        .all(&state.db)
        .await?
    {
        let Some(plan) = plans::Entity::find_by_id(demande.plan_id.as_str())
            .one(&state.db)
            .await?
        else {
            continue;
        };
        if !a_venir.contains(&plan.state.as_str()) || plan.starts_at < plancher {
            continue;
        }
        let candidat = (plan.title, plan.starts_at, plan.city);
        rejoint = match rejoint {
            Some((_, quand, _)) if quand <= candidat.1 => rejoint,
            _ => Some(candidat),
        };
    }

    // Le plus proche des deux.
    let prochain = match (publie, rejoint) {
        (Some(a), Some(b)) => Some(if a.1 <= b.1 { a } else { b }),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };

    // Demandes reçues sur mes plans, encore sans décision. Un plan complet n'en
    // a plus : elles sont closes au moment où la dernière place part.
    let mes_plans: Vec<String> = plans::Entity::find()
        .filter(plans::Column::AuthorId.eq(compte_id))
        .filter(plans::Column::State.eq("ouvert"))
        .all(&state.db)
        .await?
        .into_iter()
        .map(|p| p.id)
        .collect();

    let a_traiter = if mes_plans.is_empty() {
        0
    } else {
        join_requests::Entity::find()
            .filter(join_requests::Column::State.eq("envoyee"))
            .filter(join_requests::Column::PlanId.is_in(mes_plans))
            .count(&state.db)
            .await?
    };

    // Mes demandes envoyées, encore sans réponse.
    let sans_reponse = join_requests::Entity::find()
        .filter(join_requests::Column::AuthorId.eq(compte_id))
        .filter(join_requests::Column::State.eq("envoyee"))
        .count(&state.db)
        .await?;

    Ok(Situation {
        prochain,
        a_traiter,
        sans_reponse,
    })
}

fn etat_de(situation: &Situation) -> Etat {
    Etat {
        plan_title: situation.prochain.as_ref().map(|(t, ..)| t.clone()),
        plan_starts_at: situation
            .prochain
            .as_ref()
            .map(|(_, quand, _)| iso8601(quand.and_utc())),
        pending_requests: situation.a_traiter,
        awaiting_reply: situation.sans_reponse,
        updated_at: iso8601(Utc::now()),
    }
}

/// Résumé compact pour watchOS : quelques centaines d'octets, pas plus. Aucun
/// nom, aucune photo, aucun message.
fn resume_montre(situation: &Situation) -> Value {
    json!({
        "pendingRequests": situation.a_traiter,
        "awaitingReply": situation.sans_reponse,
        "nextPlan": situation.prochain.as_ref().map(|(titre, quand, ville)| json!({
            "title": titre,
            "startsAt": iso8601(quand.and_utc()),
            "city": ville,
        })),
        "generatedAt": iso8601(Utc::now()),
    })
}

/// Recalcule l'état et le pousse vers les Live Activities en cours.
///
/// L'envoi est ignoré si rien d'affichable n'a bougé : une Live Activity qui
/// clignote pour rien coûte de la batterie et de la confiance.
pub async fn publier(state: &AppState, compte_id: &str) -> Result<Etat, AppError> {
    let situation = situation_de(state, compte_id).await?;
    let etat = etat_de(&situation);

    let precedent: Option<Etat> =
        cache::lire_json(&state.cache, &cache::cles::live_activity(compte_id)).await;

    // Le cache n'est pas une source de vérité : une écriture qui échoue ne doit
    // pas faire échouer la requête qui a déclenché le calcul.
    if let Err(erreur) = cache::ecrire_json(
        &state.cache,
        &cache::cles::live_activity(compte_id),
        &etat,
        TTL_ETAT_SECONDES,
    )
    .await
    {
        tracing::warn!(erreur = %erreur, "état de Live Activity non mis en cache");
    }
    if let Err(erreur) = cache::ecrire_json(
        &state.cache,
        &cache::cles::montre(compte_id),
        &resume_montre(&situation),
        TTL_ETAT_SECONDES,
    )
    .await
    {
        tracing::warn!(erreur = %erreur, "résumé de montre non mis en cache");
    }

    if precedent.as_ref().is_some_and(|p| p.meme_affichage(&etat)) {
        return Ok(etat);
    }

    let sessions = live_activity_sessions::Entity::find()
        .filter(live_activity_sessions::Column::AccountId.eq(compte_id))
        .filter(live_activity_sessions::Column::EndedAt.is_null())
        .filter(live_activity_sessions::Column::StaleAt.gt(Utc::now().naive_utc()))
        .all(&state.db)
        .await?;

    let peremption = Utc::now().timestamp() + PEREMPTION_SECONDES;
    let termine = etat.vide();
    let charge = serde_json::to_value(&etat).unwrap_or_else(|_| json!({}));

    for session in sessions {
        let resultat = state
            .apns
            .envoyer(
                &state.config,
                Envoi {
                    jeton_appareil: &session.update_token,
                    type_envoi: TypeEnvoi::LiveActivity,
                    suffixe_sujet: Some(SUFFIXE_SUJET),
                    priorite: if termine {
                        10
                    } else if etat.pending_requests > 0 {
                        10
                    } else {
                        5
                    },
                    collapse_id: (!termine).then(|| format!("plan-{compte_id}")),
                    charge: json!({
                        "aps": {
                            "timestamp": Utc::now().timestamp(),
                            "event": if termine { "end" } else { "update" },
                            "content-state": charge,
                            "stale-date": peremption,
                        }
                    }),
                },
            )
            .await;

        // Un jeton qu'Apple déclare mort ne recevra jamais rien : la ligne se
        // ferme, plutôt que de repousser dessus à chaque changement.
        let ferme = resultat.jeton_mort() || termine;
        let mut maj: live_activity_sessions::ActiveModel = session.into();
        if !resultat.jeton_mort() {
            maj.last_state_json = Set(charge.to_string());
            maj.last_push_at = Set(Some(Utc::now().naive_utc()));
        }
        if ferme {
            maj.ended_at = Set(Some(Utc::now().naive_utc()));
        }
        maj.update(&state.db).await?;
    }

    Ok(etat)
}

/// Comme `publier`, mais sans jamais faire échouer l'appelant.
///
/// Les routes qui changent l'état d'une demande poussent la Live Activity au
/// passage ; une notification perdue ne doit pas annuler l'acceptation qui l'a
/// déclenchée.
pub async fn publier_au_mieux(state: &AppState, compte_id: &str) {
    if let Err(erreur) = publier(state, compte_id).await {
        tracing::warn!(erreur = %erreur, compte = compte_id, "Live Activity non publiée");
    }
}

/// Démarre à distance la Live Activity via le jeton « push-to-start »
/// d'ActivityKit. C'est ce qui permet à la bannière d'apparaître d'elle-même
/// quand quelqu'un demande à venir, sans que l'application ait été lancée.
pub async fn demarrer_pour(state: &AppState, compte_id: &str) -> Result<u32, AppError> {
    // L'état est recalculé d'abord, appareil ou non. Sortir avant, faute
    // d'iPhone enregistré, laissait le résumé de montre sur sa valeur
    // précédente : le poignet affichait « aucune demande » alors qu'une demande
    // venait d'arriver, et ce jusqu'à l'expiration du cache.
    let etat = publier(state, compte_id).await?;

    let appareils = devices::Entity::find()
        .filter(devices::Column::AccountId.eq(compte_id))
        .filter(devices::Column::Platform.eq("ios"))
        .filter(devices::Column::PushToStartToken.is_not_null())
        .all(&state.db)
        .await?;

    // Rien à afficher, ou personne à qui l'afficher : démarrer une bannière
    // vide serait une notification pour rien.
    if appareils.is_empty() || etat.vide() {
        return Ok(0);
    }

    let peremption = Utc::now().timestamp() + PEREMPTION_SECONDES;
    let charge = serde_json::to_value(&etat).unwrap_or_else(|_| json!({}));
    let alerte = alerte_de(&etat);
    let mut demarrees = 0;

    for appareil in appareils {
        let Some(jeton) = appareil.push_to_start_token.clone() else {
            continue;
        };

        let resultat = state
            .apns
            .envoyer(
                &state.config,
                Envoi {
                    jeton_appareil: &jeton,
                    type_envoi: TypeEnvoi::LiveActivity,
                    suffixe_sujet: Some(SUFFIXE_SUJET),
                    priorite: 10,
                    collapse_id: None,
                    charge: json!({
                        "aps": {
                            "timestamp": Utc::now().timestamp(),
                            "event": "start",
                            "content-state": charge,
                            "attributes-type": TYPE_ATTRIBUTS,
                            // L'identifiant du compte n'apparaît jamais en
                            // entier dans ce qui part chez Apple.
                            "attributes": { "accountHandle": &compte_id[..compte_id.len().min(8)] },
                            "stale-date": peremption,
                            "alert": { "title": alerte.0, "body": alerte.1 },
                        }
                    }),
                },
            )
            .await;

        if resultat.ok {
            demarrees += 1;
        } else if resultat.jeton_mort() {
            let mut maj: devices::ActiveModel = appareil.into();
            maj.push_to_start_token = Set(None);
            maj.update(&state.db).await?;
        }
    }

    tracing::info!(compte = compte_id, demarrees, "Live Activities démarrées");
    Ok(demarrees)
}

/// Une phrase qui dit ce qui se passe, sans nommer personne.
fn alerte_de(etat: &Etat) -> (String, String) {
    if etat.pending_requests > 0 {
        let n = etat.pending_requests;
        return (
            if n > 1 {
                format!("{n} personnes veulent venir")
            } else {
                "Quelqu'un veut venir".to_string()
            },
            etat.plan_title
                .clone()
                .unwrap_or_else(|| "Ouvrez Weave pour lire les messages.".to_string()),
        );
    }
    (
        "Votre prochain plan".to_string(),
        etat.plan_title
            .clone()
            .unwrap_or_else(|| "Ouvrez Weave.".to_string()),
    )
}

/// Combien de temps une Live Activity déclarée reste considérée en cours.
pub fn peremption_session() -> NaiveDateTime {
    (Utc::now() + Duration::hours(ACTIVITE_MAX_HEURES)).naive_utc()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn etat(titre: Option<&str>, a_traiter: u64, sans_reponse: u64) -> Etat {
        Etat {
            plan_title: titre.map(str::to_string),
            plan_starts_at: titre.map(|_| "2026-09-14T18:00:00.000Z".to_string()),
            pending_requests: a_traiter,
            awaiting_reply: sans_reponse,
            updated_at: iso8601(Utc::now()),
        }
    }

    /// `updatedAt` bouge à chaque calcul : le comparer pousserait un envoi à
    /// chaque fois, et la bannière clignoterait pour rien.
    #[test]
    fn l_horodatage_seul_ne_declenche_pas_d_envoi() {
        let a = etat(Some("Balade"), 1, 0);
        let mut b = a.clone();
        b.updated_at = "2099-01-01T00:00:00.000Z".to_string();
        assert!(a.meme_affichage(&b));
    }

    #[test]
    fn un_compteur_qui_bouge_declenche_un_envoi() {
        assert!(!etat(Some("Balade"), 1, 0).meme_affichage(&etat(Some("Balade"), 2, 0)));
        assert!(!etat(Some("Balade"), 1, 0).meme_affichage(&etat(Some("Repas"), 1, 0)));
    }

    #[test]
    fn un_etat_sans_rien_a_montrer_est_vide() {
        assert!(etat(None, 0, 0).vide());
        assert!(!etat(None, 1, 0).vide());
        assert!(!etat(Some("Balade"), 0, 0).vide());
    }

    /// Ce qui part chez Apple ne doit nommer personne.
    #[test]
    fn l_alerte_ne_nomme_personne() {
        let (titre, _) = alerte_de(&etat(Some("Balade"), 3, 0));
        assert_eq!(titre, "3 personnes veulent venir");
        let (titre, _) = alerte_de(&etat(Some("Balade"), 1, 0));
        assert_eq!(titre, "Quelqu'un veut venir");
    }
}
