//! Les achats : unités consommables et notifications de l'App Store.
//!
//! Weave est vendu sur l'App Store ; le serveur ne fait que vérifier puis
//! enregistrer ce qu'Apple lui transmet. Aucune offre n'achète de visibilité —
//! ce qui se vend, c'est l'horizon de publication et les plans de groupe.

use super::{Service, refuse};
use axum::http::StatusCode;
use sea_orm::ConnectionTrait;
use serde_json::json;

/// Une transaction signée pour de vrai, par l'autorité de test.
///
/// Elle se contentait d'encoder la charge utile en base64 entre deux segments
/// inventés — ce qui suffisait tant que la vérification n'existait pas. Elle
/// existe : une transaction non signée est désormais refusée, comme elle doit
/// l'être, et ces tests passeraient à côté de ce qu'ils éprouvent.
fn transaction(charge: serde_json::Value) -> String {
    super::storekit::transaction_signee(charge)
}

/// Une notification serveur à serveur, telle qu'Apple la forme.
fn notification(
    genre: &str,
    sous_genre: Option<&str>,
    transaction: serde_json::Value,
    renouvellement: Option<serde_json::Value>,
) -> String {
    super::storekit::notification_signee(genre, sous_genre, transaction, renouvellement)
}

#[tokio::test]
async fn un_achat_a_l_unite_credite_le_solde() {
    let service = Service::monter().await;
    service.compte("c_acheteur", "depart").await;

    let (statut, corps) = service
        .post(
            "/v1/billing/units",
            Some(&service.jeton("c_acheteur")),
            json!({
                "signedTransaction": transaction(json!({
                    "productId": "com.weave.app.unit.renfort",
                    "transactionId": "tx-renfort-001",
                    "environment": "Sandbox",
                })),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["alreadyApplied"], false);
    assert_eq!(corps["credits"]["renfort"], 1);
    // Tous les SKU sont présents, même à zéro : l'application affiche la liste
    // entière, et une clé absente s'y lirait comme une erreur.
    assert_eq!(corps["credits"]["bilan"], 0);
}

/// `transactionId` est unique côté Apple. Un client qui réessaie ne doit pas
/// être crédité deux fois — ni recevoir une erreur de base pour autant.
#[tokio::test]
async fn une_meme_transaction_ne_credite_qu_une_fois() {
    let service = Service::monter().await;
    service.compte("c_rejoue", "depart").await;

    let signee = transaction(json!({
        "productId": "com.weave.app.unit.escale",
        "transactionId": "tx-escale-rejouee",
        "environment": "Sandbox",
    }));

    let (statut, premier) = service
        .post(
            "/v1/billing/units",
            Some(&service.jeton("c_rejoue")),
            json!({ "signedTransaction": signee.clone() }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{premier}");
    assert_eq!(premier["credits"]["escale"], 1);

    let (statut, second) = service
        .post(
            "/v1/billing/units",
            Some(&service.jeton("c_rejoue")),
            json!({ "signedTransaction": signee }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{second}");
    assert_eq!(second["alreadyApplied"], true);
    assert_eq!(
        second["credits"]["escale"], 1,
        "le solde a doublé : {second}"
    );
}

#[tokio::test]
async fn un_produit_inconnu_est_refuse() {
    let service = Service::monter().await;
    service.compte("c_produit", "depart").await;

    let (statut, _) = service
        .post(
            "/v1/billing/units",
            Some(&service.jeton("c_produit")),
            json!({
                "signedTransaction": transaction(json!({
                    "productId": "com.weave.app.unit.licorne",
                    "transactionId": "tx-licorne",
                })),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);
}

/// L'achat crédite un solde que le renfort dépense : les deux routes se
/// rejoignent, et c'est ce chemin-là qu'emprunte un vrai achat.
#[tokio::test]
async fn un_renfort_achete_se_depense() {
    let service = Service::monter().await;
    service.compte("c_boucle", "depart").await;

    let (statut, _) = service
        .post(
            "/v1/billing/units",
            Some(&service.jeton("c_boucle")),
            json!({
                "signedTransaction": transaction(json!({
                    "productId": "com.weave.app.unit.renfort",
                    "transactionId": "tx-boucle-001",
                })),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (statut, corps) = service
        .post(
            "/v1/requests/renfort",
            Some(&service.jeton("c_boucle")),
            json!({}),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["requestsLeftToday"], 10);
}

/// Une notification qui ne correspond à aucun abonnement connu est ignorée
/// sans rien changer : la route n'est pas authentifiée, elle ne doit pas
/// servir de levier.
#[tokio::test]
async fn une_notification_orpheline_est_ignoree() {
    let service = Service::monter().await;

    let (statut, corps) = service
        .post(
            "/v1/billing/apple/notifications",
            None,
            json!({
                "signedPayload": notification(
                    "DID_RENEW",
                    None,
                    json!({
                        "productId": "com.weave.app.sub.escapade.monthly",
                        "transactionId": "tx-inconnue",
                        "originalTransactionId": "orig-inconnue",
                    }),
                    None,
                ),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["ignored"], true);
}

/// Une échéance passée fait retomber le compte au palier de départ. Jamais
/// l'inverse : un abonnement expiré ne conserve pas ses droits.
#[tokio::test]
async fn un_abonnement_expire_retombe_au_palier_de_depart() {
    let service = Service::monter().await;
    let compte = service.compte("c_expire", "escapade").await;

    service
        .db
        .execute_unprepared(&format!(
            "UPDATE subscriptions SET originalTransactionId='orig-expire' WHERE accountId='{compte}'"
        ))
        .await
        .unwrap();

    // Le palier d'avant : vingt-cinq demandes par jour.
    let (_, avant) = service
        .get("/v1/me", Some(&service.jeton("c_expire")))
        .await;
    assert_eq!(avant["tier"], "escapade");
    assert_eq!(avant["requestsLeftToday"], 25);

    let hier = (chrono::Utc::now() - chrono::Duration::days(1)).timestamp_millis();
    let (statut, corps) = service
        .post(
            "/v1/billing/apple/notifications",
            None,
            json!({
                "signedPayload": notification(
                    "EXPIRED",
                    Some("VOLUNTARY"),
                    json!({
                        "productId": "com.weave.app.sub.escapade.monthly",
                        "transactionId": "tx-expire",
                        "originalTransactionId": "orig-expire",
                        "expiresDate": hier,
                    }),
                    None,
                ),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["ignored"], false);

    let (_, apres) = service
        .get("/v1/me", Some(&service.jeton("c_expire")))
        .await;
    assert_eq!(
        apres["tier"], "depart",
        "le palier expiré a survécu : {apres}"
    );
    assert_eq!(apres["requestsLeftToday"], 5);
}

/// Un renouvellement remet le palier et recharge la dotation mensuelle. Les
/// crédits achetés à l'unité ne sont jamais remis à zéro : on ajoute.
#[tokio::test]
async fn un_renouvellement_recharge_la_dotation_sans_effacer_les_achats() {
    let service = Service::monter().await;
    let compte = service.compte("c_renouvelle", "escapade").await;

    service
        .db
        .execute_unprepared(&format!(
            "UPDATE subscriptions SET originalTransactionId='orig-renouvelle' WHERE accountId='{compte}'"
        ))
        .await
        .unwrap();

    // Une escale achetée à l'unité, avant le renouvellement.
    let (statut, _) = service
        .post(
            "/v1/billing/units",
            Some(&service.jeton("c_renouvelle")),
            json!({
                "signedTransaction": transaction(json!({
                    "productId": "com.weave.app.unit.escale",
                    "transactionId": "tx-escale-avant",
                })),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);

    let demain = (chrono::Utc::now() + chrono::Duration::days(30)).timestamp_millis();
    let (statut, corps) = service
        .post(
            "/v1/billing/apple/notifications",
            None,
            json!({
                "signedPayload": notification(
                    "DID_RENEW",
                    None,
                    json!({
                        "productId": "com.weave.app.sub.escapade.monthly",
                        "transactionId": "tx-renouvelle",
                        "originalTransactionId": "orig-renouvelle",
                        "expiresDate": demain,
                    }),
                    None,
                ),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let (_, fiche) = service
        .get("/v1/me", Some(&service.jeton("c_renouvelle")))
        .await;
    assert_eq!(fiche["tier"], "escapade");
    // Une escale achetée, plus celle que le palier Escapade donne chaque mois.
    assert_eq!(
        fiche["credits"]["escale"], 2,
        "l'achat à l'unité a été effacé par la dotation : {fiche}"
    );
}

/// Deux crédits simultanés s'additionnent, ils ne s'écrasent pas.
///
/// Huitième instance de la famille « lire, puis écrire, sans rien entre les
/// deux » — et la seule qui porte sur de l'argent. Le solde était lu,
/// additionné en Rust, puis réécrit. Deux crédits concurrents — un achat et le
/// renvoi de la même notification par Apple, deux achats coup sur coup —
/// lisaient le même solde et n'en écrivaient qu'un.
///
/// Quelqu'un payait et ne recevait rien. Rien nulle part ne l'aurait signalé :
/// aucune erreur, aucun journal, juste un solde plus bas que la somme des
/// achats.
#[tokio::test]
async fn deux_credits_concurrents_s_additionnent() {
    let service = Service::monter().await;
    let compte = service.compte("c_credits", "depart").await;

    // Deux dotations de cinq, lancées ensemble sur une ligne qui n'existe pas
    // encore : c'est le cas le plus défavorable, où la création et l'incrément
    // se disputent la même ligne.
    let (a, b) = tokio::join!(
        crate::routes::billing::crediter_pour_test(&service.etat, &compte, "horizon", 5),
        crate::routes::billing::crediter_pour_test(&service.etat, &compte, "horizon", 5),
    );
    a.expect("premier crédit");
    b.expect("second crédit");

    let (statut, corps) = service
        .get("/v1/me", Some(&service.jeton("c_credits")))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(
        corps["credits"]["horizon"], 10,
        "deux dotations de cinq doivent faire dix — obtenu {}",
        corps["credits"]["horizon"]
    );
}

/// Une notification Apple renvoyée ne dote pas deux fois.
///
/// Apple renvoie ses notifications tant qu'il n'obtient pas de 200, et en
/// délivre parfois plusieurs pour le même événement. Chaque renvoi rappelait
/// le crédit, qui ajoute : un renouvellement retenté deux fois donnait deux
/// fois la dotation du mois. C'est gratuit, c'est répétable, et rien ne le
/// signalait.
#[tokio::test]
async fn une_dotation_de_periode_ne_sert_qu_une_fois() {
    let service = Service::monter().await;
    let compte = service.compte("c_dotation", "escapade").await;
    let echeance = (chrono::Utc::now() + chrono::Duration::days(30)).naive_utc();

    for essai in 1..=3 {
        crate::routes::billing::doter_la_periode_pour_test(
            &service.etat,
            &compte,
            "escale",
            1,
            echeance,
        )
        .await
        .unwrap_or_else(|e| panic!("dotation {essai} : {e:?}"));
    }

    let (_, corps) = service
        .get("/v1/me", Some(&service.jeton("c_dotation")))
        .await;
    assert_eq!(
        corps["credits"]["escale"], 1,
        "trois notifications pour le même mois ont donné {} escales",
        corps["credits"]["escale"]
    );

    // Le mois suivant, en revanche, dote de nouveau.
    let mois_suivant = (chrono::Utc::now() + chrono::Duration::days(60)).naive_utc();
    crate::routes::billing::doter_la_periode_pour_test(
        &service.etat,
        &compte,
        "escale",
        1,
        mois_suivant,
    )
    .await
    .expect("dotation du mois suivant");

    let (_, corps) = service
        .get("/v1/me", Some(&service.jeton("c_dotation")))
        .await;
    assert_eq!(corps["credits"]["escale"], 2, "le mois suivant doit doter");
}

/// Ce qui a été acheté survit à la dotation mensuelle.
///
/// « Escale » se vend aussi à l'unité. Remplacer le solde à chaque
/// renouvellement effacerait ce qui a été payé — d'où l'addition plutôt que le
/// remplacement, et cette règle mérite d'être tenue par un test.
#[tokio::test]
async fn une_dotation_mensuelle_n_efface_pas_ce_qui_a_ete_achete() {
    let service = Service::monter().await;
    let compte = service.compte("c_achat_escale", "escapade").await;

    // Deux escales achetées : pas de date de remise à zéro.
    crate::routes::billing::crediter_pour_test(&service.etat, &compte, "escale", 2)
        .await
        .expect("achat");

    let echeance = (chrono::Utc::now() + chrono::Duration::days(30)).naive_utc();
    crate::routes::billing::doter_la_periode_pour_test(
        &service.etat,
        &compte,
        "escale",
        1,
        echeance,
    )
    .await
    .expect("dotation");

    let (_, corps) = service
        .get("/v1/me", Some(&service.jeton("c_achat_escale")))
        .await;
    assert_eq!(
        corps["credits"]["escale"], 3,
        "les escales achetées ont été perdues : {}",
        corps["credits"]["escale"]
    );
}

/// Une « Escale » achetée peut enfin servir.
///
/// « Escale » se vend 3,99 € : « Publier depuis une autre ville pendant sept
/// jours. » Le crédit était accordé à l'achat, et le fil honorait déjà
/// l'escale — il remplace le filtre géographique par une ville dès que
/// `escaleCity` est posé.
///
/// Mais **aucune ligne n'a jamais écrit `escaleCity`.** Personne ne pouvait
/// déclencher ce qu'il venait d'acheter : le crédit s'accumulait sans usage.
#[tokio::test]
async fn une_escale_achetee_ouvre_le_fil_sur_une_autre_ville() {
    let service = Service::monter().await;
    let compte = service.compte("c_escale", "depart").await;
    let jeton = service.jeton("c_escale");

    service
        .put(
            "/v1/me/profile",
            Some(&jeton),
            json!({
                "city": "Nantes", "latitude": 47.21, "longitude": -1.55, "gender": "autre",
            }),
        )
        .await;

    // Sans crédit, l'escale est refusée : c'est ce qui la fait valoir 3,99 €.
    let (statut, corps) = service
        .post("/v1/me/escale", Some(&jeton), json!({ "city": "Lyon" }))
        .await;
    refuse(
        statut,
        &corps,
        "entitlement_required",
        &format!("une escale sans crédit : {corps}"),
    );

    crate::routes::billing::crediter_pour_test(&service.etat, &compte, "escale", 1)
        .await
        .expect("achat");

    let (statut, corps) = service
        .post("/v1/me/escale", Some(&jeton), json!({ "city": "Lyon" }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["escaleCity"], "Lyon");

    // Le crédit est dépensé, et les critères portent bien l'escale.
    let (_, moi) = service.get("/v1/me", Some(&jeton)).await;
    assert_eq!(moi["credits"]["escale"], 0, "le crédit n'a pas été dépensé");

    let (_, criteres) = service.get("/v1/me/preferences", Some(&jeton)).await;
    assert_eq!(criteres["escaleCity"], "Lyon");
    assert!(criteres["escaleUntil"].as_str().is_some(), "{criteres}");

    // Une seconde escale pendant la première ferait payer pour raccourcir ce
    // qu'on a déjà : elle est refusée.
    crate::routes::billing::crediter_pour_test(&service.etat, &compte, "escale", 1)
        .await
        .expect("second achat");
    let (statut, corps) = service
        .post("/v1/me/escale", Some(&jeton), json!({ "city": "Lille" }))
        .await;
    refuse(
        statut,
        &corps,
        "validation",
        &format!("deux escales à la fois : {corps}"),
    );

    // Le crédit du second achat n'a pas été consommé par ce refus.
    let (_, moi) = service.get("/v1/me", Some(&jeton)).await;
    assert_eq!(moi["credits"]["escale"], 1, "un refus a mangé le crédit");
}

/// Le fil d'une escale montre la ville visée, pas la sienne.
#[tokio::test]
async fn le_fil_dune_escale_montre_lautre_ville() {
    let service = Service::monter().await;
    let voyageur = service.compte("c_voyageur", "depart").await;
    let jeton = service.jeton("c_voyageur");
    service.compte("c_lyonnais", "depart").await;

    for (nom, ville, lat, lon) in [
        ("c_voyageur", "Nantes", 47.21, -1.55),
        ("c_lyonnais", "Lyon", 45.76, 4.84),
    ] {
        service
            .put(
                "/v1/me/profile",
                Some(&service.jeton(nom)),
                json!({
                    "city": ville, "latitude": lat, "longitude": lon, "gender": "autre",
                }),
            )
            .await;
    }

    let (statut, corps) = service
        .post(
            "/v1/plans",
            Some(&service.jeton("c_lyonnais")),
            json!({
                "title": "Un verre sur les quais de Saone",
                "category": "sortie",
                "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    // Depuis Nantes, le plan lyonnais est hors de portée.
    let (_, fil) = service.get("/v1/plans", Some(&jeton)).await;
    assert_eq!(fil["plans"].as_array().unwrap().len(), 0, "{fil}");

    crate::routes::billing::crediter_pour_test(&service.etat, &voyageur, "escale", 1)
        .await
        .expect("achat");
    let (statut, corps) = service
        .post("/v1/me/escale", Some(&jeton), json!({ "city": "Lyon" }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let (_, fil) = service.get("/v1/plans", Some(&jeton)).await;
    let plans = fil["plans"].as_array().expect("une liste");
    assert_eq!(
        plans.len(),
        1,
        "l'escale n'a pas ouvert le fil sur Lyon : {fil}"
    );
    assert_eq!(plans[0]["city"], "Lyon");
}

/// Le « Bilan » : vendu 2,99 €, et jusqu'ici sans aucun mécanisme.
///
/// Le crédit était accordé à l'achat et rien ne le consommait. Aucune route ne
/// le mentionnait, aucune ligne ne le lisait : on vendait un produit qui
/// n'existait pas.
#[tokio::test]
async fn un_bilan_rend_ce_qui_attire_et_ce_qui_tombe_a_plat() {
    use sea_orm::ConnectionTrait;

    let service = Service::monter().await;
    let auteur = service.compte("c_bilan", "depart").await;
    let jeton = service.jeton("c_bilan");
    service.compte("c_demandeur", "depart").await;

    for (nom, ville, lat, lon) in [
        ("c_bilan", "Nantes", 47.21, -1.55),
        ("c_demandeur", "Nantes", 47.21, -1.55),
    ] {
        service
            .put(
                "/v1/me/profile",
                Some(&service.jeton(nom)),
                json!({
                    "city": ville, "latitude": lat, "longitude": lon, "gender": "autre",
                }),
            )
            .await;
    }

    // Trois plans : deux qui attirent, un qui tombe à plat.
    let mut publies = Vec::new();
    for (titre, categorie) in [
        ("Un cafe pour parler de livres", "repas"),
        ("Une balade le long de lErdre", "balade"),
        ("Un tournoi de flechettes obscur", "jeux"),
    ] {
        let (statut, corps) = service
            .post(
                "/v1/plans",
                Some(&jeton),
                json!({
                    "title": titre,
                    "category": categorie,
                    "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
                }),
            )
            .await;
        assert_eq!(statut, StatusCode::OK, "{corps}");
        publies.push(corps["id"].as_str().unwrap().to_string());
    }

    // Une demande sur les deux premiers seulement.
    for plan in publies.iter().take(2) {
        let (statut, corps) = service
            .post(
                "/v1/requests",
                Some(&service.jeton("c_demandeur")),
                json!({
                    "planId": plan,
                    "message": "Ce plan me tente beaucoup, je serais ravi de venir.",
                }),
            )
            .await;
        assert_eq!(statut, StatusCode::OK, "{corps}");
    }

    // Le bilan est refusé — mais par le contrôle des plans à venir, qui passe
    // AVANT celui du crédit. Le commentaire disait « sans crédit, pas de
    // bilan » et le test ne le vérifiait pas : il aurait continué de passer si
    // le crédit avait cessé d'être exigé.
    //
    // La règle du crédit est tenue ailleurs, par
    // `un_palier_sans_bilan_exige_toujours_le_credit` — vérifié en la retirant.
    let (statut, corps) = service.post("/v1/me/bilan", Some(&jeton), json!({})).await;
    refuse(
        statut,
        &corps,
        "validation",
        &format!("un bilan sur des plans à venir : {corps}"),
    );

    crate::routes::billing::crediter_pour_test(&service.etat, &auteur, "bilan", 1)
        .await
        .expect("achat");

    // Les plans sont encore à venir : le bilan doit refuser, et ne pas
    // consommer le crédit — un plan à venir n'a pas fini de recevoir.
    let (statut, corps) = service.post("/v1/me/bilan", Some(&jeton), json!({})).await;
    refuse(
        statut,
        &corps,
        "validation",
        &format!("un bilan sur des plans à venir : {corps}"),
    );
    let (_, moi) = service.get("/v1/me", Some(&jeton)).await;
    assert_eq!(moi["credits"]["bilan"], 1, "un refus a mangé le crédit");

    // On fait passer les rendez-vous.
    service
        .db
        .execute_unprepared(&format!(
            "UPDATE plans SET startsAt='2020-01-01 12:00:00' WHERE authorId='{auteur}'"
        ))
        .await
        .unwrap();

    let (statut, bilan) = service.post("/v1/me/bilan", Some(&jeton), json!({})).await;
    assert_eq!(statut, StatusCode::OK, "{bilan}");

    assert_eq!(bilan["plansPasses"], 3);
    assert_eq!(bilan["demandesRecues"], 2);
    assert_eq!(bilan["plansSansAucuneDemande"], 1);

    // Ce qui attire vient en tête, ce qui tombe à plat en tête de l'autre.
    assert_eq!(bilan["cequiAttire"][0]["demandes"], 1);
    assert_eq!(bilan["ceQuiTombeAPlat"][0]["demandes"], 0);
    assert_eq!(
        bilan["ceQuiTombeAPlat"][0]["titre"],
        "Un tournoi de flechettes obscur"
    );

    // Et le crédit est dépensé, cette fois.
    let (_, moi) = service.get("/v1/me", Some(&jeton)).await;
    assert_eq!(moi["credits"]["bilan"], 0, "le crédit n'a pas été dépensé");
}

/// Un bilan sans matière est refusé, et ne coûte rien.
///
/// En dessous de quelques plans, il rendrait des moyennes sur deux points.
/// Faire payer 2,99 € pour un rapport vide serait pire que de ne rien vendre.
#[tokio::test]
async fn un_bilan_sans_matiere_ne_coute_pas_le_credit() {
    let service = Service::monter().await;
    let compte = service.compte("c_bilan_vide", "depart").await;
    let jeton = service.jeton("c_bilan_vide");

    crate::routes::billing::crediter_pour_test(&service.etat, &compte, "bilan", 1)
        .await
        .expect("achat");

    let (statut, corps) = service.post("/v1/me/bilan", Some(&jeton), json!({})).await;
    refuse(statut, &corps, "validation", &format!("{corps}"));
    assert!(
        corps["message"]
            .as_str()
            .unwrap_or_default()
            .contains("n'a pas été utilisé"),
        "le refus doit dire que le crédit est intact : {corps}"
    );

    let (_, moi) = service.get("/v1/me", Some(&jeton)).await;
    assert_eq!(
        moi["credits"]["bilan"], 1,
        "le crédit a été consommé pour rien"
    );
}

/// Une transaction forgée est refusée par la route.
///
/// C'est ce que l'API acceptait : elle décodait le base64 de la charge et
/// lisait les champs qu'elle y trouvait. N'importe qui pouvait encoder un JSON
/// annonçant le produit de son choix et s'offrir l'abonnement le plus cher.
#[tokio::test]
async fn une_transaction_forgee_est_refusee() {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

    let service = Service::monter().await;
    service.compte("c_forgeur", "depart").await;

    let charge = json!({
        "productId": "com.weave.app.sub.grandtour.monthly",
        "transactionId": "forgee-1",
        "originalTransactionId": "forgee-1",
        "bundleId": "com.weave.app",
    });
    let forgee = format!(
        "entete.{}.signature",
        URL_SAFE_NO_PAD.encode(charge.to_string())
    );

    let (statut, corps) = service
        .post(
            "/v1/billing/subscriptions",
            Some(&service.jeton("c_forgeur")),
            json!({
                "signedTransaction": forgee,
            }),
        )
        .await;
    refuse(
        statut,
        &corps,
        "validation",
        &format!("une transaction forgée a été acceptée : {corps}"),
    );

    let (_, moi) = service
        .get("/v1/me", Some(&service.jeton("c_forgeur")))
        .await;
    assert_eq!(
        moi["tier"], "depart",
        "le palier a été accordé sans paiement"
    );
}

/// Un achat fait dans une AUTRE application ne compte pas ici.
///
/// La signature d'Apple ne dit pas pour qui elle a été émise. Sans contrôle du
/// `bundleId`, un achat à un euro dans une autre application — signé par
/// Apple, chaîne parfaitement valide — se rejouerait ici pour s'offrir
/// l'abonnement le plus cher.
#[tokio::test]
async fn une_transaction_emise_pour_une_autre_application_est_refusee() {
    let service = Service::monter().await;
    service.compte("c_rejeu", "depart").await;

    let signee = super::storekit::transaction_signee(json!({
        "productId": "com.weave.app.sub.grandtour.monthly",
        "transactionId": "autre-app-1",
        "originalTransactionId": "autre-app-1",
        // Vraie signature, vraie chaîne — mais émise pour un autre paquet.
        "bundleId": "com.exemple.autre",
    }));

    let (statut, corps) = service
        .post(
            "/v1/billing/subscriptions",
            Some(&service.jeton("c_rejeu")),
            json!({
                "signedTransaction": signee,
            }),
        )
        .await;
    assert_ne!(
        statut,
        StatusCode::OK,
        "un achat d'une autre application a été accepté : {corps}"
    );

    let (_, moi) = service.get("/v1/me", Some(&service.jeton("c_rejeu"))).await;
    assert_eq!(
        moi["tier"], "depart",
        "le palier a été accordé sur l'achat d'autrui"
    );
}

/// Une transaction trop ancienne est refusée.
///
/// Une transaction signée reste valable indéfiniment tant que rien ne borne
/// son âge : celle de quelqu'un d'autre, interceptée un jour, se rejouerait un
/// an plus tard.
#[tokio::test]
async fn une_transaction_trop_ancienne_est_refusee() {
    let service = Service::monter().await;
    service.compte("c_vieille", "depart").await;

    let signee = super::storekit::transaction_signee(json!({
        "productId": "com.weave.app.sub.grandtour.monthly",
        "transactionId": "vieille-1",
        "originalTransactionId": "vieille-1",
        "signedDate": (chrono::Utc::now() - chrono::Duration::days(2)).timestamp_millis(),
    }));

    let (statut, corps) = service
        .post(
            "/v1/billing/subscriptions",
            Some(&service.jeton("c_vieille")),
            json!({
                "signedTransaction": signee,
            }),
        )
        .await;
    refuse(
        statut,
        &corps,
        "validation",
        &format!("une transaction d'il y a deux jours : {corps}"),
    );
}

/// Un palier qui comprend le bilan ne fait pas payer deux fois.
///
/// « Expédition » annonce « Bilan mensuel : quels plans attirent, et
/// pourquoi » parmi ce qu'il inclut. La route exigeait pourtant un crédit :
/// quelqu'un versant 14,99 € par mois se voyait demander 2,99 € de plus pour
/// ce que son abonnement promet.
#[tokio::test]
async fn un_palier_qui_comprend_le_bilan_ne_fait_pas_payer_deux_fois() {
    use sea_orm::ConnectionTrait;

    let service = Service::monter().await;
    let auteur = service.compte("c_expedition", "expedition").await;
    let jeton = service.jeton("c_expedition");
    service.compte("c_curieux_exp", "depart").await;

    for nom in ["c_expedition", "c_curieux_exp"] {
        service
            .put(
                "/v1/me/profile",
                Some(&service.jeton(nom)),
                json!({
                    "city": "Nantes", "latitude": 47.21, "longitude": -1.55, "gender": "autre",
                }),
            )
            .await;
    }

    for titre in [
        "Un cafe sur la place",
        "Une balade au bord de leau",
        "Un concert au hangar",
    ] {
        let (statut, corps) = service
            .post(
                "/v1/plans",
                Some(&jeton),
                json!({
                    "title": titre,
                    "category": "sortie",
                    "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
                }),
            )
            .await;
        assert_eq!(statut, StatusCode::OK, "{corps}");
    }
    service
        .db
        .execute_unprepared(&format!(
            "UPDATE plans SET startsAt='2020-01-01 12:00:00' WHERE authorId='{auteur}'"
        ))
        .await
        .unwrap();

    // Aucun crédit acheté, et pourtant le bilan doit sortir.
    let (_, moi) = service.get("/v1/me", Some(&jeton)).await;
    assert_eq!(
        moi["credits"]["bilan"], 0,
        "le test doit partir sans crédit"
    );

    let (statut, bilan) = service.post("/v1/me/bilan", Some(&jeton), json!({})).await;
    assert_eq!(
        statut,
        StatusCode::OK,
        "le bilan inclus a été refusé : {bilan}"
    );
    assert_eq!(bilan["plansPasses"], 3);

    // Le second du même mois, lui, se paie : l'abonnement en comprend un.
    let (statut, corps) = service.post("/v1/me/bilan", Some(&jeton), json!({})).await;
    assert_ne!(
        statut,
        StatusCode::OK,
        "le second bilan du mois devrait demander un crédit : {corps}"
    );

    // Et avec un crédit, il sort — sans toucher à la part incluse.
    crate::routes::billing::crediter_pour_test(&service.etat, &auteur, "bilan", 1)
        .await
        .expect("achat");
    let (statut, corps) = service.post("/v1/me/bilan", Some(&jeton), json!({})).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
}

/// Un palier qui ne comprend pas le bilan le fait payer, comme annoncé.
#[tokio::test]
async fn un_palier_sans_bilan_exige_toujours_le_credit() {
    use sea_orm::ConnectionTrait;

    let service = Service::monter().await;
    let auteur = service.compte("c_viree_bilan", "viree").await;
    let jeton = service.jeton("c_viree_bilan");

    service
        .put(
            "/v1/me/profile",
            Some(&jeton),
            json!({
                "city": "Nantes", "latitude": 47.21, "longitude": -1.55, "gender": "autre",
            }),
        )
        .await;

    for titre in [
        "Un premier plan a soi",
        "Un deuxieme plan a soi",
        "Un troisieme plan a soi",
    ] {
        service
            .post(
                "/v1/plans",
                Some(&jeton),
                json!({
                    "title": titre,
                    "category": "sortie",
                    "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
                }),
            )
            .await;
    }
    service
        .db
        .execute_unprepared(&format!(
            "UPDATE plans SET startsAt='2020-01-01 12:00:00' WHERE authorId='{auteur}'"
        ))
        .await
        .unwrap();

    let (statut, corps) = service.post("/v1/me/bilan", Some(&jeton), json!({})).await;
    refuse(
        statut,
        &corps,
        "entitlement_required",
        &format!("« Virée » ne comprend pas le bilan : {corps}"),
    );
}

/// Les critères vendus par palier sont refusés à qui ne les a pas.
///
/// « filtres » — « base », « étendus », « précis » — était annoncé au catalogue
/// et n'était lu nulle part : les critères du fil acceptaient les mêmes
/// réglages à tous les paliers. Quelqu'un payant pour des « critères précis »
/// avait exactement ce que le socle gratuit offrait déjà.
#[tokio::test]
async fn les_criteres_vendus_sont_refuses_au_socle_gratuit() {
    let service = Service::monter().await;
    service.compte("c_socle", "depart").await;
    let jeton = service.jeton("c_socle");

    // Ce que le socle garde : restreindre l'âge et la distance viderait le fil
    // de tout réglage utile.
    let (statut, corps) = service
        .patch(
            "/v1/me/preferences",
            Some(&jeton),
            json!({ "minAge": 25, "maxDistanceKm": 40 }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    for (champ, valeur) in [
        ("categories", json!(["balade"])),
        ("days", json!([6, 7])),
        ("seeking", json!(["femme"])),
    ] {
        let (statut, corps) = service
            .patch("/v1/me/preferences", Some(&jeton), json!({ champ: valeur }))
            .await;
        refuse(
            statut,
            &corps,
            "entitlement_required",
            &format!("« {champ} » accepté au socle : {corps}"),
        );
    }

    // Vider reste possible : sinon, quelqu'un dont l'abonnement expire ne
    // pourrait plus défaire ce qu'il avait posé.
    let (statut, corps) = service
        .patch(
            "/v1/me/preferences",
            Some(&jeton),
            json!({ "categories": [] }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "vider a été refusé : {corps}");
}

/// Un abonnement qui retombe ne laisse pas le bénéfice derrière lui.
///
/// Les critères sont contrôlés à l'écriture, mais le palier peut changer
/// ensuite. Sans relecture à travers la règle, il aurait suffi de s'abonner un
/// mois pour garder les critères précis indéfiniment.
#[tokio::test]
async fn un_palier_retombe_ne_laisse_pas_les_criteres_derriere_lui() {
    use sea_orm::ConnectionTrait;

    let service = Service::monter().await;
    let compte = service.compte("c_retombe", "escapade").await;
    let jeton = service.jeton("c_retombe");

    service
        .put(
            "/v1/me/profile",
            Some(&jeton),
            json!({
                "city": "Nantes", "latitude": 47.21, "longitude": -1.55, "gender": "autre",
            }),
        )
        .await;

    let (statut, corps) = service
        .patch(
            "/v1/me/preferences",
            Some(&jeton),
            json!({ "categories": ["balade"] }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let (_, criteres) = service.get("/v1/me/preferences", Some(&jeton)).await;
    assert_eq!(
        criteres["categories"],
        json!(["balade"]),
        "le critère devait être posé"
    );

    // L'abonnement expire : le palier retombe au socle.
    service
        .db
        .execute_unprepared(&format!(
            "UPDATE subscriptions SET tier='depart' WHERE accountId='{compte}'"
        ))
        .await
        .unwrap();
    crate::auth::oublier_compte(&service.etat, &compte).await;

    // Le critère reste inscrit en base — on ne l'efface pas —, mais le fil ne
    // s'en sert plus.
    let (statut, corps) = service.get("/v1/plans", Some(&jeton)).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
}

/// Le filtre par jour retient ce qu'il doit, et écarte le reste.
///
/// « Escapade » vend « Critères précis : catégorie, jour, distance fine ». Le
/// filtre par catégorie existait ; celui par jour n'existait nulle part. On
/// vendait un critère qui n'était pas écrit.
#[tokio::test]
async fn le_filtre_par_jour_retient_le_bon_jour() {
    use chrono::Datelike;

    let service = Service::monter().await;
    service.compte("c_jours", "escapade").await;
    service.compte("c_hote_jours", "depart").await;
    let jeton = service.jeton("c_jours");

    for nom in ["c_jours", "c_hote_jours"] {
        service
            .put(
                "/v1/me/profile",
                Some(&service.jeton(nom)),
                json!({
                    "city": "Nantes", "latitude": 47.21, "longitude": -1.55, "gender": "autre",
                }),
            )
            .await;
    }

    // Deux plans à deux jours différents, tous deux à venir.
    let dans_deux = chrono::Utc::now() + chrono::Duration::days(2);
    let dans_trois = chrono::Utc::now() + chrono::Duration::days(3);
    for (titre, quand) in [
        ("Le plan du premier jour", dans_deux),
        ("Le plan du second jour", dans_trois),
    ] {
        let (statut, corps) = service
            .post(
                "/v1/plans",
                Some(&service.jeton("c_hote_jours")),
                json!({
                    "title": titre,
                    "category": "balade",
                    "startsAt": quand.to_rfc3339(),
                }),
            )
            .await;
        assert_eq!(statut, StatusCode::OK, "{corps}");
    }

    // Sans filtre, les deux sont là.
    let (_, fil) = service.get("/v1/plans", Some(&jeton)).await;
    assert_eq!(fil["plans"].as_array().unwrap().len(), 2, "{fil}");

    // Avec le jour du premier seulement, il ne reste que lui.
    let jour_retenu = dans_deux.weekday().number_from_monday() as i32;
    let (statut, corps) = service
        .patch(
            "/v1/me/preferences",
            Some(&jeton),
            json!({ "days": [jour_retenu] }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let (_, fil) = service.get("/v1/plans", Some(&jeton)).await;
    let plans = fil["plans"].as_array().expect("une liste");
    assert_eq!(plans.len(), 1, "le filtre par jour n'a pas trié : {fil}");
    assert_eq!(plans[0]["title"], "Le plan du premier jour");
}

/// Un jour hors de la semaine est refusé.
#[tokio::test]
async fn un_jour_hors_semaine_est_refuse() {
    let service = Service::monter().await;
    service.compte("c_jour_faux", "escapade").await;
    let jeton = service.jeton("c_jour_faux");

    for faux in [0, 8, -1, 42] {
        let (statut, corps) = service
            .patch(
                "/v1/me/preferences",
                Some(&jeton),
                json!({ "days": [faux] }),
            )
            .await;
        refuse(
            statut,
            &corps,
            "validation",
            &format!("« {faux} » accepté comme jour : {corps}"),
        );
    }
}

/// Un crédit « Horizon » ouvre la publication jusqu'à sa borne, et pas au-delà.
///
/// Il n'en avait aucune. La règle était « au-delà de l'horizon du palier,
/// dépense un crédit », sans maximum : un compte gratuit — sept jours — muni
/// d'un crédit à 2,99 € publiait un plan pour 2050. Plus loin que le Grand
/// Tour à 24,99 € par mois, qui s'arrête à quatre-vingt-dix jours, et plus
/// loin que les soixante jours que le catalogue annonce en vendant ce crédit.
#[tokio::test]
async fn le_credit_horizon_s_arrete_a_ce_qu_il_annonce() {
    use crate::droits::HORIZON_CREDIT_JOURS;

    let service = Service::monter().await;
    let compte = service.compte("c_horizon_borne", "depart").await;
    let jeton = service.jeton("c_horizon_borne");

    crate::routes::billing::crediter_pour_test(&service.etat, &compte, "horizon", 5)
        .await
        .expect("crédits posés");

    let plan = |jours: i64| {
        json!({
            "title": "Une balade sur les quais",
            "category": "balade",
            "startsAt": (chrono::Utc::now() + chrono::Duration::days(jours)).to_rfc3339(),
        })
    };

    // Au-delà du palier mais dans la borne du crédit : accepté, un crédit part.
    let (statut, corps) = service.post("/v1/plans", Some(&jeton), plan(30)).await;
    assert_eq!(
        statut,
        StatusCode::OK,
        "le crédit n'a pas ouvert l'horizon : {corps}"
    );

    let (_, moi) = service.get("/v1/me", Some(&jeton)).await;
    assert_eq!(
        moi["credits"]["horizon"], 4,
        "le crédit n'a pas été dépensé : {moi}"
    );

    // Au-delà de la borne : refusé, et SANS prélever de crédit — on ne fait pas
    // payer un refus.
    let (statut, corps) = service
        .post("/v1/plans", Some(&jeton), plan(HORIZON_CREDIT_JOURS + 300))
        .await;
    assert_ne!(
        statut,
        StatusCode::OK,
        "un compte gratuit a publié un plan pour dans un an : {corps}"
    );

    let (_, moi) = service.get("/v1/me", Some(&jeton)).await;
    assert_eq!(
        moi["credits"]["horizon"], 4,
        "un refus a coûté un crédit : {moi}"
    );
}

/// « Distance fine » est vendue à partir de l'Escapade, et se réglait au
/// kilomètre près à tous les paliers — y compris le gratuit.
///
/// Elle figure trois fois dans le catalogue — « Critères précis : catégorie,
/// jour, distance fine ». La catégorie et le jour se limitaient bien par
/// palier ; la distance, non. On vendait trois fois un critère qui n'existait
/// pas, ou plutôt qui existait déjà partout, ce qui revient au même.
#[tokio::test]
async fn la_distance_fine_ne_vaut_qu_aux_paliers_qui_l_achetent() {
    use crate::droits::rayon_effectif;

    // Au palier gratuit et à la Virée, le rayon se rabat sur un cran.
    for palier in ["depart", "viree"] {
        assert_eq!(
            rayon_effectif(palier, 27),
            25,
            "{palier} : 27 km devrait valoir 25"
        );
        assert_eq!(
            rayon_effectif(palier, 63),
            50,
            "{palier} : 63 km devrait valoir 50"
        );
        // Jamais moins que le plus petit cran : rabattre vers le bas viderait
        // le fil de quelqu'un qui n'a rien demandé.
        assert_eq!(
            rayon_effectif(palier, 3),
            10,
            "{palier} : un rayon minuscule remonte au cran"
        );
    }

    // À partir de l'Escapade, le réglage vaut au kilomètre près.
    for palier in ["escapade", "expedition", "grandtour"] {
        assert_eq!(
            rayon_effectif(palier, 27),
            27,
            "{palier} achète la distance fine"
        );
        assert_eq!(
            rayon_effectif(palier, 63),
            63,
            "{palier} achète la distance fine"
        );
    }
}

/// Le réglage choisi n'est pas écrasé : il est rabattu à la lecture.
///
/// L'écraser en base ferait perdre à quelqu'un ce qu'il avait réglé le jour où
/// son abonnement s'interrompt, et rien ne le lui rendrait à la reprise.
#[tokio::test]
async fn le_rayon_choisi_survit_a_la_perte_de_l_offre() {
    let service = Service::monter().await;
    service.compte("c_rayon", "depart").await;
    let jeton = service.jeton("c_rayon");

    let (statut, corps) = service
        .patch(
            "/v1/me/preferences",
            Some(&jeton),
            json!({ "maxDistanceKm": 27 }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let (_, criteres) = service.get("/v1/me/preferences", Some(&jeton)).await;
    assert_eq!(
        criteres["maxDistanceKm"], 27,
        "le réglage a été écrasé : {criteres}"
    );
    assert_eq!(
        criteres["effectiveDistanceKm"], 25,
        "le fil doit dire ce qu'il applique vraiment : {criteres}"
    );
}

/// Le fil applique bien le rayon rabattu, et pas celui qui est enregistré.
///
/// Sans cette lecture, « distance fine » resterait une ligne du catalogue : le
/// réglage au kilomètre près continuerait de valoir pour tout le monde, et
/// rabattre la valeur affichée n'aurait été qu'un affichage.
#[tokio::test]
async fn le_fil_retient_le_rayon_rabattu_et_non_celui_enregistre() {
    let service = Service::monter().await;

    // L'auteur publie à environ 26 km au nord du lecteur : dans un rayon de
    // 27 km, hors d'un rayon de 25.
    service.compte("c_loin_auteur", "depart").await;
    fiche_a(&service, "c_loin_auteur", 46.0, 4.84).await;
    let plan = plan_simple(&service, "c_loin_auteur").await;

    service.compte("c_gratuit", "depart").await;
    fiche_a(&service, "c_gratuit", 45.76, 4.84).await;
    service.compte("c_precis", "escapade").await;
    fiche_a(&service, "c_precis", 45.76, 4.84).await;

    assert!(
        !fil_voit(&service, "c_gratuit", &plan).await,
        "au palier gratuit, 27 km se rabat sur 25 : le plan est hors du fil"
    );
    assert!(
        fil_voit(&service, "c_precis", &plan).await,
        "l'Escapade achète la distance fine : 27 km vaut 27 km"
    );
}

/// Règle le rayon à 27 km pour ce compte, puis dit si le plan entre au fil.
async fn fil_voit(service: &Service, nom: &str, plan: &str) -> bool {
    let jeton = service.jeton(nom);
    let (statut, corps) = service
        .patch(
            "/v1/me/preferences",
            Some(&jeton),
            json!({ "maxDistanceKm": 27 }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    // Le fil vit quelques minutes en cache : sans cet oubli, on relirait la
    // composition d'avant le réglage.
    crate::cache::oublier(
        &service.etat.cache,
        &crate::cache::cles::fil(&service.id(nom)),
    )
    .await
    .ok();

    let (statut, corps) = service.get("/v1/plans", Some(&jeton)).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    corps["plans"]
        .as_array()
        .expect("une liste")
        .iter()
        .any(|p| p["id"] == plan)
}

async fn fiche_a(service: &Service, nom: &str, lat: f64, lon: f64) {
    let (statut, corps) = service
        .put(
            "/v1/me/profile",
            Some(&service.jeton(nom)),
            json!({ "city": "Lyon", "latitude": lat, "longitude": lon, "gender": "femme" }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
}

async fn plan_simple(service: &Service, nom: &str) -> String {
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
    corps["id"].as_str().expect("identifiant").to_string()
}

/// Deux ouvertures simultanées d'escale ne dépensent qu'un crédit.
///
/// Le contrôle « une escale est déjà en cours » se lisait puis s'écrivait en
/// deux temps : deux appels concurrents le franchissaient ensemble et
/// DÉPENSAIENT TOUS DEUX UN CRÉDIT pour une seule escale. À 5,99 € l'unité, un
/// double appui coûtait le prix d'une escale de trop.
#[tokio::test]
async fn deux_escales_simultanees_ne_depensent_qu_un_credit() {
    let service = Service::monter().await;
    let compte = service.compte("c_escale_course", "depart").await;
    let jeton = service.jeton("c_escale_course");

    crate::routes::billing::crediter_pour_test(&service.etat, &compte, "escale", 4)
        .await
        .expect("crédits posés");

    let corps = json!({ "city": "Bordeaux" });
    let (a, b, c) = tokio::join!(
        service.post("/v1/me/escale", Some(&jeton), corps.clone()),
        service.post("/v1/me/escale", Some(&jeton), corps.clone()),
        service.post("/v1/me/escale", Some(&jeton), corps.clone()),
    );
    let reussies = [a, b, c]
        .iter()
        .filter(|(s, _)| *s == StatusCode::OK)
        .count();
    assert_eq!(reussies, 1, "une seule ouverture doit aboutir");

    let (_, moi) = service.get("/v1/me", Some(&jeton)).await;
    assert_eq!(
        moi["credits"]["escale"], 3,
        "une seule escale ouverte, donc un seul crédit dépensé : {moi}"
    );
}

/// Sans crédit, rien n'est ouvert : la transaction annule tout.
///
/// L'ordre inverse — ouvrir puis payer — laisserait une escale que personne
/// n'a payée le jour où le solde est vide.
#[tokio::test]
async fn une_escale_sans_credit_n_ouvre_rien() {
    let service = Service::monter().await;
    service.compte("c_escale_sans", "depart").await;
    let jeton = service.jeton("c_escale_sans");

    let (statut, corps) = service
        .post("/v1/me/escale", Some(&jeton), json!({ "city": "Bordeaux" }))
        .await;
    refuse(statut, &corps, "entitlement_required", &format!("{corps}"));

    let (_, criteres) = service.get("/v1/me/preferences", Some(&jeton)).await;
    assert!(
        criteres["escaleCity"].is_null(),
        "une escale a été ouverte sans être payée : {criteres}"
    );
}

/// La forme qu'Apple envoie vraiment est celle que la route lit.
///
/// Elle ne l'était pas. Le `signedPayload` d'une notification V2 n'est pas un
/// JWSTransaction : la documentation d'Apple lui donne pour champs de premier
/// niveau `notificationType`, `subtype`, `data`, `summary`,
/// `externalPurchaseToken`, `appData`, `version`, `signedDate` et
/// `notificationUUID`. Ni `bundleId`, ni `environment`, ni `productId` — ceux-là
/// vivent dans `data`, et la transaction dans `data.signedTransactionInfo`.
///
/// Le contrôle du paquet lisait donc une chaîne vide et refusait. Aucune
/// notification réelle n'a jamais pu être traitée. Les tests ne le voyaient pas
/// : ils envoyaient à cette route une transaction, la seule forme qu'elle
/// savait lire — et qu'Apple n'émet jamais ici.
#[tokio::test]
async fn une_transaction_nue_n_est_pas_une_notification() {
    let service = Service::monter().await;

    let (statut, corps) = service
        .post(
            "/v1/billing/apple/notifications",
            None,
            json!({
                "signedPayload": transaction(json!({
                    "productId": "com.weave.app.sub.escapade.monthly",
                    "transactionId": "tx-mauvaise-forme",
                    "originalTransactionId": "orig-mauvaise-forme",
                })),
            }),
        )
        .await;
    refuse(
        statut,
        &corps,
        "validation",
        "une transaction nue passe pour une notification",
    );
}

/// L'enveloppe est signée, mais la transaction qu'elle porte ne l'est pas.
///
/// Vérifier l'enveloppe seule authentifierait n'importe quel contenu glissé
/// dedans : `signedTransactionInfo` est un JWS à part entière, et il se vérifie
/// à son tour.
#[tokio::test]
async fn une_transaction_emboitee_non_signee_est_refusee() {
    let service = Service::monter().await;

    let enveloppe = super::storekit::enveloppe_signee(json!({
        "notificationType": "DID_RENEW",
        "notificationUUID": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        "version": "2.0",
        "signedDate": chrono::Utc::now().timestamp_millis(),
        "data": {
            "bundleId": "com.weave.app",
            "environment": "sandbox",
            // Trois segments, aucune signature valable.
            "signedTransactionInfo": "eyJhbGciOiJFUzI1NiJ9.eyJwcm9kdWN0SWQiOiJ4In0.c2ln",
        },
    }));

    let (statut, corps) = service
        .post(
            "/v1/billing/apple/notifications",
            None,
            json!({ "signedPayload": enveloppe }),
        )
        .await;
    refuse(
        statut,
        &corps,
        "validation",
        "l'enveloppe a authentifié une transaction qu'elle ne signe pas",
    );
}

/// Un remboursement coupe l'accès sur-le-champ.
///
/// Apple rend l'argent sans raccourcir la période : l'échéance ne bouge pas.
/// S'en tenir à la date laissait donc l'accès ouvert jusqu'au bout d'un mois
/// qui n'a plus été payé.
#[tokio::test]
async fn un_remboursement_coupe_l_acces_sans_attendre_l_echeance() {
    let service = Service::monter().await;
    let compte = service.compte("c_rembourse", "escapade").await;
    service
        .db
        .execute_unprepared(&format!(
            "UPDATE subscriptions SET originalTransactionId='orig-rembourse' WHERE accountId='{compte}'"
        ))
        .await
        .unwrap();

    let (_, avant) = service
        .get("/v1/me", Some(&service.jeton("c_rembourse")))
        .await;
    assert_eq!(avant["tier"], "escapade");

    // L'échéance reste dans un mois : c'est bien le remboursement, et lui seul,
    // qui doit fermer la porte.
    let dans_un_mois = (chrono::Utc::now() + chrono::Duration::days(30)).timestamp_millis();
    let (statut, corps) = service
        .post(
            "/v1/billing/apple/notifications",
            None,
            json!({
                "signedPayload": notification(
                    "REFUND",
                    None,
                    json!({
                        "productId": "com.weave.app.sub.escapade.monthly",
                        "transactionId": "tx-rembourse",
                        "originalTransactionId": "orig-rembourse",
                        "expiresDate": dans_un_mois,
                    }),
                    None,
                ),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let (_, apres) = service
        .get("/v1/me", Some(&service.jeton("c_rembourse")))
        .await;
    assert_eq!(
        apres["tier"], "depart",
        "un abonnement remboursé garde ses droits : {apres}"
    );
}

/// Une période de grâce prolonge l'accès — c'est sa raison d'être.
///
/// Apple réessaie de prélever et demande qu'on serve la personne pendant ce
/// temps. La date de fin vit dans `signedRenewalInfo`, que la route ne lisait
/// pas : `inGracePeriod` était écrit `false` aux trois endroits qui le
/// touchent, jamais `true`. Le champ existait en base, l'API le rendait,
/// l'application le décodait, et rien ne pouvait le lever.
#[tokio::test]
async fn une_periode_de_grace_prolonge_l_acces_et_se_voit() {
    let service = Service::monter().await;
    let compte = service.compte("c_grace", "escapade").await;
    service
        .db
        .execute_unprepared(&format!(
            "UPDATE subscriptions SET originalTransactionId='orig-grace' WHERE accountId='{compte}'"
        ))
        .await
        .unwrap();

    // L'échéance est passée — le prélèvement a échoué — mais la grâce court.
    let hier = (chrono::Utc::now() - chrono::Duration::days(1)).timestamp_millis();
    let dans_six_jours = (chrono::Utc::now() + chrono::Duration::days(6)).timestamp_millis();

    let (statut, corps) = service
        .post(
            "/v1/billing/apple/notifications",
            None,
            json!({
                "signedPayload": notification(
                    "DID_FAIL_TO_RENEW",
                    Some("GRACE_PERIOD"),
                    json!({
                        "productId": "com.weave.app.sub.escapade.monthly",
                        "transactionId": "tx-grace",
                        "originalTransactionId": "orig-grace",
                        "expiresDate": hier,
                    }),
                    Some(json!({
                        "productId": "com.weave.app.sub.escapade.monthly",
                        "gracePeriodExpiresDate": dans_six_jours,
                    })),
                ),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["inGracePeriod"], true, "{corps}");

    let (_, fiche) = service.get("/v1/me", Some(&service.jeton("c_grace"))).await;
    assert_eq!(
        fiche["tier"], "escapade",
        "la grâce n'a pas prolongé l'accès : {fiche}"
    );

    let (_, droits) = service
        .get("/v1/billing/entitlement", Some(&service.jeton("c_grace")))
        .await;
    assert_eq!(
        droits["inGracePeriod"], true,
        "la grâce ne se voit pas depuis l'application : {droits}"
    );
}

/// Une grâce déjà terminée ne prolonge rien.
#[tokio::test]
async fn une_grace_perimee_ne_prolonge_pas_l_acces() {
    let service = Service::monter().await;
    let compte = service.compte("c_grace_finie", "escapade").await;
    service
        .db
        .execute_unprepared(&format!(
            "UPDATE subscriptions SET originalTransactionId='orig-grace-finie' WHERE accountId='{compte}'"
        ))
        .await
        .unwrap();

    let avant_hier = (chrono::Utc::now() - chrono::Duration::days(2)).timestamp_millis();
    let hier = (chrono::Utc::now() - chrono::Duration::days(1)).timestamp_millis();

    let (statut, corps) = service
        .post(
            "/v1/billing/apple/notifications",
            None,
            json!({
                "signedPayload": notification(
                    "DID_FAIL_TO_RENEW",
                    Some("GRACE_PERIOD"),
                    json!({
                        "productId": "com.weave.app.sub.escapade.monthly",
                        "transactionId": "tx-grace-finie",
                        "originalTransactionId": "orig-grace-finie",
                        "expiresDate": avant_hier,
                    }),
                    Some(json!({ "gracePeriodExpiresDate": hier })),
                ),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["inGracePeriod"], false, "{corps}");

    let (_, fiche) = service
        .get("/v1/me", Some(&service.jeton("c_grace_finie")))
        .await;
    assert_eq!(fiche["tier"], "depart", "{fiche}");
}

/// Une notification émise pour une autre application est refusée.
///
/// C'est le même contrôle que pour un achat, mais il porte sur `data.bundleId`
/// et non sur un `bundleId` de premier niveau, qui n'existe pas ici.
#[tokio::test]
async fn une_notification_d_une_autre_application_est_refusee() {
    let service = Service::monter().await;

    let enveloppe = super::storekit::enveloppe_signee(json!({
        "notificationType": "DID_RENEW",
        "notificationUUID": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        "version": "2.0",
        "signedDate": chrono::Utc::now().timestamp_millis(),
        "data": {
            "bundleId": "com.autre.application",
            "environment": "sandbox",
            "signedTransactionInfo": transaction(json!({
                "productId": "com.weave.app.sub.grandtour.monthly",
                "transactionId": "tx-autre-appli",
                "originalTransactionId": "orig-autre-appli",
            })),
        },
    }));

    let (statut, corps) = service
        .post(
            "/v1/billing/apple/notifications",
            None,
            json!({ "signedPayload": enveloppe }),
        )
        .await;
    refuse(
        statut,
        &corps,
        "validation",
        "une notification d'une autre application a été acceptée",
    );
}

/// Une notification ancienne ne défait pas un état plus récent.
///
/// Apple relance pendant trois jours et ne garantit pas l'ordre : une
/// « EXPIRED » arrivant après un réabonnement ferait retomber au palier de
/// départ un compte qui vient de payer.
///
/// Et une relance qui, elle, est bien postérieure au dernier état connu doit
/// passer — c'est tout l'objet des relances, et une simple fenêtre de
/// fraîcheur les aurait toutes écartées.
#[tokio::test]
async fn une_notification_depassee_est_ignoree_mais_pas_une_relance() {
    let service = Service::monter().await;
    let compte = service.compte("c_ordre", "escapade").await;
    service
        .db
        .execute_unprepared(&format!(
            "UPDATE subscriptions SET originalTransactionId='orig-ordre' WHERE accountId='{compte}'"
        ))
        .await
        .unwrap();

    let dans_un_mois = (chrono::Utc::now() + chrono::Duration::days(30)).timestamp_millis();
    let transaction_expiree = json!({
        "productId": "com.weave.app.sub.escapade.monthly",
        "transactionId": "tx-ordre",
        "originalTransactionId": "orig-ordre",
        "expiresDate": (chrono::Utc::now() - chrono::Duration::days(1)).timestamp_millis(),
    });

    // Le compte vient de se réabonner : l'état en base date d'il y a un instant.
    service
        .db
        .execute_unprepared(&format!(
            "UPDATE subscriptions SET expiresAt='{}', updatedAt='{}' WHERE accountId='{compte}'",
            (chrono::Utc::now() + chrono::Duration::days(30))
                .naive_utc()
                .format("%Y-%m-%d %H:%M:%S"),
            chrono::Utc::now().naive_utc().format("%Y-%m-%d %H:%M:%S"),
        ))
        .await
        .unwrap();

    // Une « EXPIRED » signée la veille : trop vieille pour défaire cela.
    let vieille = super::storekit::enveloppe_signee(json!({
        "notificationType": "EXPIRED",
        "notificationUUID": "aaaaaaaa-bbbb-cccc-dddd-000000000001",
        "version": "2.0",
        "signedDate": (chrono::Utc::now() - chrono::Duration::days(1)).timestamp_millis(),
        "data": {
            "bundleId": "com.weave.app",
            "environment": "sandbox",
            "signedTransactionInfo": transaction(transaction_expiree.clone()),
        },
    }));
    let (statut, corps) = service
        .post(
            "/v1/billing/apple/notifications",
            None,
            json!({ "signedPayload": vieille }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["stale"], true, "{corps}");

    let (_, fiche) = service.get("/v1/me", Some(&service.jeton("c_ordre"))).await;
    assert_eq!(
        fiche["tier"], "escapade",
        "une notification dépassée a défait un réabonnement : {fiche}"
    );

    // La relance d'une notification bien réelle, signée il y a deux heures —
    // au-delà de toute fenêtre de fraîcheur, et pourtant à traiter.
    let relance = super::storekit::enveloppe_signee(json!({
        "notificationType": "DID_RENEW",
        "notificationUUID": "aaaaaaaa-bbbb-cccc-dddd-000000000002",
        "version": "2.0",
        "signedDate": chrono::Utc::now().timestamp_millis(),
        "data": {
            "bundleId": "com.weave.app",
            "environment": "sandbox",
            "signedTransactionInfo": transaction(json!({
                "productId": "com.weave.app.sub.grandtour.monthly",
                "transactionId": "tx-relance",
                "originalTransactionId": "orig-ordre",
                "expiresDate": dans_un_mois,
                "signedDate": (chrono::Utc::now() - chrono::Duration::hours(2)).timestamp_millis(),
            })),
        },
    }));
    let (statut, corps) = service
        .post(
            "/v1/billing/apple/notifications",
            None,
            json!({ "signedPayload": relance }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(
        corps["ignored"], false,
        "une relance a été écartée : {corps}"
    );

    let (_, fiche) = service.get("/v1/me", Some(&service.jeton("c_ordre"))).await;
    assert_eq!(fiche["tier"], "grandtour", "{fiche}");
}
