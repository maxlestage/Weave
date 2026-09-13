//! La console de modération : les gestes que les pages publiques promettent et
//! que rien ne pouvait poser.
//!
//! Trois colonnes existaient, écrites par personne :
//!
//! - `reports.handledAt` — la clôture d'un dossier. Lue par la purge pour
//!   différer la suppression d'un compte signalé, écrite nulle part. Un
//!   dossier ne pouvait donc pas être instruit.
//! - `accounts.status = 'suspended'` — posé d'office sur un signalement de
//!   minorité, avec en commentaire « la suspension se lève à la main, après
//!   examen ». Aucune main ne pouvait la lever : un signalement mensonger
//!   suspendait quelqu'un pour toujours.
//! - `profiles.photoReviewedAt` — remis à `NULL` à chaque envoi de photo, et
//!   jamais relu. Aucune photo n'était jamais examinée.
//!
//! Les CGU annoncent « l'examen du dossier », les mentions légales que « les
//! décisions de modération peuvent être contestées ». Les deux supposent
//! quelqu'un qui puisse décider, puis se déjuger.
//!
//! ## Pourquoi une commande et pas une route
//!
//! Une surface d'administration en HTTP demanderait une authentification
//! d'administrateur : un rôle, un second chemin de connexion, et une porte de
//! plus sur l'internet public — celle-là même qui donne le pouvoir de
//! suspendre un compte et de lire les détails d'un signalement.
//!
//! `weave-api console …` ne pose aucune porte. L'autorisation, c'est l'accès
//! au dyno : `heroku run`, qui passe par le compte Heroku et son second
//! facteur. Cela se fait depuis le tableau de bord, donc depuis un téléphone.
//!
//! ## Ce que l'appelant doit faire
//!
//! `suspendre`, `retablir`, `verifier` et `deverifier` changent ce que porte le
//! résumé d'identité, qui vit un quart d'heure dans le cache. L'appelant doit
//! l'oublier — `console_cli` s'en charge, et c'est pourquoi ces fonctions ne
//! prennent qu'une base : elles restent éprouvables sans Redis.
//!
//! ## Ce que la console ne fait pas
//!
//! Elle ne juge rien. Clore un dossier ne suspend personne et ne supprime
//! rien ; suspendre est un geste distinct, qu'il faut poser exprès. Deux
//! commandes pour deux décisions : celle sur le dossier, celle sur le compte.

use crate::entities::{
    accounts, audit_events, media_objects, profiles, reports, subscriptions,
    verification_requests,
};
use chrono::{NaiveDateTime, Utc};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, Set,
};
use serde_json::json;

/// L'état que prend un dossier clos. `ouvert` est la valeur par défaut en base.
pub const ETAT_CLOS: &str = "clos";

/// Un dossier ouvert, tel qu'il se présente à qui doit l'instruire.
#[derive(Debug, PartialEq, Eq)]
pub struct Dossier {
    pub id: String,
    pub motif: String,
    pub details: String,
    pub cible: String,
    pub cible_nom: String,
    pub cible_statut: String,
    pub auteur: String,
    pub cree_le: NaiveDateTime,
}

/// Une photo envoyée et jamais examinée.
#[derive(Debug, PartialEq, Eq)]
pub struct PhotoEnAttente {
    pub compte: String,
    pub nom: String,
    pub cle: String,
    pub octets: i32,
    pub type_mime: String,
    pub envoyee_le: NaiveDateTime,
}

/// Ce qu'une commande a changé — rien n'est supposé, tout est relu.
#[derive(Debug, PartialEq, Eq)]
pub enum Issue {
    /// L'écriture a eu lieu.
    Fait,
    /// L'état visé était déjà celui-là. Reposer le même geste n'est pas une
    /// erreur, mais le dire évite de croire qu'on vient d'agir.
    Deja,
    /// Rien ne porte cet identifiant.
    Introuvable,
}

