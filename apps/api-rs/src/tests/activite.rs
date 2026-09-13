//! La Live Activity et le résumé de montre.
//!
//! L'état poussé est délibérément minuscule — un titre, une date, deux
//! compteurs. Aucun nom, aucune photo, aucun message : ce qui s'affiche sur un
//! écran verrouillé doit pouvoir être lu par quelqu'un d'autre sans rien
//! révéler de qui vous voyez. Ces tests vérifient autant ce que l'état
//! contient que ce qu'il ne contient pas.

use super::Service;
use axum::http::StatusCode;
use serde_json::json;

async fn appareil(service: &Service, nom: &str, vendor: &str) {
    let (statut, corps) = service
        .put("/v1/devices", Some(&service.jeton(nom)), json!({
            "vendorId": vendor,
            "platform": "ios",
            "apnsToken": "un-jeton-apns-de-test-suffisamment-long",
            "pushToStartToken": "un-jeton-push-to-start-de-test-assez-long",
        }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
}

async fn plan_de(service: &Service, nom: &str, titre: &str) -> String {
    let (statut, corps) = service
        .post("/v1/plans", Some(&service.jeton(nom)), json!({
            "title": titre,
            "category": "balade",
            "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
        }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    corps["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn l_etat_porte_le_prochain_plan_et_les_deux_compteurs() {
    let service = Service::monter().await;
    service.compte("c_hote_la", "depart").await;
    service.compte("c_invite_la", "depart").await;

    let plan = plan_de(&service, "c_hote_la", "Une balade sur les quais").await;
    let (statut, _) = service
        .post("/v1/requests", Some(&service.jeton("c_invite_la")), json!({
            "planId": plan,
            "message": "Cette balade me tente beaucoup, je serais ravi de venir.",
        }))
        .await;
    assert_eq!(statut, StatusCode::OK);

    // Vu de l'hôte : son plan, et une demande à trancher.
    let (statut, corps) = service
        .get("/v1/live-activity/state", Some(&service.jeton("c_hote_la")))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["planTitle"], "Une balade sur les quais");
    assert_eq!(corps["pendingRequests"], 1);
    assert_eq!(corps["awaitingReply"], 0);
    assert!(corps["planStartsAt"].as_str().unwrap().ends_with('Z'));

    // Vu de l'invité : pas encore de plan, une attente.
    let (_, corps) = service
        .get("/v1/live-activity/state", Some(&service.jeton("c_invite_la")))
        .await;
    assert_eq!(corps["planTitle"], json!(null));
    assert_eq!(corps["pendingRequests"], 0);
    assert_eq!(corps["awaitingReply"], 1);
}

/// Ce qui part chez Apple ne doit nommer personne : ni pseudo, ni message, ni
/// photo. Le test porte sur la charge rendue, qui est exactement celle qui est
/// poussée.
#[tokio::test]
async fn l_etat_ne_nomme_personne() {
    let service = Service::monter().await;
    service.compte("c_hote_muet", "depart").await;
    service.compte("c_invite_muet", "depart").await;

    let plan = plan_de(&service, "c_hote_muet", "Un plan dont on ne dira rien").await;
    let (statut, _) = service
        .post("/v1/requests", Some(&service.jeton("c_invite_muet")), json!({
            "planId": plan,
            "message": "Un message que personne ne doit lire sur un ecran verrouille.",
        }))
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (_, corps) = service
        .get("/v1/live-activity/state", Some(&service.jeton("c_hote_muet")))
        .await;

    let rendu = corps.to_string();
    for interdit in [
        "Un message que personne",
        &service.id("c_invite_muet"),
        "Compte ",
    ] {
        assert!(
            !rendu.contains(interdit),
            "l'état laisse filtrer « {interdit} » : {rendu}"
        );
    }
    // Les seules clés autorisées.
    let objet = corps.as_object().expect("un objet");
    let mut cles: Vec<&String> = objet.keys().collect();
    cles.sort();
    assert_eq!(
        cles,
        vec![
            "awaitingReply",
            "pendingRequests",
            "planStartsAt",
            "planTitle",
            "updatedAt"
        ]
    );
}

#[tokio::test]
async fn declarer_une_activite_exige_un_appareil_connu() {
    let service = Service::monter().await;
    service.compte("c_sans_appareil", "depart").await;

    let (statut, _) = service
        .post("/v1/live-activity/sessions", Some(&service.jeton("c_sans_appareil")), json!({
            "vendorId": "vendor-inconnu-0001",
            "updateToken": "un-jeton-de-mise-a-jour-assez-long",
        }))
        .await;
    assert_eq!(statut, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn declarer_une_activite_rend_l_etat_courant() {
    let service = Service::monter().await;
    service.compte("c_activite", "depart").await;
    appareil(&service, "c_activite", "vendor-activite-0001").await;
    plan_de(&service, "c_activite", "Un plan a afficher sur l ecran").await;

    let (statut, corps) = service
        .post("/v1/live-activity/sessions", Some(&service.jeton("c_activite")), json!({
            "vendorId": "vendor-activite-0001",
            "updateToken": "un-jeton-de-mise-a-jour-assez-long",
        }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["state"]["planTitle"], "Un plan a afficher sur l ecran");
}

/// ActivityKit re-déclare la même activité après un redémarrage : cela doit
/// mettre à jour la ligne, pas en créer une seconde.
#[tokio::test]
async fn declarer_deux_fois_le_meme_jeton_ne_cree_qu_une_session() {
    let service = Service::monter().await;
    service.compte("c_redeclare", "depart").await;
    appareil(&service, "c_redeclare", "vendor-redeclare-001").await;

    for _ in 0..2 {
        let (statut, corps) = service
            .post("/v1/live-activity/sessions", Some(&service.jeton("c_redeclare")), json!({
                "vendorId": "vendor-redeclare-001",
                "updateToken": "un-jeton-redeclare-assez-long-pour-passer",
            }))
            .await;
        assert_eq!(statut, StatusCode::OK, "{corps}");
    }

    let compte = service.id("c_redeclare");
    let sessions = {
        use sea_orm::{ConnectionTrait, Statement};
        service
            .db
            .query_one_raw(Statement::from_string(
                service.db.get_database_backend(),
                format!("SELECT COUNT(*) AS v FROM live_activity_sessions WHERE accountId='{compte}'"),
            ))
            .await
            .unwrap()
            .map(|l| l.try_get::<i64>("", "v").unwrap())
            .unwrap()
    };
    assert_eq!(sessions, 1, "une seconde session a été créée");
}

#[tokio::test]
async fn terminer_une_activite_ne_ferme_que_les_siennes() {
    let service = Service::monter().await;
    service.compte("c_fin_a", "depart").await;
    service.compte("c_fin_b", "depart").await;
    appareil(&service, "c_fin_a", "vendor-fin-a-0001").await;
    appareil(&service, "c_fin_b", "vendor-fin-b-0001").await;
    // Sans rien à afficher, l'état serait vide et la session se fermerait
    // d'elle-même : ce n'est pas ce qu'on cherche à éprouver ici.
    plan_de(&service, "c_fin_a", "Un plan qui tient la banniere").await;

    let jeton_partage = "un-jeton-que-deux-comptes-invoquent-1234";
    let (statut, _) = service
        .post("/v1/live-activity/sessions", Some(&service.jeton("c_fin_a")), json!({
            "vendorId": "vendor-fin-a-0001",
            "updateToken": jeton_partage,
        }))
        .await;
    assert_eq!(statut, StatusCode::OK);

    // B tente de fermer la session de A en présentant le même jeton.
    let (statut, _) = service
        .delete(
            &format!("/v1/live-activity/sessions/{jeton_partage}"),
            Some(&service.jeton("c_fin_b")),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);

    let compte_a = service.id("c_fin_a");
    let ouvertes = {
        use sea_orm::{ConnectionTrait, Statement};
        service
            .db
            .query_one_raw(Statement::from_string(
                service.db.get_database_backend(),
                format!(
                    "SELECT COUNT(*) AS v FROM live_activity_sessions \
                     WHERE accountId='{compte_a}' AND endedAt IS NULL"
                ),
            ))
            .await
            .unwrap()
            .map(|l| l.try_get::<i64>("", "v").unwrap())
            .unwrap()
    };
    assert_eq!(ouvertes, 1, "un tiers a fermé la Live Activity d'un autre");
}

/// Sans appareil déclaré, il n'y a rien à démarrer — et ce n'est pas une
/// erreur : le compte n'a simplement pas d'iPhone enregistré.
#[tokio::test]
async fn demarrer_sans_appareil_ne_demarre_rien() {
    let service = Service::monter().await;
    service.compte("c_rien_a_demarrer", "depart").await;

    let (statut, corps) = service
        .post("/v1/live-activity/start", Some(&service.jeton("c_rien_a_demarrer")), json!({}))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["started"], 0);
}

/// APNs n'est pas configuré en test : l'envoi est simulé et compté comme
/// réussi. Ce qui est éprouvé ici, c'est que la route va jusqu'au bout.
#[tokio::test]
async fn demarrer_avec_un_appareil_et_un_plan() {
    let service = Service::monter().await;
    service.compte("c_demarre", "depart").await;
    appareil(&service, "c_demarre", "vendor-demarre-0001").await;
    plan_de(&service, "c_demarre", "Un plan qui vaut une banniere").await;

    let (statut, corps) = service
        .post("/v1/live-activity/start", Some(&service.jeton("c_demarre")), json!({}))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["started"], 1);
}

#[tokio::test]
async fn le_resume_de_montre_est_compact_et_anonyme() {
    let service = Service::monter().await;
    service.compte("c_montre", "depart").await;
    service.compte("c_invite_montre", "depart").await;

    let plan = plan_de(&service, "c_montre", "Un plan a lire au poignet").await;
    let (statut, _) = service
        .post("/v1/requests", Some(&service.jeton("c_invite_montre")), json!({
            "planId": plan,
            "message": "Cette balade me tente beaucoup, je serais ravi de venir.",
        }))
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (statut, corps) = service
        .get("/v1/watch/summary", Some(&service.jeton("c_montre")))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["pendingRequests"], 1);
    assert_eq!(corps["awaitingReply"], 0);
    assert_eq!(corps["nextPlan"]["title"], "Un plan a lire au poignet");
    assert_eq!(corps["nextPlan"]["city"], "Lyon");

    let mut cles: Vec<&String> = corps.as_object().expect("un objet").keys().collect();
    cles.sort();
    assert_eq!(
        cles,
        vec!["awaitingReply", "generatedAt", "nextPlan", "pendingRequests"]
    );
    // Quelques centaines d'octets, pas plus : c'est une montre.
    assert!(corps.to_string().len() < 400, "résumé trop gros : {corps}");
}

/// Un plan complet reste un rendez-vous — c'est même celui dont on a le plus
/// besoin sur l'écran verrouillé.
#[tokio::test]
async fn un_plan_annule_disparait_de_l_etat() {
    let service = Service::monter().await;
    service.compte("c_annule", "depart").await;
    let plan = plan_de(&service, "c_annule", "Un plan qui ne se fera pas").await;

    let (_, avant) = service
        .get("/v1/live-activity/state", Some(&service.jeton("c_annule")))
        .await;
    assert_eq!(avant["planTitle"], "Un plan qui ne se fera pas");

    let (statut, _) = service
        .delete(&format!("/v1/plans/{plan}"), Some(&service.jeton("c_annule")))
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (_, apres) = service
        .get("/v1/live-activity/state", Some(&service.jeton("c_annule")))
        .await;
    assert_eq!(apres["planTitle"], json!(null), "le plan annulé s'affiche encore");
}

/// Fermer une activité efface sa ligne, elle ne la marque pas close.
///
/// La politique de confidentialité promet que les activités en direct sont
/// « effacées dès la fin de l'activité ». La ligne portait `last_state_json` —
/// L'INSTANTANÉ DE CE QUI S'EST AFFICHÉ SUR UN ÉCRAN VERROUILLÉ — et restait en
/// base jusqu'à la péremption de la session, soit des heures après que la
/// personne l'a fermée.
#[tokio::test]
async fn fermer_une_activite_efface_sa_ligne() {
    let service = Service::monter().await;
    service.compte("c_fin_efface", "depart").await;
    appareil(&service, "c_fin_efface", "vendor-efface-0001").await;
    plan_de(&service, "c_fin_efface", "Un plan qui tient la banniere").await;

    let jeton = "un-jeton-de-session-a-effacer-0001";
    let (statut, _) = service
        .post("/v1/live-activity/sessions", Some(&service.jeton("c_fin_efface")), json!({
            "vendorId": "vendor-efface-0001",
            "updateToken": jeton,
        }))
        .await;
    assert_eq!(statut, StatusCode::OK);
    assert_eq!(lignes_de_session(&service, &service.id("c_fin_efface")).await, 1);

    let (statut, _) = service
        .delete(
            &format!("/v1/live-activity/sessions/{jeton}"),
            Some(&service.jeton("c_fin_efface")),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);

    assert_eq!(
        lignes_de_session(&service, &service.id("c_fin_efface")).await,
        0,
        "la ligne survit à la fermeture, avec l'instantané de l'écran verrouillé"
    );
}

async fn lignes_de_session(service: &Service, compte: &str) -> i64 {
    use sea_orm::{ConnectionTrait, Statement};
    service
        .db
        .query_one_raw(Statement::from_string(
            service.db.get_database_backend(),
            format!("SELECT COUNT(*) AS v FROM live_activity_sessions WHERE accountId='{compte}'"),
        ))
        .await
        .unwrap()
        .map(|l| l.try_get::<i64>("", "v").unwrap())
        .unwrap()
}
