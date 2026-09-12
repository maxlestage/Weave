//! La purge : ce que les dates de rétention promettaient sans que rien ne
//! l'exécute.
//!
//! Jusqu'ici, supprimer son compte posait `deletion_requested_at` et clore une
//! conversation posait `purge_after` — puis les lignes restaient en base,
//! indéfiniment. Les deux colonnes décrivaient une intention, pas un effet.
//!
//! `weave-api purge` est un processus séparé, comme `migrate` : il ne démarre
//! ni serveur ni cache, se termine, et se planifie (Heroku Scheduler, cron).
//! Le faire tourner dans le dyno web aurait eu deux défauts — deux dynos
//! auraient purgé en même temps, et un dyno en veille n'aurait rien purgé du
//! tout.
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

use crate::entities::{accounts, messages, reports};
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

    // 2. Les comptes dont le délai est écoulé.
    let echeance = maintenant - Duration::days(PURGE_COMPTE_JOURS);
    let echus: Vec<String> = accounts::Entity::find()
        .filter(accounts::Column::DeletionRequestedAt.is_not_null())
        .filter(accounts::Column::DeletionRequestedAt.lte(echeance))
        .select_only()
        .column(accounts::Column::Id)
        .into_tuple()
        .all(db)
        .await?;

    let mut bilan = Bilan { messages_effaces, ..Default::default() };

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

/// Un signalement visant ce compte est-il encore à instruire ?
///
/// `handled_at` est la marque de clôture : tant qu'elle est nulle, le dossier
/// est ouvert, quelle que soit la valeur de `state`.
async fn signalement_en_cours(db: &DatabaseConnection, compte: &str) -> Result<bool, DbErr> {
    let ouverts = reports::Entity::find()
        .filter(reports::Column::TargetId.eq(compte))
        .filter(reports::Column::HandledAt.is_null())
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