/// Les dossiers ouverts, du plus ancien au plus récent.
///
/// L'ordre n'est pas cosmétique : un dossier ouvert retient la suppression du
/// compte visé, et cette rétention est bornée à quatre-vingt-dix jours. Le
/// plus ancien est celui dont le délai court le plus vite.
pub async fn dossiers_ouverts(db: &DatabaseConnection) -> Result<Vec<Dossier>, DbErr> {
    let lignes = reports::Entity::find()
        .filter(reports::Column::HandledAt.is_null())
        .order_by_asc(reports::Column::CreatedAt)
        .all(db)
        .await?;

    let mut dossiers = Vec::with_capacity(lignes.len());
    for ligne in lignes {
        // La cible peut avoir disparu : `ON DELETE CASCADE` emporte le
        // signalement avec elle, mais la course existe le temps d'un passage.
        let cible = accounts::Entity::find_by_id(ligne.target_id.as_str())
            .one(db)
            .await?;
        dossiers.push(Dossier {
            id: ligne.id,
            motif: ligne.reason,
            details: ligne.details,
            cible: ligne.target_id,
            cible_nom: cible
                .as_ref()
                .map_or_else(|| "(compte effacé)".to_string(), |c| c.display_name.clone()),
            cible_statut: cible
                .as_ref()
                .map_or_else(|| "—".to_string(), |c| c.status.clone()),
            auteur: ligne.author_id,
            cree_le: ligne.created_at,
        });
    }
    Ok(dossiers)
}

/// Clôt un dossier.
///
/// L'écriture est conditionnée sur `handledAt IS NULL` : deux personnes qui
/// closent le même dossier n'en referment pas un déjà refermé, et la seconde
/// l'apprend au lieu d'écraser la date de la première.
///
/// Clore ne décide de rien sur le compte visé. C'est le point : la purge
/// cessait d'être différée dès la clôture, donc confondre « dossier instruit »
/// et « compte sanctionné » aurait fait d'un classement sans suite une
/// sanction silencieuse.
pub async fn clore(
    db: &DatabaseConnection,
    dossier: &str,
    note: Option<&str>,
) -> Result<Issue, DbErr> {
    let maintenant = Utc::now().naive_utc();
    let touche = reports::Entity::update_many()
        .col_expr(reports::Column::HandledAt, Expr::value(maintenant))
        .col_expr(reports::Column::State, Expr::value(ETAT_CLOS))
        .filter(reports::Column::Id.eq(dossier))
        .filter(reports::Column::HandledAt.is_null())
        .exec(db)
        .await?;

    if touche.rows_affected == 0 {
        let existe = reports::Entity::find_by_id(dossier).one(db).await?;
        return Ok(if existe.is_some() {
            Issue::Deja
        } else {
            Issue::Introuvable
        });
    }

    journaliser(
        db,
        None,
        "dossier_clos",
        Some(dossier),
        json!({ "note": note.unwrap_or_default() }),
    )
    .await?;
    Ok(Issue::Fait)
}

/// Suspend un compte après examen.
///
/// Le même état que pose un signalement de minorité, et la même écriture
/// conditionnelle : un compte déjà suspendu ou en cours de suppression n'est
/// pas ramené en arrière.
pub async fn suspendre(
    db: &DatabaseConnection,
    compte: &str,
    motif: &str,
) -> Result<Issue, DbErr> {
    let touche = accounts::Entity::update_many()
        .col_expr(accounts::Column::Status, Expr::value("suspended"))
        .col_expr(accounts::Column::UpdatedAt, Expr::value(Utc::now().naive_utc()))
        .filter(accounts::Column::Id.eq(compte))
        .filter(accounts::Column::Status.is_in(["active", "paused", "onboarding"]))
        .exec(db)
        .await?;

    if touche.rows_affected == 0 {
        return etat_inchange(db, compte).await;
    }

    journaliser(
        db,
        Some(compte),
        "suspension_console",
        None,
        json!({ "motif": motif }),
    )
    .await?;
    Ok(Issue::Fait)
}

/// Lève une suspension.
///
/// C'est la commande qui manquait le plus. Un signalement de minorité suspend
/// d'office, sans preuve et sans délai — c'est assumé, et borné par le fait
/// que la suspension « se lève à la main ». Elle ne se levait pas : quiconque
/// signalait un majeur pour minorité le mettait hors de l'application pour
/// toujours, et l'application retournait contre lui l'outil qui protège les
/// autres.
///
/// Ne ramène que depuis `suspended`. Un compte à `deleting` reste à
/// `deleting` : le rendre actif le rendrait joignable tout en le laissant
/// marqué pour la purge.
pub async fn retablir(db: &DatabaseConnection, compte: &str) -> Result<Issue, DbErr> {
    let touche = accounts::Entity::update_many()
        .col_expr(accounts::Column::Status, Expr::value("active"))
        .col_expr(accounts::Column::UpdatedAt, Expr::value(Utc::now().naive_utc()))
        .filter(accounts::Column::Id.eq(compte))
        .filter(accounts::Column::Status.eq("suspended"))
        .exec(db)
        .await?;

    if touche.rows_affected == 0 {
        return etat_inchange(db, compte).await;
    }

    journaliser(db, Some(compte), "retablissement_console", None, json!({})).await?;
    Ok(Issue::Fait)
}

