//! Les conversations, ouvertes par une demande acceptée.
//!
//! Sans accusé de lecture : savoir si l'autre a lu n'aide personne à décider.

use crate::messages::Msg;
use crate::{
    AppState,
    auth::Authentifie,
    crypto::signer_url_media,
    entities::{accounts, conversations, messages, plans, profiles},
    error::{AppError, Code, introuvable, invalide},
    limitation::{Regle, consommer},
    live_activity,
    temps::{age_depuis, iso8601},
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get},
};
use chrono::{DateTime, Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    Set, TransactionTrait,
};
use serde::Deserialize;
use serde_json::{Value, json};

/// Borne haute très large, uniquement anti-abus : une conversation entamée ne
/// doit jamais buter sur un plafond.
const REGLE_MESSAGE: Regle = Regle {
    seau: "message",
    limite: 240,
    fenetre_secondes: 60 * 60,
};

pub(crate) const MESSAGE_MAX: usize = 2000;

/// Page de messages renvoyée par lecture.
const PAGE: u64 = 50;

/// Conversations rendues d'un coup. Une borne, pas une pagination.
pub(crate) const CONVERSATIONS_RENDUES_MAX: u64 = 100;

/// Les messages d'une conversation close sont purgés au bout de ce délai.
/// `packages/contracts` fait foi.
pub(crate) const RETENTION_MESSAGES_JOURS: i64 = 90;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/conversations", get(les_miennes))
        .route("/v1/conversations/{id}", delete(clore))
        .route("/v1/conversations/{id}/messages", get(lire).post(ecrire))
}

/// Mes conversations.
///
/// Triées par dernier message, puis par ouverture : c'est l'ordre dans lequel
/// on s'attend à les retrouver. Pas de « vu à », pas d'indicateur de frappe —
/// ces mécaniques servent à retenir, pas à se parler.
async fn les_miennes(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
) -> Result<Json<Value>, AppError> {
    let lignes = conversations::Entity::find()
        .filter(
            Condition::any()
                .add(conversations::Column::HostId.eq(compte.id.as_str()))
                .add(conversations::Column::GuestId.eq(compte.id.as_str())),
        )
        .order_by_desc(conversations::Column::LastMessageAt)
        .order_by_desc(conversations::Column::OpenedAt)
        .limit(CONVERSATIONS_RENDUES_MAX)
        .all(&state.db)
        .await?;

    // Tout ce dont la boucle a besoin, en cinq requêtes plutôt qu'en cinq PAR
    // conversation.
    //
    // Elle interrogeait la base pour chacune : le plan, l'autre personne, sa
    // fiche pour la photo signée, le dernier message et le compte des non-lus.
    // Cent conversations sont rendues au plus — soit jusqu'à CINQ CENTS
    // allers-retours pour ouvrir l'écran des messages.
    let plans_ids: Vec<String> = lignes.iter().map(|l| l.plan_id.clone()).collect();
    let autres_ids: Vec<String> = lignes
        .iter()
        .map(|l| {
            if l.host_id == compte.id {
                l.guest_id.clone()
            } else {
                l.host_id.clone()
            }
        })
        .collect();
    let conversations_ids: Vec<String> = lignes.iter().map(|l| l.id.clone()).collect();

    let plans: std::collections::HashMap<String, plans::Model> = plans::Entity::find()
        .filter(plans::Column::Id.is_in(plans_ids))
        .all(&state.db)
        .await?
        .into_iter()
        .map(|p| (p.id.clone(), p))
        .collect();

    let autres: std::collections::HashMap<String, accounts::Model> = accounts::Entity::find()
        .filter(accounts::Column::Id.is_in(autres_ids.clone()))
        .all(&state.db)
        .await?
        .into_iter()
        .map(|a| (a.id.clone(), a))
        .collect();

    let photos = photos_signees(&state, &autres_ids).await?;

    // Le dernier message de chacune, en une requête.
    //
    // `lastMessageAt` est déjà porté par la conversation : il suffit de
    // demander les messages qui tombent à cette date-là. Deux messages d'une
    // MÊME conversation à la même milliseconde en rendraient un des deux —
    // c'est l'aperçu d'une liste, et cela n'est pas arrivé.
    let dates: Vec<chrono::NaiveDateTime> =
        lignes.iter().filter_map(|l| l.last_message_at).collect();
    let mut derniers: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if !dates.is_empty() {
        for (conversation, corps) in messages::Entity::find()
            .filter(messages::Column::ConversationId.is_in(conversations_ids.clone()))
            .filter(messages::Column::SentAt.is_in(dates))
            .select_only()
            .column(messages::Column::ConversationId)
            .column(messages::Column::Body)
            .into_tuple::<(String, String)>()
            .all(&state.db)
            .await?
        {
            derniers.entry(conversation).or_insert(corps);
        }
    }

    // Les non-lus, groupés en une seule requête.
    let mut non_lus_par_conversation: std::collections::HashMap<String, i64> =
        std::collections::HashMap::new();
    for (conversation, nombre) in messages::Entity::find()
        .filter(messages::Column::ConversationId.is_in(conversations_ids))
        .filter(messages::Column::AuthorId.ne(compte.id.as_str()))
        .filter(messages::Column::ReadAt.is_null())
        .select_only()
        .column(messages::Column::ConversationId)
        .column_as(messages::Column::Id.count(), "nombre")
        .group_by(messages::Column::ConversationId)
        .into_tuple::<(String, i64)>()
        .all(&state.db)
        .await?
    {
        non_lus_par_conversation.insert(conversation, nombre);
    }

    let mut rendues = Vec::with_capacity(lignes.len());
    for ligne in &lignes {
        let Some(plan) = plans.get(&ligne.plan_id) else {
            continue;
        };

        // « L'autre » : celui des deux qui n'est pas moi.
        let autre_id = if ligne.host_id == compte.id {
            &ligne.guest_id
        } else {
            &ligne.host_id
        };
        let Some(autre) = autres.get(autre_id.as_str()) else {
            continue;
        };

        let dernier: Option<&String> = derniers.get(&ligne.id);
        let non_lus = non_lus_par_conversation
            .get(&ligne.id)
            .copied()
            .unwrap_or(0);

        rendues.push(json!({
            "id": ligne.id,
            "planId": ligne.plan_id,
            "planTitle": plan.title,
            "planStartsAt": iso8601(plan.starts_at.and_utc()),
            "other": {
                "id": autre.id,
                "displayName": autre.display_name,
                "age": age_depuis(autre.birth_date.and_utc(), Utc::now()),
                "photoUrl": photos.get(autre.id.as_str()).cloned().flatten(),
                "verified": autre.verified,
            },
            "lastMessage": dernier,
            "lastMessageAt": ligne.last_message_at.map(|d| iso8601(d.and_utc())),
            "unread": non_lus,
            "closed": ligne.closed_at.is_some(),
        }));
    }

    Ok(Json(Value::Array(rendues)))
}

