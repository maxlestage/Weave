//! La purge : ce que les dates de rétention promettaient sans que rien ne
//! l'exécute.
//!
//! Jusqu'ici, supprimer son compte posait `deletion_requested_at` et clore une
//! conversation posait `purge_after` — puis les lignes restaient en base,
//! indéfiniment. Les deux colonnes décrivaient une intention, pas un effet.
//!
//! ## Deux façons de la déclencher
//!
//! `weave-api purge` est un processus séparé, comme `migrate` : il ne démarre
//! ni serveur ni cache, se termine, et se planifie (Heroku Scheduler, cron).
//! C'est la forme préférable — une tâche d'entretien n'a rien à faire dans le
//! processus qui sert les requêtes.
//!
//! Mais elle demande une intervention d'exploitation, et sans elle la purge
//! n'avait tout simplement pas lieu. `planifier` la fait donc tourner depuis le
//! service lui-même, et répond aux deux objections qu'on lui oppose : un verrou
//! pris dans le cache empêche deux dynos de purger en même temps, et une
//! tentative par heure plutôt qu'un horaire fixe fait qu'un dyno endormi ne
//! manque rien — il n'a pas de rendez-vous à tenir, seulement un verrou à
//! prendre dès qu'il se réveille. Le détail est sur `planifier`.
//!
//! ## Ce que la cascade fait à notre place
//!
//! Toutes les tables liées à `accounts` sont déclarées `ON DELETE CASCADE` :
//! supprimer la ligne du compte emporte fiche, critères, plans, demandes,
//! conversations, messages, appareils, abonnements, achats, consentements.
//! Seul `audit_events` est en `SET NULL` — la trace de l'action survit, son
//! auteur devient anonyme. C'est exactement ce qu'on veut d'un journal.
//!
//! ## L'exception, et sa raison
//!
//! Un compte visé par un signalement non traité n'est **pas** purgé : sa
//! suppression emporterait le dossier en cours d'instruction, et il suffirait
//! alors de supprimer son compte pour effacer les preuves d'un comportement
//! qu'on vient de signaler. Le compte reste hors circulation — statut
//! `deleting`, invisible du fil, sans session valide — et sera purgé lors d'un
//! passage ultérieur, une fois le signalement clos.

use crate::{
    cache,
    entities::{accounts, live_activity_sessions, messages, reports},
    AppState,
};
use chrono::{Duration, Utc};
use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect,
};

/// Délai entre la demande de suppression et l'effacement réel.
/// `packages/contracts` fait foi : `ACCOUNT_PURGE_DAYS`.
pub const PURGE_COMPTE_JOURS: i64 = 30;

/// Ce qu'un passage a fait, pour le journal et pour les tests.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Bilan {
    pub messages_effaces: u64,
    pub comptes_effaces: u64,
    /// Comptes échus mais retenus par un signalement encore ouvert.
    pub comptes_differes: u64,
    pub activites_effacees: u64,
}

/// Combien de temps le verrou quotidien reste tenu.
///
/// Vingt-trois heures, pas vingt-quatre : sinon deux passages consécutifs
/// dériveraient d'un jour à l'autre et finiraient par sauter une journée.
const VERROU_SECONDES: u64 = 23 * 60 * 60;

/// Entre deux tentatives de prise du verrou.
const INTERVALLE: Duration = Duration::hours(1);

/// Délai avant la première tentative, après le démarrage.
///
/// Le service doit d'abord être en mesure de répondre : une purge lancée dans
/// la même seconde disputerait ses connexions à la base aux premières requêtes.
const DELAI_INITIAL: Duration = Duration::minutes(2);