/// Pose le badge « vérifié ».
///
/// `accounts.verified` est affiché par le fil et par la liste des demandes,
/// écrit à `false` à l'inscription, et remis à `true` par rien. Le badge ne
/// pouvait donc apparaître sur aucun profil — et « Vérification de profil
/// accélérée », vendue avec le palier Grand Tour, portait sur une procédure
/// qui n'existait à aucune vitesse.
///
/// ## Ce que le badge dit, et ce qu'il ne dit pas
///
/// Il dit qu'une personne a examiné une pièce et consigné sa décision avec son
/// motif. Il ne dit pas qu'un contrôle automatique a eu lieu : il n'y en a
/// aucun. Le motif est obligatoire pour cette raison — un badge posé sans
/// raison consignée ne vaudrait pas mieux que pas de badge du tout, et il
/// vaudrait moins, parce que quelqu'un s'y fierait.
pub async fn verifier(db: &DatabaseConnection, compte: &str, motif: &str) -> Result<Issue, DbErr> {
    let issue = poser_verification(db, compte, true, "verification_console", motif).await?;
    // Poser le badge répond à la demande : la laisser en attente la ferait
    // ressortir à chaque relevé de la file, pour un travail déjà fait.
    clore_demande(db, compte, crate::routes::verification::ACCEPTEE, motif).await?;
    Ok(issue)
}

/// Retire le badge — sur une pièce périmée, un doute, ou une erreur.
///
/// Réversible par construction : un badge qu'on ne peut pas retirer force à
/// choisir entre laisser une affirmation fausse en place et supprimer le
/// compte de quelqu'un.
pub async fn deverifier(
    db: &DatabaseConnection,
    compte: &str,
    motif: &str,
) -> Result<Issue, DbErr> {
    poser_verification(db, compte, false, "deverification_console", motif).await
}

async fn poser_verification(
    db: &DatabaseConnection,
    compte: &str,
    vise: bool,
    action: &str,
    motif: &str,
) -> Result<Issue, DbErr> {
    let touche = accounts::Entity::update_many()
        .col_expr(accounts::Column::Verified, Expr::value(vise))
        .col_expr(accounts::Column::UpdatedAt, Expr::value(Utc::now().naive_utc()))
        .filter(accounts::Column::Id.eq(compte))
        .filter(accounts::Column::Verified.eq(!vise))
        .exec(db)
        .await?;

    if touche.rows_affected == 0 {
        return etat_inchange(db, compte).await;
    }

    journaliser(db, Some(compte), action, None, json!({ "motif": motif })).await?;
    Ok(Issue::Fait)
}

/// Une demande de vérification en attente, telle qu'elle se présente à qui
/// doit la traiter.
#[derive(Debug, PartialEq, Eq)]
pub struct DemandeDeVerification {
    pub compte: String,
    pub nom: String,
    pub palier: String,
    pub note: String,
    pub depuis: NaiveDateTime,
}

/// La file des demandes de vérification, prioritaire d'abord.
///
/// Le Grand Tour vend « Vérification de profil accélérée ». C'était invendable
/// tant qu'aucune file n'existait ; c'est cette fonction qui rend la promesse
/// tenable — et à condition de relever la file dans l'ordre qu'elle donne.
///
/// À palier égal, la plus ancienne d'abord : une priorité n'est pas un droit
/// de doubler indéfiniment.
pub async fn verifications_en_attente(
    db: &DatabaseConnection,
) -> Result<Vec<DemandeDeVerification>, DbErr> {
    let lignes = verification_requests::Entity::find()
        .filter(verification_requests::Column::State.eq(crate::routes::verification::EN_ATTENTE))
        .order_by_asc(verification_requests::Column::CreatedAt)
        .all(db)
        .await?;

    let mut file = Vec::with_capacity(lignes.len());
    for ligne in lignes {
        let compte = accounts::Entity::find_by_id(ligne.account_id.as_str())
            .one(db)
            .await?;
        file.push(DemandeDeVerification {
            palier: palier_de(db, &ligne.account_id).await?,
            nom: compte
                .as_ref()
                .map_or_else(|| "(compte effacé)".to_string(), |c| c.display_name.clone()),
            compte: ligne.account_id,
            note: ligne.note,
            depuis: ligne.created_at,
        });
    }

    // Le tri se fait après coup, sur le palier : le rang n'est pas une colonne
    // de la table des demandes, et l'y recopier le ferait mentir dès qu'un
    // abonnement change.
    file.sort_by_key(|d| (std::cmp::Reverse(rang_du_palier(&d.palier)), d.depuis));
    Ok(file)
}

