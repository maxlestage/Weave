//! Export de ses propres données — article 20 du RGPD.
//!
//! « Format structuré, couramment utilisé et lisible par machine » : du JSON,
//! en un seul document, sans pagination. Une personne qui exerce ce droit veut
//! un fichier, pas une interface à parcourir.
//!
//! Ce qui en est exclu, et pourquoi :
//!
//! * **Les secrets.** Empreinte de l'adresse, empreintes des jetons et des
//!   codes : ce ne sont pas ses données, ce sont les nôtres à son sujet, et les
//!   rendre affaiblirait le compte sans rien lui apprendre.
//! * **Les messages reçus.** Un message écrit par quelqu'un d'autre est aussi
//!   la donnée de cette personne. L'export contient donc ce que l'on a écrit,
//!   et la liste des conversations — pas les mots des autres.
//! * **Les signalements que l'on a reçus.** Les rendre livrerait qui a signalé,
//!   ou permettrait de le déduire. La modération ne serait plus sûre pour
//!   personne.
//!
//! Ce qu'il contient, en revanche, va au-delà du strict nécessaire : les
//! consentements avec leur date, les appareils, les achats. C'est ce qui permet
//! de vérifier ce que le service sait, ce qui est le point du droit d'accès.

use crate::messages::Msg;
use crate::{
    AppState,
    auth::Authentifie,
    entities::{
        accounts, blocks, consent_records, conversations, credit_balances, devices, join_requests,
        media_objects, messages, plans, preferences, profiles, reports, subscriptions,
        unit_purchases,
    },
    error::{AppError, non_autorise},
    limitation::{consommer, regles},
    temps::iso8601,
};
use axum::{Json, Router, extract::State, response::IntoResponse, routing::get};
use base64::{Engine, engine::general_purpose::STANDARD};
use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder};
use serde_json::{Value, json};

pub fn routes() -> Router<AppState> {
    Router::new().route("/v1/me/export", get(exporter))
}