/// Fait tourner la purge depuis le service lui-même.
///
/// Un planificateur externe — Heroku Scheduler, cron — reste préférable : une
/// tâche d'entretien n'a rien à faire dans le processus qui sert les requêtes.
/// Mais il demande une intervention d'exploitation, et sans elle la purge
/// n'avait tout simplement pas lieu : `deletionRequestedAt` et `purgeAfter`
/// décrivaient une intention que rien n'exécutait.
///
/// Deux précautions rendent la chose sûre :
///
/// * **un verrou dans le cache**, pris en une seule commande. Deux dynos qui
///   se réveillent ensemble ne purgent pas deux fois — le second voit le
///   verrou et repasse son tour.
/// * **une tentative par heure**, pas une purge par heure. Le verrou tient
///   vingt-trois heures : au plus un passage par jour, quel que soit le nombre
///   de dynos et quelle que soit l'heure de leur réveil.
///
/// C'est ce dernier point qui rend l'approche acceptable sur un dyno qui
/// s'endort : il n'y a pas d'horaire à manquer, seulement un verrou à prendre
/// dès qu'on est réveillé.
pub fn planifier(state: AppState) {
    tokio::spawn(async move {
        tokio::time::sleep(DELAI_INITIAL.to_std().unwrap_or_default()).await;
        loop {
            passer_si_c_est_notre_tour(&state).await;
            tokio::time::sleep(INTERVALLE.to_std().unwrap_or_default()).await;
        }
    });
}

async fn passer_si_c_est_notre_tour(state: &AppState) {
    let cle = cache::cles::verrou("purge");
    match cache::prendre_verrou(&state.cache, &cle, VERROU_SECONDES).await {
        Ok(false) => return,
        Err(erreur) => {
            // Cache indisponible : on ne purge pas. Purger sans verrou serait
            // le seul moyen de purger deux fois, et rien ne presse — la
            // tentative suivante aura lieu dans une heure.
            tracing::warn!(erreur = %erreur, "verrou de purge indisponible, passage reporté");
            return;
        }
        Ok(true) => {}
    }

    match executer(&state.db).await {
        Ok(bilan) => tracing::info!(
            messages = bilan.messages_effaces,
            activites = bilan.activites_effacees,
            comptes = bilan.comptes_effaces,
            differes = bilan.comptes_differes,
            "purge effectuée"
        ),
        Err(erreur) => tracing::error!(erreur = %erreur, "purge en échec"),
    }
}

pub async fn executer(db: &DatabaseConnection) -> Result<Bilan, DbErr> {
    let maintenant = Utc::now().naive_utc();

    // 1. Les messages dont la conversation est close depuis assez longtemps.
    //    `purge_after` est posé à la clôture, jamais à l'envoi : un échange
    //    ouvert n'expire pas.
    let messages_effaces = messages::Entity::delete_many()
        .filter(messages::Column::PurgeAfter.is_not_null())
        .filter(messages::Column::PurgeAfter.lte(maintenant))
        .exec(db)
        .await?
        .rows_affected;

    // 2. Les activités en direct périmées.
    //
    //    L'envoi les ignore déjà — il ne retient que celles dont `stale_at`
    //    est à venir — mais leurs lignes restaient là, chacune portant un
    //    `last_state_json` : l'instantané de ce qui s'est affiché sur un écran
    //    verrouillé. Le garder après la fin de l'activité ne sert plus rien,
    //    et la minimisation qu'annonce la politique de confidentialité vaut
    //    aussi pour ce qu'on a cessé d'utiliser.
    let activites_effacees = live_activity_sessions::Entity::delete_many()
        .filter(live_activity_sessions::Column::StaleAt.lte(maintenant))
        .exec(db)
        .await?
        .rows_affected;

    // 3. Les comptes dont le délai est écoulé.
    let echeance = maintenant - Duration::days(PURGE_COMPTE_JOURS);
    let echus: Vec<String> = accounts::Entity::find()
        .filter(accounts::Column::DeletionRequestedAt.is_not_null())
        .filter(accounts::Column::DeletionRequestedAt.lte(echeance))
        .select_only()
        .column(accounts::Column::Id)
        .into_tuple()
        .all(db)
        .await?;

    let mut bilan = Bilan { messages_effaces, activites_effacees, ..Default::default() };

    for id in echus {
        if signalement_en_cours(db, &id).await? {
            bilan.comptes_differes += 1;
            tracing::info!(
                compte = %id,
                "purge différée : un signalement visant ce compte est encore ouvert"
            );
            continue;
        }

        let efface = accounts::Entity::delete_by_id(&id).exec(db).await?.rows_affected;
        bilan.comptes_effaces += efface;
    }

    Ok(bilan)
}