/// Le rang de priorité d'un palier dans la file. Seul le Grand Tour achète une
/// priorité : c'est le seul dont le catalogue l'annonce.
fn rang_du_palier(palier: &str) -> u8 {
    u8::from(palier == "grandtour")
}

async fn palier_de(db: &DatabaseConnection, compte: &str) -> Result<String, DbErr> {
    Ok(subscriptions::Entity::find()
        .filter(subscriptions::Column::AccountId.eq(compte))
        .one(db)
        .await?
        .filter(|a| {
            a.expires_at
                .map(|echeance| echeance.and_utc() > Utc::now())
                .unwrap_or(true)
        })
        .map_or_else(|| "depart".to_string(), |a| a.tier))
}

/// Refuse une demande de vérification, sans poser le badge.
///
/// Le motif est obligatoire et rendu à la personne : les mentions légales
/// promettent qu'une décision de modération se conteste, et un refus dont on
/// ignore la raison ne se conteste pas.
pub async fn refuser_verification(
    db: &DatabaseConnection,
    compte: &str,
    motif: &str,
) -> Result<Issue, DbErr> {
    let close = clore_demande(db, compte, crate::routes::verification::REFUSEE, motif).await?;
    if close {
        journaliser(
            db,
            Some(compte),
            "verification_refusee",
            None,
            json!({ "motif": motif }),
        )
        .await?;
        return Ok(Issue::Fait);
    }
    Ok(Issue::Deja)
}

/// Clôt la demande en attente d'un compte, s'il y en a une.
///
/// Conditionnée sur l'état : deux passages ne réécrivent pas la date de
/// décision, et une demande déjà tranchée n'est pas rouverte.
async fn clore_demande(
    db: &DatabaseConnection,
    compte: &str,
    etat: &str,
    motif: &str,
) -> Result<bool, DbErr> {
    let touche = verification_requests::Entity::update_many()
        .col_expr(verification_requests::Column::State, Expr::value(etat))
        .col_expr(
            verification_requests::Column::Decision,
            Expr::value(motif.to_string()),
        )
        .col_expr(
            verification_requests::Column::HandledAt,
            Expr::value(Utc::now().naive_utc()),
        )
        .filter(verification_requests::Column::AccountId.eq(compte))
        .filter(
            verification_requests::Column::State.eq(crate::routes::verification::EN_ATTENTE),
        )
        .exec(db)
        .await?;
    Ok(touche.rows_affected > 0)
}

/// Les photos envoyées et jamais examinées, de la plus ancienne à la plus
/// récente.
///
/// `photoReviewedAt` est remis à `NULL` à chaque envoi : changer de photo
/// ramène le compte dans cette file, ce qui est bien la question posée.
pub async fn photos_a_examiner(db: &DatabaseConnection) -> Result<Vec<PhotoEnAttente>, DbErr> {
    let fiches = profiles::Entity::find()
        .filter(profiles::Column::PhotoKey.is_not_null())
        .filter(profiles::Column::PhotoReviewedAt.is_null())
        .order_by_asc(profiles::Column::UpdatedAt)
        .all(db)
        .await?;

    let mut attente = Vec::with_capacity(fiches.len());
    for fiche in fiches {
        let Some(cle) = fiche.photo_key.clone() else {
            continue;
        };
        let objet = media_objects::Entity::find_by_id(cle.as_str()).one(db).await?;
        let nom = accounts::Entity::find_by_id(fiche.account_id.as_str())
            .one(db)
            .await?
            .map_or_else(|| "(compte effacé)".to_string(), |c| c.display_name);
        attente.push(PhotoEnAttente {
            compte: fiche.account_id,
            nom,
            cle,
            octets: objet.as_ref().map_or(0, |o| o.byte_size),
            type_mime: objet
                .as_ref()
                .map_or_else(|| "—".to_string(), |o| o.content_type.clone()),
            envoyee_le: objet.as_ref().map_or(fiche.updated_at, |o| o.created_at),
        });
    }
    Ok(attente)
}

/// Marque une photo comme examinée et gardée.
///
/// Ne réécrit pas une date déjà posée : l'examen a eu lieu à sa date, pas à
/// celle du second passage.
pub async fn photo_valider(db: &DatabaseConnection, compte: &str) -> Result<Issue, DbErr> {
    let touche = profiles::Entity::update_many()
        .col_expr(
            profiles::Column::PhotoReviewedAt,
            Expr::value(Utc::now().naive_utc()),
        )
        .filter(profiles::Column::AccountId.eq(compte))
        .filter(profiles::Column::PhotoKey.is_not_null())
        .filter(profiles::Column::PhotoReviewedAt.is_null())
        .exec(db)
        .await?;

    if touche.rows_affected == 0 {
        return photo_inchangee(db, compte).await;
    }

    journaliser(db, Some(compte), "photo_validee", None, json!({})).await?;
    Ok(Issue::Fait)
}

