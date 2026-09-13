//! Le consentement de l'article 9 : demandé à part, retirable, et daté.
//!
//! La page « Confidentialité » fait trois promesses. Un fichier par promesse
//! aurait été plus joli ; elles se tiennent, et se cassent, ensemble.

use super::{refuse, Service};
use crate::routes::consentements::{DONNEES_SENSIBLES, VERSION_POLITIQUE};
use axum::http::StatusCode;
use serde_json::json;

fn consentement(corps: &serde_json::Value) -> &serde_json::Value {
    corps["consents"]
        .as_array()
        .expect("la liste entière, toujours")
        .iter()
        .find(|c| c["kind"] == DONNEES_SENSIBLES)
        .expect("l'objet figure même sans consentement")
}

/// « Jamais demandé » est une réponse, pas un trou.
#[tokio::test]
async fn un_compte_neuf_n_a_consenti_a_rien_et_le_dit() {
    let service = Service::monter().await;
    service.compte("c_neuf", "escapade").await;
    let jeton = service.jeton("c_neuf");

    let (statut, corps) = service.get("/v1/me/consents", Some(&jeton)).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["policyVersion"], VERSION_POLITIQUE);

    let objet = consentement(&corps);
    assert_eq!(objet["active"], false);
    assert!(objet["grantedAt"].is_null(), "rien n'a été accordé");
}

/// Le cœur de l'affaire : le critère de genre ne s'enregistre pas sans
/// consentement.
///
/// Il s'enregistrait, jusqu'ici, sans qu'aucun consentement n'ait jamais été
/// demandé ni conservé — un traitement de catégorie particulière sans la base
/// légale que l'article 9 exige, et que la page publique promet.
#[tokio::test]
async fn chercher_par_genre_exige_le_consentement() {
    let service = Service::monter().await;
    service.compte("c_art9", "escapade").await;
    let jeton = service.jeton("c_art9");

    let (statut, corps) = service
        .patch("/v1/me/preferences", Some(&jeton), json!({ "seeking": ["homme"] }))
        .await;
    assert_eq!(statut, StatusCode::FORBIDDEN, "accepté sans consentement : {corps}");
    assert!(
        corps["message"].as_str().unwrap_or_default().contains("Confidentialité"),
        "le refus doit dire où le donner : {corps}"
    );

    service.consentir(&jeton).await;

    let (statut, corps) = service
        .patch("/v1/me/preferences", Some(&jeton), json!({ "seeking": ["homme"] }))
        .await;
    assert_eq!(statut, StatusCode::OK, "refusé après consentement : {corps}");
}

