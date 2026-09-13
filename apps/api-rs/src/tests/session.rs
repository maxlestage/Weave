//! La session : l'ouvrir, la renouveler, la fermer, s'en aller.
//!
//! Ces routes manquaient au portage. `POST /v1/auth/refresh` à elle seule
//! décide si une session dure : sans elle, le jeton d'accès expire au bout de
//! quinze minutes et rien ne le remplace.

use super::{SECRET, Service};
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
    let code = corps["devCode"]
        .as_str()
        .expect("le code hors production")
        .to_string();

    let (statut, corps) = service
        .post(
            "/v1/auth/otp/verify",
            None,
            json!({
                "email": email,
                "code": code,
                "displayName": "Camille",
                "birthDate": "1994-03-08",
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    (
        corps["session"]["accessToken"]
            .as_str()
            .unwrap()
            .to_string(),
        corps["session"]["refreshToken"]
            .as_str()
            .unwrap()
            .to_string(),
    )
}

#[tokio::test]
async fn une_session_se_renouvelle() {
    let service = Service::monter().await;
    let (_, renouvellement) = session(&service, "renouvelle").await;

    let (statut, corps) = service
        .post(
            "/v1/auth/refresh",
            None,
            json!({ "refreshToken": renouvellement }),
        )
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
        .post(
            "/v1/auth/refresh",
            None,
            json!({ "refreshToken": renouvellement.clone() }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (statut, corps) = service
        .post(
            "/v1/auth/refresh",
            None,
            json!({ "refreshToken": renouvellement }),
        )
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
        .post(
            "/v1/auth/refresh",
            None,
            json!({ "refreshToken": "aucun-jeton-de-ce-nom-nexiste-ici" }),
        )
        .await;
    assert_eq!(statut, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn fermer_une_session_revoque_son_jeton() {
    let service = Service::monter().await;
    let (acces, renouvellement) = session(&service, "ferme").await;

    let (statut, _) = service
        .post(
            "/v1/auth/logout",
            Some(&acces),
            json!({ "refreshToken": renouvellement.clone() }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (statut, _) = service
        .post(
            "/v1/auth/refresh",
            None,
            json!({ "refreshToken": renouvellement }),
        )
        .await;
    assert_eq!(
        statut,
        StatusCode::UNAUTHORIZED,
        "la session fermée renouvelle encore"
    );
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
    let second = corps["session"]["refreshToken"]
        .as_str()
        .unwrap()
        .to_string();

    let (statut, _) = service
        .post("/v1/auth/logout", Some(&acces), json!({}))
        .await;
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
        .post(
            "/v1/plans",
            Some(&acces),
            json!({
                "title": "Une balade que personne ne fera",
                "category": "balade",
                "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{plan}");
    let plan_id = plan["id"].as_str().unwrap().to_string();

    let invite = service.compte("c_invite_partant", "depart").await;
    let jeton_invite = emettre_jeton(SECRET, &invite, 900).expect("jeton émis");
    let (statut, _) = service
        .post(
            "/v1/requests",
            Some(&jeton_invite),
            json!({
                "planId": plan_id,
                "message": "Cette balade me tente beaucoup, je serais ravi de venir.",
            }),
        )
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
    assert_eq!(
        corps["requestsLeftToday"], 4,
        "quota rendu ou non : {corps}"
    );
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
    let second = corps["session"]["refreshToken"]
        .as_str()
        .unwrap()
        .to_string();

    // Le voleur rejoue la copie qu'il détient.
    let (statut, _) = service
        .post("/v1/auth/refresh", None, json!({ "refreshToken": premier }))
        .await;
    assert_eq!(
        statut,
        StatusCode::UNAUTHORIZED,
        "un jeton tourné ne vaut plus"
    );

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

/// Cinq tentatives, pas une de plus — même lancées ensemble.
///
/// Le compteur était lu, comparé, puis réécrit. N essais simultanés lisaient
/// tous la même valeur et n'en consommaient qu'une : le plafond ne coûtait
/// qu'une tentative, autant de fois qu'on voulait. Un code à six chiffres ne
/// résiste pas à cela.
#[tokio::test]
async fn le_plafond_de_tentatives_tient_meme_en_rafale() {
    let service = Service::monter().await;
    let email = service.email("rafale");

    let (statut, _) = service
        .post("/v1/auth/otp/request", None, json!({ "email": email }))
        .await;
    assert_eq!(statut, StatusCode::OK);

    // Huit essais faux lancés ensemble, pour un plafond de cinq.
    let mauvais = json!({ "email": email, "code": "000000" });
    let r = tokio::join!(
        service.post("/v1/auth/otp/verify", None, mauvais.clone()),
        service.post("/v1/auth/otp/verify", None, mauvais.clone()),
        service.post("/v1/auth/otp/verify", None, mauvais.clone()),
        service.post("/v1/auth/otp/verify", None, mauvais.clone()),
        service.post("/v1/auth/otp/verify", None, mauvais.clone()),
        service.post("/v1/auth/otp/verify", None, mauvais.clone()),
        service.post("/v1/auth/otp/verify", None, mauvais.clone()),
        service.post("/v1/auth/otp/verify", None, mauvais.clone()),
    );
    let reponses = [r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7];

    let incorrects = reponses
        .iter()
        .filter(|(_, corps)| corps["message"].as_str() == Some("Code incorrect."))
        .count();
    assert!(
        incorrects <= 5,
        "le plafond de cinq tentatives a été franchi : {incorrects} essais décomptés"
    );

    // Et le plafond est bien atteint : un neuvième essai est refusé comme tel.
    let (_, corps) = service
        .post("/v1/auth/otp/verify", None, mauvais.clone())
        .await;
    assert!(
        corps["message"]
            .as_str()
            .unwrap_or("")
            .contains("Trop de tentatives"),
        "après la rafale, le code doit être épuisé — obtenu : {corps}"
    );
}

/// Un compte en cours de suppression ne doit plus rien pouvoir faire.
///
/// La suppression révoque les jetons de renouvellement, mais le jeton d'accès
/// déjà émis reste valide un quart d'heure — et le portier ne refusait que le
/// statut « suspended ». Pendant ce quart d'heure, le compte continuait donc
/// de publier, de demander, de discuter, alors que la page publique promet
/// qu'il « demeure invisible et inutilisable dans l'intervalle ».
#[tokio::test]
async fn un_compte_en_suppression_ne_peut_plus_agir() {
    let service = Service::monter().await;
    let compte = service.compte("c_efface", "depart").await;
    let acces = emettre_jeton(SECRET, &compte, 900).expect("jeton émis");

    let (statut, corps) = service.delete("/v1/auth/account", Some(&acces)).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let (statut, corps) = service
        .post(
            "/v1/plans",
            Some(&acces),
            json!({
                "title": "Un plan publie apres la suppression",
                "category": "balade",
                "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
            }),
        )
        .await;
    assert_eq!(
        statut,
        StatusCode::UNAUTHORIZED,
        "un compte supprimé a publié un plan : {corps}"
    );
}

/// La mise en pause ne doit pas ressusciter un compte en cours de suppression.
///
/// « Reprendre » écrivait « active » sans regarder le statut de départ. Un
/// compte à « deleting » redevenait donc « active » — visible dans le fil, et
/// joignable — tout en restant marqué pour la purge. Il se serait évanoui au
/// bout de trente jours au milieu de conversations en cours.
#[tokio::test]
async fn reprendre_ne_ressuscite_pas_un_compte_supprime() {
    let service = Service::monter().await;
    let compte = service.compte("c_revenant", "depart").await;
    let acces = emettre_jeton(SECRET, &compte, 900).expect("jeton émis");

    let (statut, _) = service.delete("/v1/auth/account", Some(&acces)).await;
    assert_eq!(statut, StatusCode::OK);

    // Le portier devrait déjà refuser ; l'assertion porte sur l'état en base,
    // pour que le test tienne même si l'un des deux verrous cédait.
    let _ = service
        .post("/v1/me/pause", Some(&acces), json!({ "paused": false }))
        .await;

    let statut_final: String = {
        use sea_orm::{ConnectionTrait, Statement};
        service
            .db
            .query_one_raw(Statement::from_string(
                service.db.get_database_backend(),
                format!("SELECT status AS s FROM accounts WHERE id='{compte}'"),
            ))
            .await
            .unwrap()
            .map(|l| l.try_get::<String>("", "s").unwrap())
            .expect("compte présent")
    };
    assert_eq!(
        statut_final, "deleting",
        "le compte est revenu à « {statut_final} » alors qu'il était supprimé"
    );
}

/// Se reconnecter ne doit pas rouvrir une session sur un compte supprimé.
///
/// La vérification du code ouvrait une session sans regarder le statut. Elle
/// était inoffensive — le portier refuse chacun des appels qui suivent — mais
/// elle annonçait une reconnexion réussie à quelqu'un dont le compte part à la
/// purge, et ne disait nulle part pourquoi plus rien ne marchait ensuite.
#[tokio::test]
async fn se_reconnecter_sur_un_compte_supprime_est_refuse_et_explique() {
    let service = Service::monter().await;
    let (acces, _) = session(&service, "supprime_relog").await;

    let (statut, _) = service.delete("/v1/auth/account", Some(&acces)).await;
    assert_eq!(statut, StatusCode::OK);

    let email = &service.email("supprime_relog");
    let (statut, corps) = service
        .post("/v1/auth/otp/request", None, json!({ "email": email }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    let code = corps["devCode"]
        .as_str()
        .expect("le code hors production")
        .to_string();

    let (statut, corps) = service
        .post(
            "/v1/auth/otp/verify",
            None,
            json!({ "email": email, "code": code }),
        )
        .await;
    assert_eq!(
        statut,
        StatusCode::UNAUTHORIZED,
        "une session s'est ouverte sur un compte supprimé : {corps}"
    );
    assert!(
        corps["message"]
            .as_str()
            .unwrap_or_default()
            .contains("suppression"),
        "le refus doit dire pourquoi : {corps}"
    );
}

/// Supprimer son compte doit faire taire son téléphone, et le dire à l'autre.
///
/// Entre la demande de suppression et la purge il s'écoule trente jours.
/// Pendant ce temps, la conversation restait ouverte : le correspondant
/// continuait d'écrire dans une conversation que plus personne ne lira jamais,
/// et chaque message poussait une alerte sur le téléphone de qui venait de
/// partir. Un mois de notifications après avoir supprimé son compte.
#[tokio::test]
async fn supprimer_son_compte_clot_les_conversations_et_fait_taire_les_appareils() {
    use crate::entities::devices;
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

    let service = Service::monter().await;
    let partant = service.compte("c_partant_conv", "depart").await;
    let reste = service.compte("c_reste_conv", "depart").await;
    let acces = emettre_jeton(SECRET, &partant, 900).expect("jeton émis");
    let acces_reste = emettre_jeton(SECRET, &reste, 900).expect("jeton émis");

    // Un appareil avec ses deux jetons, comme après un vrai enregistrement.
    devices::ActiveModel {
        id: Set(format!("d-{partant}")),
        account_id: Set(partant.clone()),
        platform: Set("ios".to_string()),
        vendor_id: Set(format!("v-{partant}")),
        model: Set(None),
        os_version: Set(None),
        app_version: Set(None),
        apns_token: Set(Some("jeton-alerte".to_string())),
        push_to_start_token: Set(Some("jeton-banniere".to_string())),
        apns_environment: Set("sandbox".to_string()),
        last_seen_at: Set(chrono::Utc::now().naive_utc()),
        created_at: Set(chrono::Utc::now().naive_utc()),
    }
    .insert(&service.db)
    .await
    .expect("appareil inséré");

    // La conversation naît du parcours réel : un plan, une demande, une
    // acceptation. Elle porte des clés étrangères qu'une insertion directe ne
    // saurait pas honorer.
    let (statut, plan) = service
        .post(
            "/v1/plans",
            Some(&acces),
            json!({
                "title": "Un cafe avant de sen aller",
                "category": "repas",
                "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{plan}");

    let (statut, demande) = service
        .post(
            "/v1/requests",
            Some(&acces_reste),
            json!({
                "planId": plan["id"].as_str().unwrap(),
                "message": "Ce cafe me tente beaucoup, je serais ravi de venir.",
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{demande}");

    let (statut, accepte) = service
        .post(
            &format!("/v1/requests/{}/accept", demande["id"].as_str().unwrap()),
            Some(&acces),
            json!({}),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{accepte}");
    let conversation = accepte["conversationId"]
        .as_str()
        .expect("conversation")
        .to_string();

    let (statut, corps) = service.delete("/v1/auth/account", Some(&acces)).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let close = crate::entities::conversations::Entity::find_by_id(conversation.clone())
        .one(&service.db)
        .await
        .unwrap()
        .expect("conversation présente");
    assert!(
        close.closed_at.is_some(),
        "la conversation est restée ouverte : le correspondant écrirait dans le vide"
    );

    for appareil in devices::Entity::find()
        .filter(devices::Column::AccountId.eq(partant.as_str()))
        .all(&service.db)
        .await
        .unwrap()
    {
        assert!(
            appareil.apns_token.is_none() && appareil.push_to_start_token.is_none(),
            "un jeton de poussée a survécu à la suppression : le téléphone sonnerait encore"
        );
    }

    // Et l'autre côté ne peut plus écrire — il le voit, plutôt que de deviner.
    let (statut, corps) = service
        .post(
            &format!("/v1/conversations/{conversation}/messages"),
            Some(&acces_reste),
            json!({ "body": "Tu es toujours la ?" }),
        )
        .await;
    assert_eq!(
        statut,
        StatusCode::UNPROCESSABLE_ENTITY,
        "le correspondant a pu écrire à un compte supprimé : {corps}"
    );
    assert!(
        corps["message"]
            .as_str()
            .unwrap_or_default()
            .contains("close"),
        "le refus doit se lire dans l'application : {corps}"
    );
}