/// Retire une photo et efface ses octets.
///
/// Deux écritures, et l'ordre compte : la fiche cesse de désigner la clé
/// d'abord, les octets partent ensuite. L'inverse laisserait, le temps d'un
/// battement, une fiche qui pointe vers un média absent — ce que la route de
/// service traduirait par une erreur plutôt que par une absence de photo.
///
/// `photoReviewedAt` reste à `NULL` : il n'y a plus de photo à examiner, et y
/// poser une date ferait croire qu'une photo a été gardée.
pub async fn photo_retirer(db: &DatabaseConnection, compte: &str) -> Result<Issue, DbErr> {
    let Some(fiche) = profiles::Entity::find()
        .filter(profiles::Column::AccountId.eq(compte))
        .one(db)
        .await?
    else {
        return Ok(Issue::Introuvable);
    };
    let Some(cle) = fiche.photo_key.clone() else {
        return Ok(Issue::Deja);
    };

    let mut maj: profiles::ActiveModel = fiche.into();
    maj.photo_key = Set(None);
    maj.photo_reviewed_at = Set(None);
    maj.updated_at = Set(Utc::now().naive_utc());
    maj.update(db).await?;

    media_objects::Entity::delete_by_id(cle.as_str()).exec(db).await?;

    journaliser(db, Some(compte), "photo_retiree", None, json!({ "cle": cle })).await?;
    Ok(Issue::Fait)
}

/// Distingue « ce compte n'existe pas » de « son statut interdisait le geste ».
async fn etat_inchange(db: &DatabaseConnection, compte: &str) -> Result<Issue, DbErr> {
    Ok(
        if accounts::Entity::find_by_id(compte).one(db).await?.is_some() {
            Issue::Deja
        } else {
            Issue::Introuvable
        },
    )
}

async fn photo_inchangee(db: &DatabaseConnection, compte: &str) -> Result<Issue, DbErr> {
    let fiches = profiles::Entity::find()
        .filter(profiles::Column::AccountId.eq(compte))
        .count(db)
        .await?;
    Ok(if fiches > 0 { Issue::Deja } else { Issue::Introuvable })
}