/// Les photos signées de plusieurs comptes, en une requête.
///
/// Elle remplace une version unitaire, appelée dans la boucle de la liste des
/// conversations : une requête de plus par ligne, pour une signature qui ne
/// coûte rien à calculer une fois les clés en main.
async fn photos_signees(
    state: &AppState,
    comptes: &[String],
) -> Result<std::collections::HashMap<String, Option<String>>, AppError> {
    if comptes.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    Ok(profiles::Entity::find()
        .filter(profiles::Column::AccountId.is_in(comptes.to_vec()))
        .all(&state.db)
        .await?
        .into_iter()
        .map(|p| {
            let url = p.photo_key.map(|cle| {
                signer_url_media(
                    &state.config.media.base_url,
                    &state.config.media.signing_secret,
                    &cle,
                    state.config.media.ttl_url_signee_secondes,
                    0,
                )
            });
            (p.account_id, url)
        })
        .collect())
}

#[derive(Deserialize)]
struct AvantQuand {
    /// Curseur de pagination : ne rendre que ce qui précède cette date.
    before: Option<String>,
}

/// Lire une conversation.
///
/// L'ouvrir marque comme lus les messages de l'autre. Ce marquage n'est jamais
/// exposé à l'expéditeur : il ne sert qu'au compteur local et à la Live
/// Activity.
async fn lire(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Path(id): Path<String>,
    Query(page): Query<AvantQuand>,
) -> Result<Json<Value>, AppError> {
    let conversation = conversation_de(&state, &id, &compte.id).await?;

    let mut requete = messages::Entity::find()
        .filter(messages::Column::ConversationId.eq(conversation.id.as_str()));

    if let Some(avant) = &page.before {
        let borne =
            DateTime::parse_from_rfc3339(avant).map_err(|_| invalide(Msg::CurseurAvantIso8601))?;
        requete = requete.filter(messages::Column::SentAt.lt(borne.naive_utc()));
    }

    // Tirés du plus récent — c'est ce que veut une pagination vers le passé —
    // puis remis dans l'ordre de lecture.
    let mut lignes = requete
        .order_by_desc(messages::Column::SentAt)
        .limit(PAGE)
        .all(&state.db)
        .await?;
    lignes.reverse();

    let rendus: Vec<Value> = lignes
        .iter()
        .map(|ligne| {
            json!({
                "id": ligne.id,
                "conversationId": ligne.conversation_id,
                "author": if ligne.author_id == compte.id { "moi" } else { "autre" },
                "body": ligne.body,
                "sentAt": iso8601(ligne.sent_at.and_utc()),
                "readAt": ligne.read_at.map(|d| iso8601(d.and_utc())),
            })
        })
        .collect();

    messages::Entity::update_many()
        .col_expr(
            messages::Column::ReadAt,
            sea_orm::sea_query::Expr::value(Utc::now().naive_utc()),
        )
        .filter(messages::Column::ConversationId.eq(conversation.id.as_str()))
        .filter(messages::Column::AuthorId.ne(compte.id.as_str()))
        .filter(messages::Column::ReadAt.is_null())
        .exec(&state.db)
        .await?;

    Ok(Json(json!({ "messages": rendus })))
}