/// Combien de temps un signalement ouvert peut retenir une suppression.
///
/// ## Pourquoi cette borne est indispensable
///
/// `handled_at` est la marque de clôture, et **rien ne l'écrit nulle part** :
/// aucune route, aucun outil ne clôt un dossier. Un signalement restait donc
/// ouvert pour toujours, et la purge du compte visé était différée pour
/// toujours avec lui.
///
/// Deux conséquences, l'une légale et l'autre pire :
///
///   * la page publique promet que le compte « est effacé dès le dossier
///     clos ». Un dossier qui ne peut pas se clore fait de cette exception au
///     droit à l'effacement une exemption permanente — ce que l'article 17
///     n'autorise pas ;
///   * n'importe qui pouvait **empêcher définitivement** la suppression du
///     compte d'autrui : un seul signalement suffisait, et rien ne pouvait le
///     lever. On retourne contre quelqu'un l'outil censé le protéger.
///
/// Quatre-vingt-dix jours : la durée déjà retenue pour les messages d'une
/// conversation close. Une instruction qui n'a pas eu lieu en trois mois ne
/// justifie plus de conserver les données de quelqu'un qui a demandé leur
/// effacement.
const SIGNALEMENT_DIFFERE_JOURS: i64 = 90;

/// Un signalement visant ce compte retient-il encore sa suppression ?
///
/// Ouvert ET récent. `handled_at` reste la marque de clôture — un dossier
/// instruit ne retient plus rien —, mais elle ne suffit pas : faute de
/// quiconque pour l'écrire, elle laisserait la suppression en suspens
/// indéfiniment.
///
/// Le signalement n'est pas marqué comme traité par la borne : il ne l'a pas
/// été, et l'écrire serait consigner une instruction qui n'a pas eu lieu. Il
/// cesse simplement de justifier une rétention.
async fn signalement_en_cours(db: &DatabaseConnection, compte: &str) -> Result<bool, DbErr> {
    let limite = Utc::now().naive_utc() - Duration::days(SIGNALEMENT_DIFFERE_JOURS);
    let ouverts = reports::Entity::find()
        .filter(reports::Column::TargetId.eq(compte))
        .filter(reports::Column::HandledAt.is_null())
        .filter(reports::Column::CreatedAt.gt(limite))
        .count(db)
        .await?;
    Ok(ouverts > 0)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::tests::base_de_test;
    use sea_orm::{ActiveModelTrait, Set};

    pub(super) async fn compte(db: &DatabaseConnection, id: &str, supprime_il_y_a: Option<i64>) {
        accounts::ActiveModel {
            id: Set(id.to_string()),
            email: Set(format!("{id}@exemple.test")),
            email_hash: Set(format!("h-{id}")),
            handle: Set(id.to_string()),
            display_name: Set(id.to_string()),
            birth_date: Set(
                chrono::NaiveDate::from_ymd_opt(1995, 1, 1)
                    .expect("date valide")
                    .and_hms_opt(0, 0, 0)
                    .expect("heure valide"),
            ),
            status: Set(if supprime_il_y_a.is_some() { "deleting" } else { "active" }.to_string()),
            timezone: Set("Europe/Paris".to_string()),
            locale: Set("fr".to_string()),
            verified: Set(true),
            last_seen_at: Set(None),
            last_bilan_at: Set(None),
            deletion_requested_at: Set(
                supprime_il_y_a.map(|j| Utc::now().naive_utc() - Duration::days(j))
            ),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
        }
        .insert(db)
        .await
        .expect("compte inséré");
    }

    #[tokio::test]
    async fn un_compte_echu_est_efface_un_compte_actif_ne_l_est_pas() {
        let db = base_de_test().await;
        compte(&db, "echu", Some(PURGE_COMPTE_JOURS + 1)).await;
        compte(&db, "recent", Some(1)).await;
        compte(&db, "actif", None).await;

        let bilan = executer(&db).await.expect("purge");
        assert_eq!(bilan.comptes_effaces, 1, "seul le compte échu part");

        let restants = accounts::Entity::find().all(&db).await.expect("lecture");
        let ids: Vec<&str> = restants.iter().map(|c| c.id.as_str()).collect();
        assert!(!ids.contains(&"echu"));
        assert!(ids.contains(&"recent"), "le délai de trente jours n'est pas écoulé");
        assert!(ids.contains(&"actif"), "un compte vivant n'est jamais touché");
    }

    /// La règle qui protège la modération : supprimer son compte ne doit pas
    /// suffire à effacer le dossier qu'on vient d'ouvrir contre soi.
    /// Un signalement jamais instruit cesse de retenir une suppression.
    ///
    /// `handled_at` est la marque de clôture, et rien ne l'écrit nulle part :
    /// aucune route, aucun outil ne clôt un dossier. Sans borne, la purge
    /// était différée pour toujours — et n'importe qui pouvait empêcher
    /// définitivement la suppression du compte d'autrui en le signalant une
    /// fois.
    #[tokio::test]
    async fn un_signalement_jamais_instruit_cesse_de_retenir_la_suppression() {
        let db = base_de_test().await;
        compte(&db, "vise_vieux", Some(PURGE_COMPTE_JOURS + 5)).await;
        compte(&db, "plaignant_vieux", None).await;

        reports::ActiveModel {
            id: Set("r_vieux".to_string()),
            author_id: Set("plaignant_vieux".to_string()),
            target_id: Set("vise_vieux".to_string()),
            reason: Set("harcelement".to_string()),
            details: Set(String::new()),
            state: Set("ouvert".to_string()),
            // Jamais instruit — et déposé il y a plus longtemps que la borne.
            handled_at: Set(None),
            created_at: Set(
                Utc::now().naive_utc() - Duration::days(SIGNALEMENT_DIFFERE_JOURS + 1),
            ),
        }
        .insert(&db)
        .await
        .expect("signalement inséré");

        let bilan = executer(&db).await.expect("purge");
        assert_eq!(bilan.comptes_differes, 0, "le signalement retient encore");
        assert_eq!(bilan.comptes_effaces, 1);
        assert!(
            accounts::Entity::find_by_id("vise_vieux")
                .one(&db)
                .await
                .expect("lecture")
                .is_none(),
            "une instruction qui n'a pas eu lieu en trois mois ne justifie plus \
             de conserver les données de quelqu'un qui a demandé leur effacement"
        );
    }

    #[tokio::test]
    async fn un_compte_vise_par_un_signalement_ouvert_est_differe() {
        let db = base_de_test().await;
        compte(&db, "vise", Some(PURGE_COMPTE_JOURS + 5)).await;
        compte(&db, "plaignant", None).await;

        reports::ActiveModel {
            id: Set("r1".to_string()),
            author_id: Set("plaignant".to_string()),
            target_id: Set("vise".to_string()),
            reason: Set("harcelement".to_string()),
            details: Set(String::new()),
            state: Set("ouvert".to_string()),
            handled_at: Set(None),
            created_at: Set(Utc::now().naive_utc()),
        }
        .insert(&db)
        .await
        .expect("signalement inséré");

        let bilan = executer(&db).await.expect("purge");
        assert_eq!(bilan.comptes_effaces, 0);
        assert_eq!(bilan.comptes_differes, 1);
        assert!(
            accounts::Entity::find_by_id("vise")
                .one(&db)
                .await
                .expect("lecture")
                .is_some(),
            "le compte reste tant que le signalement n'est pas clos"
        );
    }

    /// Et une fois le dossier clos, le compte part au passage suivant.
    #[tokio::test]
    async fn le_compte_differe_part_une_fois_le_signalement_clos() {
        let db = base_de_test().await;
        compte(&db, "vise", Some(PURGE_COMPTE_JOURS + 5)).await;
        compte(&db, "plaignant", None).await;

        let signalement = reports::ActiveModel {
            id: Set("r1".to_string()),
            author_id: Set("plaignant".to_string()),
            target_id: Set("vise".to_string()),
            reason: Set("harcelement".to_string()),
            details: Set(String::new()),
            state: Set("ouvert".to_string()),
            handled_at: Set(None),
            created_at: Set(Utc::now().naive_utc()),
        }
        .insert(&db)
        .await
        .expect("signalement inséré");

        assert_eq!(executer(&db).await.expect("purge").comptes_differes, 1);

        let mut clos: reports::ActiveModel = signalement.into();
        clos.state = Set("traite".to_string());
        clos.handled_at = Set(Some(Utc::now().naive_utc()));
        clos.update(&db).await.expect("clôture");

        assert_eq!(executer(&db).await.expect("purge").comptes_effaces, 1);
    }

    /// Une session périmée s'en va, une session encore vivante reste.
    #[tokio::test]
    async fn les_activites_perimees_partent_les_vivantes_restent() {
        use crate::entities::{devices, live_activity_sessions};
        let db = base_de_test().await;
        compte(&db, "porteur", None).await;

        devices::ActiveModel {
            id: Set("d1".to_string()),
            account_id: Set("porteur".to_string()),
            platform: Set("ios".to_string()),
            vendor_id: Set("v1".to_string()),
            model: Set(Some("iPhone".to_string())),
            os_version: Set(Some("26.0".to_string())),
            app_version: Set(Some("0.1.0".to_string())),
            apns_token: Set(None),
            push_to_start_token: Set(None),
            apns_environment: Set("sandbox".to_string()),
            last_seen_at: Set(Utc::now().naive_utc()),
            created_at: Set(Utc::now().naive_utc()),
        }
        .insert(&db)
        .await
        .expect("appareil inséré");

        for (id, decalage) in [("perimee", -1_i64), ("vivante", 1)] {
            live_activity_sessions::ActiveModel {
                id: Set(id.to_string()),
                account_id: Set("porteur".to_string()),
                device_id: Set("d1".to_string()),
                update_token: Set(format!("jeton-{id}")),
                started_at: Set(Utc::now().naive_utc()),
                last_state_json: Set(String::new()),
                last_push_at: Set(None),
                ended_at: Set(None),
                stale_at: Set(Utc::now().naive_utc() + Duration::hours(decalage)),
            }
            .insert(&db)
            .await
            .expect("session insérée");
        }

        let bilan = executer(&db).await.expect("purge");
        assert_eq!(bilan.activites_effacees, 1);

        let restantes = live_activity_sessions::Entity::find()
            .all(&db)
            .await
            .expect("lecture");
        assert_eq!(restantes.len(), 1);
        assert_eq!(restantes[0].id, "vivante");
    }

    /// Le point qui rend la purge dans le dyno acceptable : deux dynos qui se
    /// réveillent ensemble ne purgent pas deux fois. Le second voit le verrou.
    #[tokio::test]
    async fn un_seul_passage_est_accorde_par_periode() {
        use crate::tests::Service;
        let service = Service::monter().await;
        let cle = crate::cache::cles::verrou(&format!("purge-test-{}", service.id("v")));

        let premier = crate::cache::prendre_verrou(&service.etat.cache, &cle, 60)
            .await
            .expect("cache joignable");
        let second = crate::cache::prendre_verrou(&service.etat.cache, &cle, 60)
            .await
            .expect("cache joignable");

        assert!(premier, "le premier passage prend le verrou");
        assert!(!second, "le second doit repasser son tour");

        let _ = crate::cache::oublier(&service.etat.cache, &cle).await;
    }

    #[tokio::test]
    async fn une_purge_sans_rien_a_faire_ne_casse_pas() {
        let db = base_de_test().await;
        assert_eq!(executer(&db).await.expect("purge"), Bilan::default());
    }
}