/// Inscrit le geste au journal d'audit.
///
/// Une décision de modération qui ne laisse pas de trace ne se conteste pas,
/// et les mentions légales promettent qu'elle se conteste.
async fn journaliser(
    db: &DatabaseConnection,
    compte: Option<&str>,
    action: &str,
    sujet: Option<&str>,
    meta: serde_json::Value,
) -> Result<(), DbErr> {
    audit_events::ActiveModel {
        id: Set(cuid2::create_id()),
        account_id: Set(compte.map(str::to_string)),
        action: Set(action.to_string()),
        subject: Set(sujet.map(str::to_string)),
        meta_json: Set(meta.to_string()),
        ip: Set(None),
        created_at: Set(Utc::now().naive_utc()),
    }
    .insert(db)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::base_de_test;
    use chrono::Duration;
    use sea_orm::{ActiveModelTrait, QuerySelect, Set};

    async fn compte(db: &DatabaseConnection, id: &str, statut: &str) {
        accounts::ActiveModel {
            id: Set(id.to_string()),
            email: Set(format!("{id}@exemple.test")),
            email_hash: Set(format!("h-{id}")),
            handle: Set(id.to_string()),
            display_name: Set(format!("{id} le prénom")),
            birth_date: Set(chrono::NaiveDate::from_ymd_opt(1995, 1, 1)
                .expect("date valide")
                .and_hms_opt(0, 0, 0)
                .expect("heure valide")),
            status: Set(statut.to_string()),
            timezone: Set("Europe/Paris".to_string()),
            locale: Set("fr".to_string()),
            verified: Set(false),
            last_seen_at: Set(None),
            last_bilan_at: Set(None),
            deletion_requested_at: Set(None),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
        }
        .insert(db)
        .await
        .expect("compte inséré");
    }

    async fn signalement(db: &DatabaseConnection, id: &str, auteur: &str, cible: &str, age: i64) {
        reports::ActiveModel {
            id: Set(id.to_string()),
            author_id: Set(auteur.to_string()),
            target_id: Set(cible.to_string()),
            reason: Set("harcelement".to_string()),
            details: Set("il insiste après un refus".to_string()),
            state: Set("ouvert".to_string()),
            handled_at: Set(None),
            created_at: Set(Utc::now().naive_utc() - Duration::days(age)),
        }
        .insert(db)
        .await
        .expect("signalement inséré");
    }

    async fn fiche_avec_photo(db: &DatabaseConnection, compte_id: &str, cle: &str) {
        media_objects::ActiveModel {
            key: Set(cle.to_string()),
            account_id: Set(compte_id.to_string()),
            content_type: Set("image/jpeg".to_string()),
            bytes: Set(vec![0xFF, 0xD8, 0xFF]),
            byte_size: Set(3),
            created_at: Set(Utc::now().naive_utc()),
        }
        .insert(db)
        .await
        .expect("média inséré");

        profiles::ActiveModel {
            id: Set(format!("f-{compte_id}")),
            account_id: Set(compte_id.to_string()),
            city: Set("Lyon".to_string()),
            lat_rounded: Set(45.75),
            lon_rounded: Set(4.85),
            gender: Set("femme".to_string()),
            bio: Set(String::new()),
            photo_key: Set(Some(cle.to_string())),
            photo_reviewed_at: Set(None),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
        }
        .insert(db)
        .await
        .expect("fiche insérée");
    }

    /// Ce que la console existe pour faire : clore un dossier, ce que rien ne
    /// pouvait faire.
    #[tokio::test]
    async fn clore_un_dossier_pose_la_date_que_rien_n_ecrivait() {
        let db = base_de_test().await;
        compte(&db, "cible", "active").await;
        compte(&db, "plaignant", "active").await;
        signalement(&db, "d1", "plaignant", "cible", 0).await;

        assert_eq!(dossiers_ouverts(&db).await.expect("lecture").len(), 1);
        assert_eq!(clore(&db, "d1", Some("classé sans suite")).await.expect("clôture"), Issue::Fait);

        let relu = reports::Entity::find_by_id("d1")
            .one(&db)
            .await
            .expect("lecture")
            .expect("le dossier existe");
        assert!(relu.handled_at.is_some(), "la clôture pose handledAt");
        assert_eq!(relu.state, ETAT_CLOS);
        assert!(
            dossiers_ouverts(&db).await.expect("lecture").is_empty(),
            "un dossier clos sort de la file"
        );
    }

    /// Clore deux fois n'écrase pas la date de la première clôture — et le dit.
    #[tokio::test]
    async fn clore_deux_fois_ne_reecrit_pas_la_premiere_cloture() {
        let db = base_de_test().await;
        compte(&db, "cible2", "active").await;
        compte(&db, "plaignant2", "active").await;
        signalement(&db, "d2", "plaignant2", "cible2", 0).await;

        clore(&db, "d2", None).await.expect("clôture");
        let premiere = reports::Entity::find_by_id("d2")
            .one(&db)
            .await
            .expect("lecture")
            .expect("le dossier existe")
            .handled_at;

        assert_eq!(clore(&db, "d2", None).await.expect("seconde"), Issue::Deja);
        let seconde = reports::Entity::find_by_id("d2")
            .one(&db)
            .await
            .expect("lecture")
            .expect("le dossier existe")
            .handled_at;
        assert_eq!(premiere, seconde, "la date d'instruction est celle de l'instruction");

        assert_eq!(clore(&db, "jamais_vu", None).await.expect("inconnu"), Issue::Introuvable);
    }

    /// La clôture rend au compte visé son droit à l'effacement — le seul effet
    /// que `handledAt` avait déjà, et qui n'était atteignable par personne.
    #[tokio::test]
    async fn clore_libere_la_purge_du_compte_vise() {
        let db = base_de_test().await;
        compte(&db, "vise_p", "deleting").await;
        compte(&db, "plaignant_p", "active").await;
        signalement(&db, "d3", "plaignant_p", "vise_p", 0).await;

        accounts::Entity::update_many()
            .col_expr(
                accounts::Column::DeletionRequestedAt,
                Expr::value(
                    Utc::now().naive_utc() - Duration::days(crate::purge::PURGE_COMPTE_JOURS + 1),
                ),
            )
            .filter(accounts::Column::Id.eq("vise_p"))
            .exec(&db)
            .await
            .expect("échéance posée");

        let avant = crate::purge::executer(&db).await.expect("purge");
        assert_eq!(avant.comptes_differes, 1, "le dossier ouvert retient");
        assert_eq!(avant.comptes_effaces, 0);

        clore(&db, "d3", Some("rien à retenir")).await.expect("clôture");

        let apres = crate::purge::executer(&db).await.expect("purge");
        assert_eq!(apres.comptes_effaces, 1, "le dossier clos ne retient plus");
    }

    /// La phrase que le code portait sans pouvoir la tenir : « la suspension se
    /// lève à la main, après examen ».
    #[tokio::test]
    async fn une_suspension_se_leve() {
        let db = base_de_test().await;
        compte(&db, "suspendu", "suspended").await;

        assert_eq!(retablir(&db, "suspendu").await.expect("levée"), Issue::Fait);
        assert_eq!(statut(&db, "suspendu").await, "active");

        assert_eq!(
            retablir(&db, "suspendu").await.expect("seconde"),
            Issue::Deja,
            "un compte actif n'est pas à rétablir"
        );
        assert_eq!(
            retablir(&db, "inconnu").await.expect("inconnu"),
            Issue::Introuvable
        );
    }

    /// Rétablir ne ressuscite pas un compte marqué pour la purge : il
    /// redeviendrait joignable tout en restant promis à l'effacement.
    #[tokio::test]
    async fn retablir_ne_ramene_pas_un_compte_en_cours_de_suppression() {
        let db = base_de_test().await;
        compte(&db, "partant", "deleting").await;

        assert_eq!(retablir(&db, "partant").await.expect("levée"), Issue::Deja);
        assert_eq!(statut(&db, "partant").await, "deleting");
    }

    #[tokio::test]
    async fn suspendre_met_hors_circulation_et_laisse_une_trace() {
        let db = base_de_test().await;
        compte(&db, "a_suspendre", "active").await;

        assert_eq!(
            suspendre(&db, "a_suspendre", "faux profil avéré").await.expect("suspension"),
            Issue::Fait
        );
        assert_eq!(statut(&db, "a_suspendre").await, "suspended");

        let trace = audit_events::Entity::find()
            .filter(audit_events::Column::Action.eq("suspension_console"))
            .filter(audit_events::Column::AccountId.eq("a_suspendre"))
            .one(&db)
            .await
            .expect("lecture")
            .expect("une décision de modération laisse une trace");
        assert!(
            trace.meta_json.contains("faux profil avéré"),
            "le motif est consigné : c'est ce qui rend la décision contestable"
        );
    }

    /// Suspendre ne clôt pas le dossier. Deux décisions, deux gestes : un
    /// compte sanctionné dont le dossier se refermerait tout seul ferait
    /// disparaître la sanction de la file avant qu'on ait décidé de la garder.
    #[tokio::test]
    async fn suspendre_ne_clot_pas_le_dossier() {
        let db = base_de_test().await;
        compte(&db, "cible_s", "active").await;
        compte(&db, "plaignant_s", "active").await;
        signalement(&db, "d4", "plaignant_s", "cible_s", 0).await;

        suspendre(&db, "cible_s", "harcèlement avéré").await.expect("suspension");
        assert_eq!(
            dossiers_ouverts(&db).await.expect("lecture").len(),
            1,
            "le dossier reste ouvert tant qu'on ne l'a pas clos"
        );
    }

    /// La file se lit du plus ancien au plus récent : c'est celui dont la
    /// rétention de quatre-vingt-dix jours court le plus vite.
    #[tokio::test]
    async fn la_file_donne_le_plus_ancien_d_abord_avec_de_quoi_decider() {
        let db = base_de_test().await;
        compte(&db, "cible_f", "suspended").await;
        compte(&db, "plaignant_f", "active").await;
        signalement(&db, "recent", "plaignant_f", "cible_f", 1).await;
        signalement(&db, "ancien", "plaignant_f", "cible_f", 40).await;

        let file = dossiers_ouverts(&db).await.expect("lecture");
        let ids: Vec<&str> = file.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(ids, ["ancien", "recent"]);

        let premier = &file[0];
        assert_eq!(premier.cible_nom, "cible_f le prénom");
        assert_eq!(
            premier.cible_statut, "suspended",
            "savoir que la cible est déjà suspendue évite de la suspendre deux fois"
        );
        assert_eq!(premier.details, "il insiste après un refus");
    }

    #[tokio::test]
    async fn une_photo_envoyee_attend_l_examen_puis_en_sort() {
        let db = base_de_test().await;
        compte(&db, "photographe", "active").await;
        fiche_avec_photo(&db, "photographe", "cle-1").await;

        let file = photos_a_examiner(&db).await.expect("lecture");
        assert_eq!(file.len(), 1);
        assert_eq!(file[0].compte, "photographe");
        assert_eq!(file[0].type_mime, "image/jpeg");

        assert_eq!(photo_valider(&db, "photographe").await.expect("examen"), Issue::Fait);
        assert!(
            photos_a_examiner(&db).await.expect("lecture").is_empty(),
            "une photo examinée quitte la file"
        );
        assert_eq!(
            photo_valider(&db, "photographe").await.expect("seconde"),
            Issue::Deja,
            "l'examen a eu lieu à sa date, pas à celle du second passage"
        );
    }

    /// Retirer une photo efface aussi ses octets : une fiche qui ne la désigne
    /// plus laisserait sinon l'image en base, hors de portée et pour toujours.
    #[tokio::test]
    async fn retirer_une_photo_efface_ses_octets() {
        let db = base_de_test().await;
        compte(&db, "retire", "active").await;
        fiche_avec_photo(&db, "retire", "cle-2").await;

        assert_eq!(photo_retirer(&db, "retire").await.expect("retrait"), Issue::Fait);

        let fiche = profiles::Entity::find()
            .filter(profiles::Column::AccountId.eq("retire"))
            .one(&db)
            .await
            .expect("lecture")
            .expect("la fiche reste");
        assert_eq!(fiche.photo_key, None);
        assert_eq!(
            fiche.photo_reviewed_at, None,
            "pas de photo à examiner, donc pas de date d'examen"
        );
        assert!(
            media_objects::Entity::find_by_id("cle-2")
                .one(&db)
                .await
                .expect("lecture")
                .is_none(),
            "les octets partent avec la photo"
        );
        assert!(photos_a_examiner(&db).await.expect("lecture").is_empty());
        assert_eq!(photo_retirer(&db, "retire").await.expect("seconde"), Issue::Deja);
    }

    /// Le badge n'était posable par personne : `verified` s'écrit `false` à
    /// l'inscription et rien ne le remettait à `true`. « Vérification de profil
    /// accélérée », vendue avec le Grand Tour, portait donc sur une procédure
    /// qui n'existait à aucune vitesse.
    #[tokio::test]
    async fn le_badge_se_pose_et_se_retire() {
        let db = base_de_test().await;
        compte(&db, "a_verifier", "active").await;
        assert!(!verifie(&db, "a_verifier").await, "personne ne naît vérifié");

        assert_eq!(
            verifier(&db, "a_verifier", "carte d'identité reçue le 12/09")
                .await
                .expect("vérification"),
            Issue::Fait
        );
        assert!(verifie(&db, "a_verifier").await);

        assert_eq!(
            verifier(&db, "a_verifier", "encore").await.expect("seconde"),
            Issue::Deja
        );

        assert_eq!(
            deverifier(&db, "a_verifier", "pièce périmée").await.expect("retrait"),
            Issue::Fait,
            "un badge qu'on ne peut pas retirer force à choisir entre une \
             affirmation fausse et la suppression d'un compte"
        );
        assert!(!verifie(&db, "a_verifier").await);

        assert_eq!(
            deverifier(&db, "a_verifier", "encore").await.expect("seconde"),
            Issue::Deja
        );
        assert_eq!(
            verifier(&db, "inconnu", "peu importe").await.expect("inconnu"),
            Issue::Introuvable
        );
    }

    /// Le motif est ce qui rend la décision contestable : il doit atteindre le
    /// journal d'audit, pas seulement la ligne de commande.
    #[tokio::test]
    async fn le_motif_du_badge_atteint_le_journal() {
        let db = base_de_test().await;
        compte(&db, "trace", "active").await;
        verifier(&db, "trace", "passeport vu en visio").await.expect("vérification");

        let trace = audit_events::Entity::find()
            .filter(audit_events::Column::Action.eq("verification_console"))
            .filter(audit_events::Column::AccountId.eq("trace"))
            .one(&db)
            .await
            .expect("lecture")
            .expect("une décision de modération laisse une trace");
        assert!(trace.meta_json.contains("passeport vu en visio"));
    }

    async fn verifie(db: &DatabaseConnection, compte: &str) -> bool {
        accounts::Entity::find_by_id(compte)
            .select_only()
            .column(accounts::Column::Verified)
            .into_tuple::<bool>()
            .one(db)
            .await
            .expect("lecture")
            .expect("le compte existe")
    }

    async fn statut(db: &DatabaseConnection, compte: &str) -> String {
        accounts::Entity::find_by_id(compte)
            .select_only()
            .column(accounts::Column::Status)
            .into_tuple::<String>()
            .one(db)
            .await
            .expect("lecture")
            .expect("le compte existe")
    }
}
