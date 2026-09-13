//! La session : l'ouvrir, la renouveler, la fermer, s'en aller.
//!
//! Ces routes manquaient au portage. `POST /v1/auth/refresh` à elle seule
//! décide si une session dure : sans elle, le jeton d'accès expire au bout de
//! quinze minutes et rien ne le remplace.

use super::{Service, SECRET};
use crate::auth::emettre_jeton;
use axum::http::StatusCode;
use serde_json::json;

/// Ouvre une session en déroulant le parcours réel, et rend (accès, renouvellement).
async fn session(service: &Service, nom: &str) -> (String, String) {
    let email = &service.email(nom);
    let (statut, corps) = service
        .post("/v1/auth/otp/request", None, json!({ "email": email }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    let code = corps["devCode"].as_str().expect("le code hors production").to_string();

    let (statut, corps) = service
        .post("/v1/auth/otp/verify", None, json!({
            "email": email,
            "code": code,
            "displayName": "Camille",
            "birthDate": "1994-03-08",
        }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    (
        corps["session"]["accessToken"].as_str().unwrap().to_string(),
        corps["session"]["refreshToken"].as_str().unwrap().to_string(),
    )
}

#[tokio::test]
async fn une_session_se_renouvelle() {
    let service = Service::monter().await;
    let (_, renouvellement) = session(&service, "renouvelle").await;

    let (statut, corps) = service
        .post("/v1/auth/refresh", None, json!({ "refreshToken": renouvellement }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    // Le nouvel accès ouvre bien les routes protégées.
    let acces = corps["session"]["accessToken"].as_str().expect("un accès");
    let (statut, _) = service.get("/v1/me", Some(acces)).await;
    assert_eq!(statut, StatusCode::OK, "le jeton renouvelé n'ouvre rien");
}

/// La rotation est ce qui distingue un jeton volé d'un jeton légitime : le
/// légitime n'est présenté qu'une fois. Rejouer l'ancien doit échouer.
#[tokio::test]
async fn un_jeton_de_renouvellement_ne_sert_qu_une_fois() {
    let service = Service::monter().await;
    let (_, renouvellement) = session(&service, "rotation").await;

    let (statut, _) = service
        .post("/v1/auth/refresh", None, json!({ "refreshToken": renouvellement.clone() }))
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (statut, corps) = service
        .post("/v1/auth/refresh", None, json!({ "refreshToken": renouvellement }))
        .await;
    assert_eq!(
        statut,
        StatusCode::UNAUTHORIZED,
        "l'ancien jeton passe encore : {corps}"
    );
}

#[tokio::test]
async fn un_jeton_inconnu_ne_renouvelle_rien() {
    let service = Service::monter().await;
    let (statut, _) = service
        .post("/v1/auth/refresh", None, json!({ "refreshToken": "aucun-jeton-de-ce-nom-nexiste-ici" }))
        .await;
    assert_eq!(statut, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn fermer_une_session_revoque_son_jeton() {
    let service = Service::monter().await;
    let (acces, renouvellement) = session(&service, "ferme").await;

    let (statut, _) = service
        .post("/v1/auth/logout", Some(&acces), json!({ "refreshToken": renouvellement.clone() }))
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (statut, _) = service
        .post("/v1/auth/refresh", None, json!({ "refreshToken": renouvellement }))
        .await;
    assert_eq!(statut, StatusCode::UNAUTHORIZED, "la session fermée renouvelle encore");
}

/// Sans jeton précisé, ce sont toutes les sessions du compte qui tombent :
/// c'est ce qu'attend « se déconnecter partout » après un appareil perdu.
#[tokio::test]
async fn fermer_sans_preciser_revoque_tout() {
    let service = Service::monter().await;
    let (acces, premier) = session(&service, "partout").await;

    // Un second appareil, sur le même compte.
    let (statut, corps) = service
        .post("/v1/auth/refresh", None, json!({ "refreshToken": premier }))
        .await;
    assert_eq!(statut, StatusCode::OK);
    let second = corps["session"]["refreshToken"].as_str().unwrap().to_string();

    let (statut, _) = service.post("/v1/auth/logout", Some(&acces), json!({})).await;
    assert_eq!(statut, StatusCode::OK);

    let (statut, _) = service
        .post("/v1/auth/refresh", None, json!({ "refreshToken": second }))
        .await;
    assert_eq!(statut, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn fermer_une_session_exige_d_etre_connecte() {
    let service = Service::monter().await;
    let (statut, _) = service.post("/v1/auth/logout", None, json!({})).await;
    assert_eq!(statut, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn supprimer_son_compte_le_sort_de_la_circulation() {
    let service = Service::monter().await;
    let compte = service.compte("c_partant", "depart").await;
    let acces = emettre_jeton(SECRET, &compte, 900).expect("jeton émis");

    // Un plan ouvert, et une demande en attente dessus.
    let (statut, plan) = service
        .post("/v1/plans", Some(&acces), json!({
            "title": "Une balade que personne ne fera",
            "category": "balade",
            "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
        }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{plan}");
    let plan_id = plan["id"].as_str().unwrap().to_string();

    let invite = service.compte("c_invite_partant", "depart").await;
    let jeton_invite = emettre_jeton(SECRET, &invite, 900).expect("jeton émis");
    let (statut, _) = service
        .post("/v1/requests", Some(&jeton_invite), json!({
            "planId": plan_id,
            "message": "Cette balade me tente beaucoup, je serais ravi de venir.",
        }))
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (statut, corps) = service.delete("/v1/auth/account", Some(&acces)).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["purgeAfterDays"], 30);

    let etat: Option<(String, String)> = {
        use sea_orm::{ConnectionTrait, Statement};
        let ligne = service
            .db
            .query_one_raw(Statement::from_string(
                service.db.get_database_backend(),
                format!(
                    "SELECT (SELECT status FROM accounts WHERE id='{compte}') AS c, \
                     (SELECT state FROM plans WHERE id='{plan_id}') AS p"
                ),
            ))
            .await
            .unwrap();
        ligne.map(|l| {
            (
                l.try_get::<String>("", "c").unwrap(),
                l.try_get::<String>("", "p").unwrap(),
            )
        })
    };
    assert_eq!(
        etat,
        Some(("deleting".to_string(), "annule".to_string())),
        "le compte ou son plan est resté en circulation"
    );

    // La demande de l'invité ne doit plus retenir son quota : plus personne ne
    // lui répondra.
    let (statut, corps) = service.get("/v1/me", Some(&jeton_invite)).await;
    assert_eq!(statut, StatusCode::OK);
    assert_eq!(corps["requestsLeftToday"], 4, "quota rendu ou non : {corps}");
}

/// Rejouer un jeton déjà tourné doit couper toutes les sessions du compte.
///
/// C'est la raison d'être de la rotation, et elle ne tenait pas : `rotatedTo`
/// était écrit à chaque renouvellement et jamais relu. Un jeton volé
/// fonctionnait donc jusqu'à son expiration, et le vol ne se voyait nulle part.
#[tokio::test]
async fn rejouer_un_jeton_deja_tourne_coupe_toutes_les_sessions() {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let service = Service::monter().await;
    let (_, premier) = session(&service, "vole").await;

    // Le porteur légitime renouvelle : `premier` est tourné.
    let (statut, corps) = service
        .post("/v1/auth/refresh", None, json!({ "refreshToken": premier }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    let second = corps["session"]["refreshToken"].as_str().unwrap().to_string();

    // Le voleur rejoue la copie qu'il détient.
    let (statut, _) = service
        .post("/v1/auth/refresh", None, json!({ "refreshToken": premier }))
        .await;
    assert_eq!(statut, StatusCode::UNAUTHORIZED, "un jeton tourné ne vaut plus");

    // Et le jeton du porteur légitime ne vaut plus rien non plus : on ne sait
    // pas lequel des deux est le voleur, donc on coupe tout.
    let (statut, _) = service
        .post("/v1/auth/refresh", None, json!({ "refreshToken": second }))
        .await;
    assert_eq!(
        statut,
        StatusCode::UNAUTHORIZED,
        "le réemploi doit couper la famille entière, pas seulement la copie rejouée"
    );

    // L'incident est consigné — la table d'audit servait à cela.
    let traces = crate::entities::audit_events::Entity::find()
        .filter(crate::entities::audit_events::Column::Action.eq("refresh_reuse"))
        .all(&service.db)
        .await
        .expect("lecture du journal");
    assert_eq!(traces.len(), 1, "le réemploi doit laisser une trace");
    // Le compte est créé par le parcours réel : son identifiant est engendré,
    // pas celui que le test nomme. Ce qui compte est qu'il soit consigné.
    assert!(traces[0].account_id.is_some(), "la trace désigne un compte");
    assert!(traces[0].ip.is_some(), "la trace porte l'adresse d'origine");
    assert!(
        traces[0].meta_json.contains("sessionsRevoquees"),
        "la trace dit combien de sessions ont été coupées"
    );
}

/// Deux renouvellements du même jeton ne doivent ouvrir qu'une session.
#[tokio::test]
async fn un_jeton_ne_se_renouvelle_qu_une_fois() {
    let service = Service::monter().await;
    let (_, jeton) = session(&service, "course").await;

    let corps = json!({ "refreshToken": jeton });
    let (a, b) = tokio::join!(
        service.post("/v1/auth/refresh", None, corps.clone()),
        service.post("/v1/auth/refresh", None, corps.clone()),
    );

    let reussites = [a.0, b.0].iter().filter(|s| s.is_success()).count();
    assert_eq!(reussites, 1, "un jeton, une rotation — obtenu {reussites}");
}