#[cfg(test)]
mod tests_cascade {
    use super::*;
    use crate::entities::profiles;
    use crate::tests::base_de_test;
    use sea_orm::{ActiveModelTrait, Set};

    /// La purge s'appuie entièrement sur `ON DELETE CASCADE`. Or SQLite
    /// n'applique les clés étrangères que si `PRAGMA foreign_keys` est actif :
    /// sans lui, le compte disparaîtrait en laissant sa fiche, ses plans et ses
    /// messages derrière — une purge qui ne purge pas, et que rien ne
    /// signalerait.
    #[tokio::test]
    async fn effacer_un_compte_emporte_ses_donnees_liees() {
        let db = base_de_test().await;
        super::tests::compte(&db, "partant", Some(PURGE_COMPTE_JOURS + 1)).await;

        profiles::ActiveModel {
            id: Set("p1".to_string()),
            account_id: Set("partant".to_string()),
            city: Set("Lyon".to_string()),
            lat_rounded: Set(45.75),
            lon_rounded: Set(4.85),
            gender: Set("autre".to_string()),
            bio: Set("Une phrase.".to_string()),
            photo_key: Set(None),
            photo_reviewed_at: Set(None),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
        }
        .insert(&db)
        .await
        .expect("fiche insérée");

        assert_eq!(executer(&db).await.expect("purge").comptes_effaces, 1);

        let fiches = profiles::Entity::find()
            .filter(profiles::Column::AccountId.eq("partant"))
            .all(&db)
            .await
            .expect("lecture");
        assert!(
            fiches.is_empty(),
            "la fiche a survécu au compte : la cascade ne s'applique pas"
        );
    }
}
