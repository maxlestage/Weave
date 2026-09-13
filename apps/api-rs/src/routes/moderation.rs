//! Bloquer, signaler, se mettre en pause.
//!
//! Trois gestes que quelqu'un pose quand la situation se dégrade. Aucun ne
//! notifie l'autre partie : prévenir qu'on vient d'être bloqué ou signalé
//! n'apaise rien et expose la personne qui s'est protégée.

use crate::{
    AppState,
    auth::Authentifie,
    entities::{
        accounts, audit_events, blocks, conversations, join_requests, messages, plans, reports,
    },
    error::{AppError, invalide},
    limitation::{Regle, consommer},
};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, post},
};
use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, QueryFilter, Set, TransactionTrait,
};
use serde::Deserialize;
use serde_json::{Value, json};

/// Évite le harcèlement par signalement en masse.
const REGLE_SIGNALEMENT: Regle = Regle {
    seau: "report",
    limite: 20,
    fenetre_secondes: 24 * 60 * 60,
};

/// Les messages d'une conversation close sont purgés après ce délai.
///
/// Réexporté depuis `conversations`, qui pose la même date quand on clôt une
/// conversation à la main. Deux définitions auraient laissé le blocage et la
/// clôture promettre des délais différents pour les mêmes messages.
use super::conversations::RETENTION_MESSAGES_JOURS;
use super::me::oublier_fil;
use crate::messages::Msg;

/// Le motif qui ne peut pas attendre l'examen d'un dossier.
///
/// La politique de confidentialité s'y engage publiquement : un signalement
/// indiquant qu'un compte appartient à une personne mineure « entraîne la
/// suspension immédiate du compte ». Elle le disait sans que rien ne le fasse.
pub(crate) const MOTIF_MINEUR: &str = "mineur";

pub(crate) const MOTIFS: [&str; 6] = [
    "contenu_sexuel",
    "faux_profil",
    "mineur",
    "harcelement",
    "arnaque",
    "autre",
];

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/blocks", post(bloquer))
        .route("/v1/blocks/{account_id}", delete(lever_blocage))
        .route("/v1/reports", post(signaler))
        .route("/v1/me/pause", post(mettre_en_pause))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CibleCompte {
    account_id: String,
}

async fn bloquer(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Json(corps): Json<CibleCompte>,
) -> Result<Json<Value>, AppError> {
    if corps.account_id == compte.id {
        return Err(invalide(Msg::PasDeAutoBlocage));
    }

    poser_blocage(&state, &compte.id, &corps.account_id).await?;
    couper_entre(&state, &compte.id, &corps.account_id).await?;

    Ok(Json(json!({ "ok": true })))
}