/// Rassemble tout ce que le service détient sur un compte.
///
/// Les requêtes sont séquentielles et non bornées : l'export est une opération
/// rare, et un compte Weave ne porte pas de volumétrie. Le paginer
/// compliquerait la seule chose qu'on attend de lui — être complet.
async fn exporter(
    State(state): State<AppState>,
    Authentifie(compte): Authentifie,
) -> Result<impl IntoResponse, AppError> {
    // La lecture la plus lourde du service, accessible à tout compte connecté
    // et bornée par rien. La borne est large : le droit d'accès ne se refuse
    // pas, il se protège d'une boucle.
    consommer(&state, regles::EXPORT, &compte.id).await?;

    let id = compte.id.as_str();

    let ligne = accounts::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| non_autorise(Msg::SessionExpiree))?;

    let fiche = profiles::Entity::find()
        .filter(profiles::Column::AccountId.eq(id))
        .one(&state.db)
        .await?;

    // Les octets de la photo, joints à l'export plutôt que désignés par une
    // clé. `None` quand il n'y en a pas, ou quand l'objet a disparu — auquel
    // cas mieux vaut rien qu'une clé qui ne mène nulle part.
    let photo = match fiche.as_ref().and_then(|f| f.photo_key.clone()) {
        Some(cle) => media_objects::Entity::find_by_id(cle)
            .one(&state.db)
            .await?
            .map(|objet| {
                json!({
                    "typeDeContenu": objet.content_type,
                    "octets": objet.byte_size,
                    // En base64 standard, tel qu'un `data:` URI l'attend : le
                    // fichier s'ouvre en le collant dans un navigateur.
                    "donneesBase64": STANDARD.encode(&objet.bytes),
                })
            }),
        None => None,
    };

    let criteres = preferences::Entity::find()
        .filter(preferences::Column::AccountId.eq(id))
        .one(&state.db)
        .await?;

    let mes_plans = plans::Entity::find()
        .filter(plans::Column::AuthorId.eq(id))
        .order_by_asc(plans::Column::CreatedAt)
        .all(&state.db)
        .await?;

    let mes_demandes = join_requests::Entity::find()
        .filter(join_requests::Column::AuthorId.eq(id))
        .order_by_asc(join_requests::Column::SentAt)
        .all(&state.db)
        .await?;

    let mes_conversations = conversations::Entity::find()
        .filter(
            Condition::any()
                .add(conversations::Column::HostId.eq(id))
                .add(conversations::Column::GuestId.eq(id)),
        )
        .order_by_asc(conversations::Column::OpenedAt)
        .all(&state.db)
        .await?;

    let mes_messages = messages::Entity::find()
        .filter(messages::Column::AuthorId.eq(id))
        .order_by_asc(messages::Column::SentAt)
        .all(&state.db)
        .await?;

    let mes_blocages = blocks::Entity::find()
        .filter(blocks::Column::AuthorId.eq(id))
        .all(&state.db)
        .await?;

    let mes_signalements = reports::Entity::find()
        .filter(reports::Column::AuthorId.eq(id))
        .all(&state.db)
        .await?;

    let consentements = consent_records::Entity::find()
        .filter(consent_records::Column::AccountId.eq(id))
        .order_by_asc(consent_records::Column::GrantedAt)
        .all(&state.db)
        .await?;

    let appareils = devices::Entity::find()
        .filter(devices::Column::AccountId.eq(id))
        .all(&state.db)
        .await?;

    let abonnements = subscriptions::Entity::find()
        .filter(subscriptions::Column::AccountId.eq(id))
        .all(&state.db)
        .await?;

    let achats = unit_purchases::Entity::find()
        .filter(unit_purchases::Column::AccountId.eq(id))
        .all(&state.db)
        .await?;

    let credits = credit_balances::Entity::find()
        .filter(credit_balances::Column::AccountId.eq(id))
        .all(&state.db)
        .await?;

    let document = json!({
        "exportedAt": iso8601(chrono::Utc::now()),
        "format": "weave.export.v1",
        "notice": concat!(
            "Export au titre de l'article 20 du RGPD. Il contient ce que vous ",
            "avez écrit et ce que le service sait de vous. Il ne contient pas ",
            "les messages écrits par d'autres, ni l'identité de qui vous aurait ",
            "signalé : ce sont les données de ces personnes."
        ),
        "compte": {
            "id": ligne.id,
            "email": ligne.email,
            "pseudonyme": ligne.handle,
            "nomAffiche": ligne.display_name,
            "dateDeNaissance": ligne.birth_date,
            "statut": ligne.status,
            "fuseau": ligne.timezone,
            "langue": ligne.locale,
            "verifie": ligne.verified,
            "vuLe": ligne.last_seen_at.map(|quand| iso8601(quand.and_utc())),
            "suppressionDemandeeLe": ligne.deletion_requested_at.map(|quand| iso8601(quand.and_utc())),
            "creeLe": iso8601(ligne.created_at.and_utc()),
        },
        "fiche": fiche.map(|f| json!({
            "ville": f.city,
            "latitudeArrondie": f.lat_rounded,
            "longitudeArrondie": f.lon_rounded,
            "genre": f.gender,
            "presentation": f.bio,
            // La photo est jointe entière, en clair.
            //
            // L'export rendait `photoKey` — un identifiant opaque qui ne
            // désigne rien d'accessible à qui le reçoit. L'article 20 demande
            // un format exploitable : une clé ne l'est pas, et une URL signée
            // ne le serait pas davantage, puisqu'elle expire en quelques
            // minutes alors qu'un export se garde.
            "photo": photo,
            "creeLe": iso8601(f.created_at.and_utc()),
        })),
        "criteres": criteres.map(|c| json!({
            "ageMinimum": c.min_age,
            "ageMaximum": c.max_age,
            "distanceMaximaleKm": c.max_distance_km,
            // Les listes sont rendues comme listes, pas comme du JSON dans
            // une chaîne. Elles sont stockées en texte — le schéma est partagé
            // avec SQLite, qui n'a pas de type tableau — et les recopier telles
            // quelles donnait « "[\"femme\"]" » à relire à la main.
            "recherche": liste(&c.seeking_json),
            "categories": liste(&c.categories_json),
            "escaleVille": c.escale_city,
            "escaleJusquA": c.escale_until.map(|quand| iso8601(quand.and_utc())),
        })),
        "plans": mes_plans.into_iter().map(|p| json!({
            "intitule": p.title,
            "note": p.note,
            "categorie": p.category,
            "commenceLe": iso8601(p.starts_at.and_utc()),
            "ville": p.city,
            "places": p.capacity,
            "etat": p.state,
            "creeLe": iso8601(p.created_at.and_utc()),
        })).collect::<Vec<Value>>(),
        "demandesEnvoyees": mes_demandes.into_iter().map(|d| json!({
            "planId": d.plan_id,
            "message": d.message,
            "etat": d.state,
            "envoyeeLe": iso8601(d.sent_at.and_utc()),
            "decideeLe": d.decided_at.map(|quand| iso8601(quand.and_utc())),
        })).collect::<Vec<Value>>(),
        "conversations": mes_conversations.into_iter().map(|c| json!({
            "id": c.id,
            "planId": c.plan_id,
            "role": if c.host_id == id { "hote" } else { "invite" },
            "ouverteLe": iso8601(c.opened_at.and_utc()),
            "fermeeLe": c.closed_at.map(|quand| iso8601(quand.and_utc())),
        })).collect::<Vec<Value>>(),
        "messagesEnvoyes": mes_messages.into_iter().map(|m| json!({
            "conversationId": m.conversation_id,
            "texte": m.body,
            "envoyeLe": iso8601(m.sent_at.and_utc()),
        })).collect::<Vec<Value>>(),
        "blocages": mes_blocages.into_iter().map(|b| json!({
            "compteBloque": b.target_id,
            "le": iso8601(b.created_at.and_utc()),
        })).collect::<Vec<Value>>(),
        "signalementsEnvoyes": mes_signalements.into_iter().map(|r| json!({
            "motif": r.reason,
            "details": r.details,
            "etat": r.state,
            "le": iso8601(r.created_at.and_utc()),
        })).collect::<Vec<Value>>(),
        "consentements": consentements.into_iter().map(|c| json!({
            "objet": c.kind,
            "version": c.version,
            "accorde": c.granted,
            "accordeLe": iso8601(c.granted_at.and_utc()),
            "retireLe": c.revoked_at.map(|quand| iso8601(quand.and_utc())),
        })).collect::<Vec<Value>>(),
        "appareils": appareils.into_iter().map(|d| json!({
            "plateforme": d.platform,
            "modele": d.model,
            "versionSysteme": d.os_version,
            "versionApplication": d.app_version,
            "vuLe": iso8601(d.last_seen_at.and_utc()),
            "ajouteLe": iso8601(d.created_at.and_utc()),
        })).collect::<Vec<Value>>(),
        "abonnements": abonnements.into_iter().map(|s| json!({
            "palier": s.tier,
            "periode": s.period,
            "produit": s.store_kit_product_id,
            "renouvelleLe": s.renews_at.map(|quand| iso8601(quand.and_utc())),
            "expireLe": s.expires_at.map(|quand| iso8601(quand.and_utc())),
            "resilieLe": s.cancelled_at.map(|quand| iso8601(quand.and_utc())),
        })).collect::<Vec<Value>>(),
        "achats": achats.into_iter().map(|a| json!({
            "produit": a.sku,
            "quantite": a.quantity,
            "prixCentimes": a.price_cents,
            "devise": a.currency,
            "acheteLe": iso8601(a.purchased_at.and_utc()),
            "rembourseLe": a.refunded_at.map(|quand| iso8601(quand.and_utc())),
        })).collect::<Vec<Value>>(),
        "credits": credits.into_iter().map(|c| json!({
            "produit": c.sku,
            "solde": c.balance,
            "remisAZeroLe": c.resets_at.map(|quand| iso8601(quand.and_utc())),
        })).collect::<Vec<Value>>(),
    });

    // Le nom de fichier compte : sans lui, le navigateur affiche le JSON au
    // lieu de l'enregistrer, et le droit à la portabilité suppose un fichier
    // qu'on emporte.
    Ok((
        [(
            axum::http::header::CONTENT_DISPOSITION,
            "attachment; filename=\"weave-mes-donnees.json\"",
        )],
        Json(document),
    ))
}

/// Relit une liste stockée en texte JSON.
///
/// Les colonnes de listes sont du texte : le schéma est partagé avec SQLite,
/// qui n'a pas de type tableau. Une colonne illisible vaut « aucun » plutôt
/// que de faire échouer tout l'export — quelqu'un qui demande ses données doit
/// les recevoir, même si l'une d'elles est abîmée.
fn liste(brut: &str) -> Vec<String> {
    serde_json::from_str(brut).unwrap_or_default()
}