/// Les autres critères ne sont pas sensibles, et ne doivent rien demander.
///
/// C'est ce que veut dire « distinct » : un consentement exigé pour tout
/// vaudrait une case unique valant acceptation de tout le reste, que la page
/// publique exclut nommément.
#[tokio::test]
async fn les_autres_criteres_ne_demandent_aucun_consentement() {
    let service = Service::monter().await;
    service.compte("c_autres", "escapade").await;
    let jeton = service.jeton("c_autres");

    let (statut, corps) = service
        .patch(
            "/v1/me/preferences",
            Some(&jeton),
            json!({ "minAge": 25, "maxDistanceKm": 40, "categories": ["balade"] }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
}

/// Retirer arrête le traitement : le critère est effacé, pas seulement ignoré.
///
/// Le laisser en base ferait durer un traitement de données sensibles sans
/// base légale, et la page promet « un fil non filtré sur ce critère ».
#[tokio::test]
async fn retirer_efface_le_critere_et_date_le_retrait() {
    let service = Service::monter().await;
    service.compte("c_retrait", "escapade").await;
    let jeton = service.jeton("c_retrait");

    service.consentir(&jeton).await;
    let (statut, _) = service
        .patch("/v1/me/preferences", Some(&jeton), json!({ "seeking": ["femme"] }))
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (statut, corps) = service
        .post(
            "/v1/me/consents/revoke",
            Some(&jeton),
            json!({ "kind": DONNEES_SENSIBLES }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let (_, criteres) = service.get("/v1/me/preferences", Some(&jeton)).await;
    assert_eq!(
        criteres["seeking"],
        json!([]),
        "le critère survit au retrait : {criteres}"
    );

    let (_, consents) = service.get("/v1/me/consents", Some(&jeton)).await;
    let objet = consentement(&consents);
    assert_eq!(objet["active"], false);
    assert!(
        !objet["revokedAt"].is_null(),
        "le retrait est enregistré avec sa date : {consents}"
    );
    assert!(
        !objet["grantedAt"].is_null(),
        "la période d'activité est ce qui établit que le traitement était licite"
    );
}

/// Le service continue de fonctionner après un retrait — c'est la promesse
/// exacte de la page, et un retrait qui coûterait le compte n'en serait pas un.
#[tokio::test]
async fn le_service_marche_encore_apres_un_retrait() {
    let service = Service::monter().await;
    service.compte("c_apres", "escapade").await;
    let jeton = service.jeton("c_apres");
    fiche(&service, "c_apres", "femme").await;

    service.consentir(&jeton).await;
    service
        .post(
            "/v1/me/consents/revoke",
            Some(&jeton),
            json!({ "kind": DONNEES_SENSIBLES }),
        )
        .await;

    let (statut, corps) = service.get("/v1/plans", Some(&jeton)).await;
    assert_eq!(statut, StatusCode::OK, "le fil a cessé de répondre : {corps}");

    // Et vider une liste reste possible sans consentement : sinon le retrait
    // buterait sur son propre effet.
    let (statut, corps) = service
        .patch("/v1/me/preferences", Some(&jeton), json!({ "seeking": [] }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
}

/// Redonner un consentement déjà actif ne raccourcit pas la période établie.
#[tokio::test]
async fn consentir_deux_fois_ne_reecrit_pas_la_date_d_octroi() {
    let service = Service::monter().await;
    service.compte("c_deux", "escapade").await;
    let jeton = service.jeton("c_deux");

    service.consentir(&jeton).await;
    let (_, premier) = service.get("/v1/me/consents", Some(&jeton)).await;
    let date = consentement(&premier)["grantedAt"].clone();

    service.consentir(&jeton).await;
    let (_, second) = service.get("/v1/me/consents", Some(&jeton)).await;
    assert_eq!(
        consentement(&second)["grantedAt"], date,
        "la date d'octroi est celle du premier oui"
    );
}

/// Consentir « à la politique » sans dire laquelle ne prouve rien.
#[tokio::test]
async fn un_consentement_doit_porter_la_version_en_vigueur() {
    let service = Service::monter().await;
    service.compte("c_version", "escapade").await;
    let jeton = service.jeton("c_version");

    for version in [json!("2019-01-01"), json!(null)] {
        let (statut, corps) = service
            .post(
                "/v1/me/consents",
                Some(&jeton),
                json!({ "kind": DONNEES_SENSIBLES, "version": version }),
            )
            .await;
        refuse(statut, &corps, "validation", &format!("version « {version} » acceptée : {corps}"));
    }

    let (statut, corps) = service
        .post(
            "/v1/me/consents",
            Some(&jeton),
            json!({ "kind": "tout_et_n_importe_quoi", "version": VERSION_POLITIQUE }),
        )
        .await;
    refuse(statut, &corps, "validation", &format!("objet inconnu accepté : {corps}"));
}

/// Un consentement donné sur une version antérieure cesse de valoir, et le fil
/// cesse de filtrer dessus — sans qu'aucune écriture ne repasse sur la ligne.
///
/// C'est le seul chemin par lequel un critère de genre peut rester en base
/// alors que le consentement ne vaut plus : le retrait, lui, l'efface. Si le
/// fil ne relisait pas le consentement, le traitement continuerait jusqu'à ce
/// que la personne touche à ses critères — c'est-à-dire peut-être jamais.
#[tokio::test]
async fn un_consentement_perime_cesse_de_filtrer_le_fil() {
    use crate::entities::consent_records;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let service = Service::monter().await;
    let compte = service.compte("c_perime", "escapade").await;
    let jeton = service.jeton("c_perime");
    fiche(&service, "c_perime", "femme").await;

    // Un plan publié par un homme : exclu tant que l'on ne cherche que des
    // femmes, et c'est sa présence ou son absence qui dit si le fil filtre.
    service.compte("c_auteur_h", "escapade").await;
    fiche(&service, "c_auteur_h", "homme").await;
    let plan = plan_de(&service, "c_auteur_h").await;

    service.consentir(&jeton).await;
    let (statut, corps) = service
        .patch("/v1/me/preferences", Some(&jeton), json!({ "seeking": ["femme"] }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    assert!(
        !fil_contient(&service, &compte, &jeton, &plan).await,
        "le fil devrait filtrer tant que le consentement vaut"
    );

    // Le texte change : la version enregistrée n'est plus celle en vigueur.
    consent_records::Entity::update_many()
        .col_expr(
            consent_records::Column::Version,
            sea_orm::sea_query::Expr::value("1999-01-01"),
        )
        .filter(consent_records::Column::AccountId.eq(compte.as_str()))
        .exec(&service.db)
        .await
        .expect("version modifiée");

    let (_, corps) = service.get("/v1/me/consents", Some(&jeton)).await;
    assert_eq!(
        consentement(&corps)["active"],
        false,
        "un texte accepté il y a deux versions ne dit plus ce qui a été accepté : {corps}"
    );

    // Le critère est TOUJOURS en base — rien ne l'a effacé.
    let (_, criteres) = service.get("/v1/me/preferences", Some(&jeton)).await;
    assert_eq!(criteres["seeking"], json!(["femme"]));

    // Et pourtant le fil ne filtre plus dessus.
    assert!(
        fil_contient(&service, &compte, &jeton, &plan).await,
        "le traitement a continué après la péremption du consentement"
    );
}

async fn fiche(service: &Service, nom: &str, genre: &str) {
    let (statut, corps) = service
        .put(
            "/v1/me/profile",
            Some(&service.jeton(nom)),
            json!({
                "city": "Lyon",
                "latitude": 45.76,
                "longitude": 4.84,
                "gender": genre,
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
}

async fn plan_de(service: &Service, nom: &str) -> String {
    let (statut, corps) = service
        .post(
            "/v1/plans",
            Some(&service.jeton(nom)),
            json!({
                "title": "Une balade sur les quais",
                "category": "balade",
                "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    corps["id"].as_str().expect("identifiant du plan").to_string()
}

/// Le fil vit quelques minutes en cache : chaque lecture se fait sur un fil
/// oublié, sinon on relirait la composition d'avant le changement.
async fn fil_contient(service: &Service, compte: &str, jeton: &str, plan: &str) -> bool {
    crate::cache::oublier(&service.etat.cache, &crate::cache::cles::fil(compte))
        .await
        .ok();
    let (statut, corps) = service.get("/v1/plans", Some(jeton)).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    corps["plans"]
        .as_array()
        .expect("le fil rend une liste")
        .iter()
        .any(|p| p["id"] == plan)
}