/// Clore une conversation.
async fn clore(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let conversation = conversation_de(&state, &id, &compte.id).await?;
    // Clore ce qui est clos n'est pas une erreur : le client peut réessayer.
    if conversation.closed_at.is_some() {
        return Ok(Json(json!({ "ok": true })));
    }

    let fermeture = Utc::now().naive_utc();
    let purge = (Utc::now() + Duration::days(RETENTION_MESSAGES_JOURS)).naive_utc();

    state
        .db
        .transaction::<_, (), sea_orm::DbErr>(|tx| {
            let id = conversation.id.clone();
            let ferme_par = compte.id.clone();
            Box::pin(async move {
                let mut close: conversations::ActiveModel =
                    conversations::Entity::find_by_id(id.as_str())
                        .one(tx)
                        .await?
                        .ok_or(sea_orm::DbErr::RecordNotFound(id.clone()))?
                        .into();
                close.closed_at = Set(Some(fermeture));
                close.closed_by = Set(Some(ferme_par));
                close.update(tx).await?;

                // Les messages d'une conversation close ne sont pas gardés
                // indéfiniment : la date de purge est posée dès la clôture.
                messages::Entity::update_many()
                    .col_expr(
                        messages::Column::PurgeAfter,
                        sea_orm::sea_query::Expr::value(purge),
                    )
                    .filter(messages::Column::ConversationId.eq(id.as_str()))
                    .exec(tx)
                    .await?;
                Ok(())
            })
        })
        .await
        .map_err(|erreur| {
            tracing::error!(erreur = %erreur, "clôture de conversation impossible");
            AppError::new(Code::Internal, Msg::ErreurInterne.t())
        })?;

    live_activity::publier_au_mieux(&state, &compte.id).await;

    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct NouveauMessage {
    body: String,
}

async fn ecrire(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
    Path(id): Path<String>,
    Json(corps): Json<NouveauMessage>,
) -> Result<Json<Value>, AppError> {
    consommer(&state, REGLE_MESSAGE, &compte.id).await?;

    let conversation = conversation_de(&state, &id, &compte.id).await?;
    if conversation.closed_at.is_some() {
        return Err(invalide(Msg::ConversationClose));
    }

    let texte = corps.body.trim().to_string();
    if texte.is_empty() {
        return Err(invalide(Msg::MessageVide));
    }
    if texte.chars().count() > MESSAGE_MAX {
        return Err(invalide(Msg::MessageDeDemandeTropLong {
            maximum: MESSAGE_MAX as i64,
        }));
    }

    // Les deux écritures vont ensemble : une conversation dont `lastMessageAt`
    // ne suivrait pas remonterait au mauvais rang dans la liste.
    let transaction = state.db.begin().await?;

    let envoye = Utc::now();
    let message = messages::ActiveModel {
        id: Set(cuid2::create_id()),
        conversation_id: Set(conversation.id.clone()),
        author_id: Set(compte.id.clone()),
        body: Set(texte),
        sent_at: Set(envoye.naive_utc()),
        ..Default::default()
    }
    .insert(&transaction)
    .await?;

    let destinataire = if conversation.host_id == compte.id {
        conversation.guest_id.clone()
    } else {
        conversation.host_id.clone()
    };

    let mut maj: conversations::ActiveModel = conversation.into();
    maj.last_message_at = Set(Some(message.sent_at));
    maj.update(&transaction).await?;

    transaction.commit().await?;

    // C'est le destinataire dont l'écran change, pas l'expéditeur.
    live_activity::publier_au_mieux(&state, &destinataire).await;

    // La Live Activity ne compte que des plans et des demandes : un message ne
    // l'anime pas, et sans alerte le destinataire ne l'apprenait qu'en ouvrant
    // l'application. L'alerte ne cite ni le message ni son auteur.
    crate::alerte::prevenir_message(&state, &destinataire).await;

    Ok(Json(json!({
        "id": message.id,
        "conversationId": message.conversation_id,
        "author": "moi",
        "body": message.body,
        "sentAt": iso8601(message.sent_at.and_utc()),
        // Weave ne pose pas d'accusé de lecture : le champ existe pour le
        // contrat, il reste nul.
        "readAt": null,
    })))
}

/// Charge une conversation en vérifiant qu'on en fait partie.
async fn conversation_de(
    state: &AppState,
    id: &str,
    compte_id: &str,
) -> Result<conversations::Model, AppError> {
    let conversation = conversations::Entity::find_by_id(id.to_string())
        .one(&state.db)
        .await?
        .ok_or_else(|| introuvable(Msg::ConversationIntrouvable))?;

    if conversation.host_id != compte_id && conversation.guest_id != compte_id {
        return Err(AppError::new(
            Code::Forbidden,
            Msg::ConversationPasLaVotre.t(),
        ));
    }
    Ok(conversation)
}
