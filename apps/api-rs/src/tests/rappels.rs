//! Le rappel avant le rendez-vous.
//!
//! Ce qu'on éprouve ici n'est pas qu'une notification parte : c'est QUI la
//! reçoit, et surtout qu'elle ne parte qu'une fois.

use super::Service;
use crate::rappels;
use axum::http::StatusCode;
use sea_orm::ConnectionTrait;
use serde_json::json;

/// Publie un plan pour une personne et rend son identifiant.
async fn plan_de(service: &Service, hote: &str, titre: &str) -> String {
    let (statut, corps) = service
        .post(
            "/v1/plans",
            Some(&service.jeton(hote)),
            json!({
                "title": titre,
                "category": "balade",
                "capacity": 1,
                "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    corps["id"].as_str().unwrap().to_string()
}

/// Rapproche l'heure d'un plan, en base.
///
/// La route refuse de publier trop près, et c'est bien : ce qu'on éprouve est
/// le PASSAGE DU TEMPS, pas la publication.
async fn dans(service: &Service, plan: &str, minutes: i64) {
    let heure = (chrono::Utc::now() + chrono::Duration::minutes(minutes))
        .naive_utc()
        .format("%Y-%m-%d %H:%M:%S");
    service
        .db
        .execute_unprepared(&format!(
            "UPDATE plans SET startsAt = '{heure}' WHERE id = '{plan}'"
        ))
        .await
        .expect("heure rapprochée");
}

#[tokio::test]
async fn le_rendez_vous_qui_approche_est_rappele_a_qui_y_a_rendez_vous() {
    let service = Service::monter().await;
    let hote = service.compte("c_hote_rap", "depart").await;
    let invite = service.compte("c_invite_rap", "depart").await;
    let absent = service.compte("c_absent_rap", "depart").await;

    let telephone_hote = service.appareil(&hote).await;
    // Une bannière plutôt qu'une alerte à l'acceptation : c'est le RAPPEL
    // qu'on compte ici, et le repli du oui s'y ajouterait.
    let telephone_invite = service.appareil_avec(&invite, true).await;
    let telephone_absent = service.appareil(&absent).await;

    let plan = plan_de(&service, "c_hote_rap", "Une balade au bord de l eau").await;

    // L'un est accepté, l'autre attend toujours.
    let (statut, corps) = service
        .post(
            "/v1/requests",
            Some(&service.jeton("c_invite_rap")),
            json!({ "planId": plan, "message": "Cette balade me tente beaucoup, je viendrais volontiers." }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    let demande = corps["id"].as_str().unwrap().to_string();
    let (statut, _) = service
        .post(
            &format!("/v1/requests/{demande}/accept"),
            Some(&service.jeton("c_hote_rap")),
            json!({}),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);

    dans(&service, &plan, 60).await;
    let bilan = rappels::executer(&service.etat).await.expect("passage");
    assert_eq!(bilan.plans, 1);

    assert_eq!(
        service.alertes_vers(&telephone_hote).len(),
        1,
        "l'auteur n'a pas été rappelé de son propre plan"
    );
    assert_eq!(
        service.alertes_vers(&telephone_invite).len(),
        1,
        "la personne attendue n'a pas été rappelée"
    );
    // Celui qui n'a rien demandé n'a pas de rendez-vous à se rappeler.
    assert!(service.alertes_vers(&telephone_absent).is_empty());
}

#[tokio::test]
async fn une_demande_encore_en_attente_ne_se_rappelle_pas() {
    let service = Service::monter().await;
    service.compte("c_hote_att", "depart").await;
    let attente = service.compte("c_attente_att", "depart").await;
    let telephone = service.appareil(&attente).await;
    let plan = plan_de(&service, "c_hote_att", "Un concert au parc").await;

    let (statut, _) = service
        .post(
            "/v1/requests",
            Some(&service.jeton("c_attente_att")),
            json!({ "planId": plan, "message": "Ce concert me dit beaucoup, je serais ravi de venir." }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);

    dans(&service, &plan, 60).await;
    rappels::executer(&service.etat).await.expect("passage");

    // « C'est bientôt » pour un plan dont on attend toujours la réponse serait
    // une fausse joie, et une de trop.
    assert!(
        service.alertes_vers(&telephone).is_empty(),
        "une demande sans réponse a reçu un rappel de rendez-vous"
    );
}

#[tokio::test]
async fn un_rappel_ne_part_qu_une_fois() {
    let service = Service::monter().await;
    let hote = service.compte("c_hote_une", "depart").await;
    let telephone = service.appareil(&hote).await;
    let plan = plan_de(&service, "c_hote_une", "Un marche le dimanche").await;
    dans(&service, &plan, 60).await;

    // Le balayage tourne toutes les dix minutes : sans la marque en base, le
    // même plan serait rappelé six fois par heure. Une notification doublée
    // est une raison de couper les notifications.
    for _ in 0..3 {
        rappels::executer(&service.etat).await.expect("passage");
    }
    assert_eq!(
        service.alertes_vers(&telephone).len(),
        1,
        "le même rendez-vous a été rappelé plusieurs fois"
    );
}

#[tokio::test]
async fn un_balayage_en_retard_rappelle_quand_meme() {
    let service = Service::monter().await;
    let hote = service.compte("c_hote_tardif", "depart").await;
    let telephone = service.appareil(&hote).await;
    let plan = plan_de(&service, "c_hote_tardif", "Un cafe pres du canal").await;

    // Cinq minutes avant : une fenêtre étroite — « dans deux heures, à quinze
    // minutes près » — aurait laissé passer ce plan sans rien envoyer, et le
    // rappel n'aurait JAMAIS eu lieu. C'est le cas d'un dyno endormi, et c'est
    // pour lui que la marque existe.
    dans(&service, &plan, 5).await;
    let bilan = rappels::executer(&service.etat).await.expect("passage");

    assert_eq!(bilan.plans, 1, "un balayage en retard n'a rien rattrapé");
    assert_eq!(service.alertes_vers(&telephone).len(), 1);
}

#[tokio::test]
async fn un_rendez_vous_lointain_attend_son_tour() {
    let service = Service::monter().await;
    let hote = service.compte("c_hote_loin", "depart").await;
    let telephone = service.appareil(&hote).await;
    let plan = plan_de(&service, "c_hote_loin", "Une expo le samedi").await;

    // Deux jours : rappeler si tôt serait oublié comme le plan l'a été.
    rappels::executer(&service.etat).await.expect("passage");
    assert!(service.alertes_vers(&telephone).is_empty());

    dans(&service, &plan, 90).await;
    rappels::executer(&service.etat).await.expect("passage");
    assert_eq!(service.alertes_vers(&telephone).len(), 1);
}

#[tokio::test]
async fn un_rendez_vous_passe_ne_se_rappelle_plus() {
    let service = Service::monter().await;
    let hote = service.compte("c_hote_passe", "depart").await;
    let telephone = service.appareil(&hote).await;
    let plan = plan_de(&service, "c_hote_passe", "Un brunch dimanche matin").await;

    dans(&service, &plan, -30).await;
    let bilan = rappels::executer(&service.etat).await.expect("passage");

    // Sans borne basse, un plan passé inaperçu resterait éligible pour
    // toujours, et le rappel arriverait APRÈS le rendez-vous.
    assert_eq!(bilan.plans, 0);
    assert!(service.alertes_vers(&telephone).is_empty());
}

#[tokio::test]
async fn un_plan_annule_ne_se_rappelle_pas() {
    let service = Service::monter().await;
    let hote = service.compte("c_hote_annule_rap", "depart").await;
    let telephone = service.appareil(&hote).await;
    let plan = plan_de(&service, "c_hote_annule_rap", "Une partie de cartes").await;

    let (statut, _) = service
        .delete(
            &format!("/v1/plans/{plan}"),
            Some(&service.jeton("c_hote_annule_rap")),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);

    dans(&service, &plan, 60).await;
    rappels::executer(&service.etat).await.expect("passage");
    assert!(
        service.alertes_vers(&telephone).is_empty(),
        "un plan annulé a été rappelé à son auteur"
    );
}

#[tokio::test]
async fn le_rappel_se_coupe_et_se_rallume() {
    let service = Service::monter().await;
    let hote = service.compte("c_hote_coupe", "depart").await;
    let telephone = service.appareil(&hote).await;

    // Il faut une ligne de critères pour pouvoir la modifier : elle se crée au
    // premier ajustement, comme partout ailleurs.
    let (statut, corps) = service
        .patch(
            "/v1/me/preferences",
            Some(&service.jeton("c_hote_coupe")),
            json!({ "remindersOn": false }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let (statut, corps) = service
        .get("/v1/me/preferences", Some(&service.jeton("c_hote_coupe")))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["remindersOn"], false, "le réglage ne se relit pas");

    let premier = plan_de(&service, "c_hote_coupe", "Un cafe pres du canal").await;
    dans(&service, &premier, 60).await;
    rappels::executer(&service.etat).await.expect("passage");
    assert!(
        service.alertes_vers(&telephone).is_empty(),
        "le rappel part alors qu'il a été coupé"
    );

    // Et il se rallume : un réglage qu'on ne peut que couper n'est pas un
    // réglage.
    let (statut, _) = service
        .patch(
            "/v1/me/preferences",
            Some(&service.jeton("c_hote_coupe")),
            json!({ "remindersOn": true }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);

    let second = plan_de(&service, "c_hote_coupe", "Un marche le dimanche").await;
    dans(&service, &second, 60).await;
    rappels::executer(&service.etat).await.expect("passage");
    assert_eq!(service.alertes_vers(&telephone).len(), 1);
}

#[tokio::test]
async fn le_rappel_est_actif_sans_rien_demander() {
    let service = Service::monter().await;
    service.compte("c_hote_defaut", "depart").await;

    // La valeur par défaut est celle de la base : la ligne de critères est
    // insérée sans nommer cette colonne. Si elle basculait à « faux », plus
    // personne ne serait rappelé — et rien, pas même un écran, ne le dirait.
    let (statut, corps) = service
        .get("/v1/me/preferences", Some(&service.jeton("c_hote_defaut")))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(
        corps["remindersOn"], true,
        "le rappel est éteint par défaut : personne ne le découvrirait"
    );
}

#[tokio::test]
async fn sans_ligne_de_criteres_le_rappel_part_quand_meme() {
    let service = Service::monter().await;
    let hote = service.compte("c_hote_vierge", "depart").await;
    let telephone = service.appareil(&hote).await;
    let plan = plan_de(&service, "c_hote_vierge", "Une expo le samedi").await;

    // L'inscription pose toujours une ligne de critères — « un compte sans ces
    // lignes serait à moitié né ». Le cas ne s'atteint donc pas par la route,
    // et il est fabriqué ici : la ligne est effacée.
    //
    // C'est une garde défensive, et je préfère l'éprouver que la supposer. Lire
    // l'absence comme un refus priverait du rappel un compte dont la ligne
    // aurait été perdue — et ce serait un silence de plus, invisible.
    service
        .db
        .execute_unprepared(&format!(
            "DELETE FROM preferences WHERE accountId = '{hote}'"
        ))
        .await
        .expect("critères effacés");

    dans(&service, &plan, 60).await;
    rappels::executer(&service.etat).await.expect("passage");
    assert_eq!(service.alertes_vers(&telephone).len(), 1);
}

#[tokio::test]
async fn deux_balayages_concurrents_ne_rappellent_qu_une_fois() {
    let service = Service::monter().await;
    let hote = service.compte("c_hote_deuxb", "depart").await;
    let telephone = service.appareil(&hote).await;
    let plan = plan_de(&service, "c_hote_deuxb", "Un cafe pres du canal").await;
    dans(&service, &plan, 60).await;

    // Un verrou dans le cache espace déjà les balayages, et ce test ne le
    // remplace pas : il éprouve ce qui reste vrai SANS lui — cache
    // indisponible, deux dynos, une seconde d'écart.
    //
    // Ma première version ne faisait que répéter le balayage l'un après
    // l'autre. Le filtre « jamais rappelé » suffisait alors, et retirer la
    // garde ne cassait rien : c'est cette mutation qui me l'a appris. Ici les
    // deux passages LISENT avant que l'un n'écrive, et seule l'écriture
    // conditionnée les départage.
    let (a, b) = tokio::join!(
        rappels::executer(&service.etat),
        rappels::executer(&service.etat)
    );
    let total = a.expect("passage").plans + b.expect("passage").plans;

    assert_eq!(total, 1, "deux balayages ont rappelé le même plan");
    assert_eq!(
        service.alertes_vers(&telephone).len(),
        1,
        "le même rendez-vous a été rappelé deux fois"
    );
}