async fn lever_blocage(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Path(cible): Path<String>,
) -> Result<Json<Value>, AppError> {
    blocks::Entity::delete_many()
        .filter(blocks::Column::AuthorId.eq(compte.id.as_str()))
        .filter(blocks::Column::TargetId.eq(cible.as_str()))
        .exec(&state.db)
        .await?;

    oublier_fil(&state, &compte.id).await;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Signalement {
    account_id: String,
    reason: String,
    details: Option<String>,
}

async fn signaler(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Json(corps): Json<Signalement>,
) -> Result<Json<Value>, AppError> {
    consommer(&state, REGLE_SIGNALEMENT, &compte.id).await?;

    if !MOTIFS.contains(&corps.reason.as_str()) {
        return Err(invalide(Msg::MotifDeSignalementInconnu));
    }
    let details = corps.details.unwrap_or_default();
    if details.chars().count() > 1000 {
        return Err(invalide(Msg::PrecisionsTropLongues { maximum: 1000 }));
    }

    let motif = corps.reason.clone();
    reports::ActiveModel {
        id: Set(cuid2::create_id()),
        author_id: Set(compte.id.clone()),
        target_id: Set(corps.account_id.clone()),
        reason: Set(corps.reason),
        details: Set(details),
        ..Default::default()
    }
    .insert(&state.db)
    .await?;

    // Un signalement bloque d'office : la personne n'a pas à revoir les plans
    // de qui elle vient de signaler pendant que l'équipe examine le dossier.
    poser_blocage(&state, &compte.id, &corps.account_id).await?;
    couper_entre(&state, &compte.id, &corps.account_id).await?;

    if motif == MOTIF_MINEUR {
        suspendre_pour_examen(&state, &compte.id, &corps.account_id).await?;
    }

    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct Pause {
    paused: bool,
}

async fn mettre_en_pause(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Json(corps): Json<Pause>,
) -> Result<Json<Value>, AppError> {
    // Une seule écriture, conditionnée sur les deux statuts entre lesquels la
    // pause bascule.
    //
    // « Reprendre » écrivait « active » sans regarder le statut de départ : un
    // compte à « deleting » redevenait donc actif — visible dans le fil, et
    // joignable — tout en restant marqué pour la purge. Il se serait évanoui
    // au bout de trente jours au milieu de conversations en cours. Le portier
    // refuse désormais ces comptes, mais un basculement de statut ne doit pas
    // dépendre d'un garde placé ailleurs.
    let vise = if corps.paused { "paused" } else { "active" };
    let resultat = accounts::Entity::update_many()
        .col_expr(
            accounts::Column::Status,
            sea_orm::sea_query::Expr::value(vise),
        )
        .col_expr(
            accounts::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(Utc::now().naive_utc()),
        )
        .filter(accounts::Column::Id.eq(compte.id.as_str()))
        .filter(accounts::Column::Status.is_in(["active", "paused"]))
        .exec(&state.db)
        .await?;

    if resultat.rows_affected == 0 {
        return Err(invalide(Msg::PauseHorsEtat));
    }

    // En pause, ses plans ouverts sortent du fil des autres : rien ne sert de
    // laisser visible un rendez-vous auquel on ne répondra pas. Ils y
    // reviennent en reprenant — « suspendu » est un aller-retour, pas une
    // annulation.
    //
    // La reprise ne les restituait pas : les plans restaient « suspendu »
    // indéfiniment, un état qu'aucune autre ligne ne relit et que rien ne
    // défait. Mettre son compte en pause revenait donc à perdre ses plans pour
    // de bon, alors que la page publique promet de reprendre quand on veut.
    if corps.paused {
        plans::Entity::update_many()
            .col_expr(
                plans::Column::State,
                sea_orm::sea_query::Expr::value("suspendu"),
            )
            .filter(plans::Column::AuthorId.eq(compte.id.as_str()))
            .filter(plans::Column::State.eq("ouvert"))
            .exec(&state.db)
            .await?;
    } else {
        // Seuls les rendez-vous encore à venir reviennent. Un plan dont
        // l'heure est passée pendant la pause n'a plus lieu d'être rouvert :
        // il serait republié pour une date révolue.
        plans::Entity::update_many()
            .col_expr(
                plans::Column::State,
                sea_orm::sea_query::Expr::value("ouvert"),
            )
            .filter(plans::Column::AuthorId.eq(compte.id.as_str()))
            .filter(plans::Column::State.eq("suspendu"))
            .filter(plans::Column::StartsAt.gt(Utc::now().naive_utc()))
            .exec(&state.db)
            .await?;
    }

    // Le palier, le statut et le fuseau vivent dans le résumé d'identité : il
    // faut l'oublier, sinon le compte resterait « actif » un quart d'heure.
    crate::auth::oublier_compte(&state, &compte.id).await;
    oublier_fil(&state, &compte.id).await;
    crate::live_activity::publier_au_mieux(&state, &compte.id).await;

    Ok(Json(json!({ "ok": true })))
}

/// Suspend un compte signalé comme appartenant à une personne mineure.
///
/// ## Pourquoi c'est automatique
///
/// Weave est réservé aux majeurs, et la politique de confidentialité promet la
/// suspension immédiate. Attendre l'examen humain d'un dossier laisserait
/// l'accès ouvert pendant ce temps ; c'est le seul motif où le délai coûte
/// plus cher que l'erreur.
///
/// ## Ce que cela coûte, et pourquoi on l'accepte
///
/// Un signalement n'est pas une preuve. Celui-ci suffit donc à mettre un
/// compte hors circulation, et quelqu'un de malveillant peut s'en servir pour
/// faire taire un majeur. Trois choses bornent l'abus : les signalements sont
/// plafonnés à vingt par jour et par personne, chacun porte le nom de son
/// auteur, et la suspension s'inscrit au journal d'audit avec ce nom — un
/// usage répété se voit et se sanctionne.
///
/// La suspension se lève à la main, après examen. Elle n'efface rien : le
/// compte et ses données restent, et le dossier s'instruit.
///
/// Une seule écriture conditionnelle : un compte déjà suspendu ou en cours de
/// suppression n'est pas ramené en arrière, et le journal ne consigne que les
/// suspensions qui ont réellement eu lieu.
async fn suspendre_pour_examen(
    state: &AppState,
    auteur: &str,
    cible: &str,
) -> Result<(), AppError> {
    let suspendu = accounts::Entity::update_many()
        .col_expr(
            accounts::Column::Status,
            sea_orm::sea_query::Expr::value("suspended"),
        )
        .col_expr(
            accounts::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(Utc::now().naive_utc()),
        )
        .filter(accounts::Column::Id.eq(cible))
        .filter(accounts::Column::Status.is_in(["active", "paused", "onboarding"]))
        .exec(&state.db)
        .await?;

    if suspendu.rows_affected == 0 {
        return Ok(());
    }

    audit_events::ActiveModel {
        id: Set(cuid2::create_id()),
        account_id: Set(Some(cible.to_string())),
        action: Set("suspension_signalement_mineur".to_string()),
        subject: Set(Some(auteur.to_string())),
        meta_json: Set(json!({ "motif": MOTIF_MINEUR }).to_string()),
        ip: Set(None),
        created_at: Set(Utc::now().naive_utc()),
    }
    .insert(&state.db)
    .await?;

    // Le résumé du compte vit un quart d'heure dans le cache : sans cet oubli,
    // la suspension ne prendrait effet qu'à son expiration — et « immédiate »
    // ne serait vrai que sur le papier.
    crate::auth::oublier_compte(state, cible).await;
    oublier_fil(state, cible).await;

    tracing::warn!(
        compte = cible,
        "compte suspendu sur signalement de minorité"
    );
    Ok(())
}

/// Pose un blocage s'il n'existe pas déjà. Bloquer deux fois n'est pas une
/// erreur : c'est le même état.
async fn poser_blocage(state: &AppState, auteur: &str, cible: &str) -> Result<(), AppError> {
    let existant = blocks::Entity::find()
        .filter(blocks::Column::AuthorId.eq(auteur))
        .filter(blocks::Column::TargetId.eq(cible))
        .one(&state.db)
        .await?;
    if existant.is_some() {
        return Ok(());
    }

    blocks::ActiveModel {
        id: Set(cuid2::create_id()),
        author_id: Set(auteur.to_string()),
        target_id: Set(cible.to_string()),
        ..Default::default()
    }
    .insert(&state.db)
    .await?;
    Ok(())
}

/// Défait tout ce qui liait deux comptes : demandes en attente closes,
/// conversations fermées, fils invalidés dans les deux sens.
///
/// Rien n'est supprimé de force — les messages déjà échangés restent lisibles
/// par celui qui bloque jusqu'à la purge, et une suppression immédiate
/// effacerait aussi les preuves d'un comportement qu'on vient de signaler.
async fn couper_entre(state: &AppState, a: &str, b: &str) -> Result<(), AppError> {
    let maintenant = Utc::now().naive_utc();
    let purge = (Utc::now() + Duration::days(RETENTION_MESSAGES_JOURS)).naive_utc();

    let transaction = state.db.begin().await?;

    // Les demandes en attente entre les deux, dans un sens comme dans l'autre.
    let plans_de_a: Vec<String> = plans::Entity::find()
        .filter(plans::Column::AuthorId.eq(a))
        .all(&transaction)
        .await?
        .into_iter()
        .map(|p| p.id)
        .collect();
    let plans_de_b: Vec<String> = plans::Entity::find()
        .filter(plans::Column::AuthorId.eq(b))
        .all(&transaction)
        .await?
        .into_iter()
        .map(|p| p.id)
        .collect();

    for (demandeur, plans_cibles) in [(b, &plans_de_a), (a, &plans_de_b)] {
        if plans_cibles.is_empty() {
            continue;
        }
        join_requests::Entity::update_many()
            .col_expr(
                join_requests::Column::State,
                sea_orm::sea_query::Expr::value("expiree"),
            )
            .col_expr(
                join_requests::Column::DecidedAt,
                sea_orm::sea_query::Expr::value(maintenant),
            )
            .filter(join_requests::Column::State.eq("envoyee"))
            .filter(join_requests::Column::AuthorId.eq(demandeur))
            .filter(join_requests::Column::PlanId.is_in(plans_cibles.clone()))
            .exec(&transaction)
            .await?;
    }

    let ouvertes: Vec<String> = conversations::Entity::find()
        .filter(conversations::Column::ClosedAt.is_null())
        .filter(
            Condition::any()
                .add(
                    Condition::all()
                        .add(conversations::Column::HostId.eq(a))
                        .add(conversations::Column::GuestId.eq(b)),
                )
                .add(
                    Condition::all()
                        .add(conversations::Column::HostId.eq(b))
                        .add(conversations::Column::GuestId.eq(a)),
                ),
        )
        .all(&transaction)
        .await?
        .into_iter()
        .map(|c| c.id)
        .collect();

    if !ouvertes.is_empty() {
        conversations::Entity::update_many()
            .col_expr(
                conversations::Column::ClosedAt,
                sea_orm::sea_query::Expr::value(maintenant),
            )
            .col_expr(
                conversations::Column::ClosedBy,
                sea_orm::sea_query::Expr::value(a),
            )
            .filter(conversations::Column::Id.is_in(ouvertes.clone()))
            .exec(&transaction)
            .await?;

        messages::Entity::update_many()
            .col_expr(
                messages::Column::PurgeAfter,
                sea_orm::sea_query::Expr::value(purge),
            )
            .filter(messages::Column::ConversationId.is_in(ouvertes))
            .exec(&transaction)
            .await?;
    }

    transaction.commit().await?;

    oublier_fil(state, a).await;
    oublier_fil(state, b).await;
    // Les deux côtés perdent des conversations et des demandes : ce que chacun
    // voit sur son écran verrouillé a changé.
    crate::live_activity::publier_au_mieux(state, a).await;
    crate::live_activity::publier_au_mieux(state, b).await;
    Ok(())
}
