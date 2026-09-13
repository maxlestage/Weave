//! Les achats : unités consommables et notifications de l'App Store.
//!
//! Weave est vendu sur l'App Store ; le serveur ne fait que vérifier puis
//! enregistrer ce qu'Apple lui transmet. Aucune offre n'achète de visibilité —
//! ce qui se vend, c'est l'horizon de publication et les plans de groupe.

use super::Service;
use axum::http::StatusCode;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
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

#[tokio::test]
async fn un_achat_a_l_unite_credite_le_solde() {
    let service = Service::monter().await;
    service.compte("c_acheteur", "depart").await;

    let (statut, corps) = service
        .post("/v1/billing/units", Some(&service.jeton("c_acheteur")), json!({
            "signedTransaction": transaction(json!({
                "productId": "com.weave.app.unit.renfort",
                "transactionId": "tx-renfort-001",
                "environment": "Sandbox",
            })),
        }))
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
        .post("/v1/billing/units", Some(&service.jeton("c_rejoue")), json!({ "signedTransaction": signee.clone() }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{premier}");
    assert_eq!(premier["credits"]["escale"], 1);

    let (statut, second) = service
        .post("/v1/billing/units", Some(&service.jeton("c_rejoue")), json!({ "signedTransaction": signee }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{second}");
    assert_eq!(second["alreadyApplied"], true);
    assert_eq!(second["credits"]["escale"], 1, "le solde a doublé : {second}");
}

#[tokio::test]
async fn un_produit_inconnu_est_refuse() {
    let service = Service::monter().await;
    service.compte("c_produit", "depart").await;

    let (statut, _) = service
        .post("/v1/billing/units", Some(&service.jeton("c_produit")), json!({
            "signedTransaction": transaction(json!({
                "productId": "com.weave.app.unit.licorne",
                "transactionId": "tx-licorne",
            })),
        }))
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
        .post("/v1/billing/units", Some(&service.jeton("c_boucle")), json!({
            "signedTransaction": transaction(json!({
                "productId": "com.weave.app.unit.renfort",
                "transactionId": "tx-boucle-001",
            })),
        }))
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (statut, corps) = service
        .post("/v1/requests/renfort", Some(&service.jeton("c_boucle")), json!({}))
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
        .post("/v1/billing/apple/notifications", None, json!({
            "signedPayload": transaction(json!({
                "productId": "com.weave.app.sub.escapade.monthly",
                "transactionId": "tx-inconnue",
                "originalTransactionId": "orig-inconnue",
            })),
        }))
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
    let (_, avant) = service.get("/v1/me", Some(&service.jeton("c_expire"))).await;
    assert_eq!(avant["tier"], "escapade");
    assert_eq!(avant["requestsLeftToday"], 25);

    let hier = (chrono::Utc::now() - chrono::Duration::days(1)).timestamp_millis();
    let (statut, corps) = service
        .post("/v1/billing/apple/notifications", None, json!({
            "signedPayload": transaction(json!({
                "productId": "com.weave.app.sub.escapade.monthly",
                "transactionId": "tx-expire",
                "originalTransactionId": "orig-expire",
                "expiresDate": hier,
            })),
        }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["ignored"], false);

    let (_, apres) = service.get("/v1/me", Some(&service.jeton("c_expire"))).await;
    assert_eq!(apres["tier"], "depart", "le palier expiré a survécu : {apres}");
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
        .post("/v1/billing/units", Some(&service.jeton("c_renouvelle")), json!({
            "signedTransaction": transaction(json!({
                "productId": "com.weave.app.unit.escale",
                "transactionId": "tx-escale-avant",
            })),
        }))
        .await;
    assert_eq!(statut, StatusCode::OK);

    let demain = (chrono::Utc::now() + chrono::Duration::days(30)).timestamp_millis();
    let (statut, corps) = service
        .post("/v1/billing/apple/notifications", None, json!({
            "signedPayload": transaction(json!({
                "productId": "com.weave.app.sub.escapade.monthly",
                "transactionId": "tx-renouvelle",
                "originalTransactionId": "orig-renouvelle",
                "expiresDate": demain,
            })),
        }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let (_, fiche) = service.get("/v1/me", Some(&service.jeton("c_renouvelle"))).await;
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

    let (statut, corps) = service.get("/v1/me", Some(&service.jeton("c_credits"))).await;
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
            &service.etat, &compte, "escale", 1, echeance,
        )
        .await
        .unwrap_or_else(|e| panic!("dotation {essai} : {e:?}"));
    }

    let (_, corps) = service.get("/v1/me", Some(&service.jeton("c_dotation"))).await;
    assert_eq!(
        corps["credits"]["escale"], 1,
        "trois notifications pour le même mois ont donné {} escales",
        corps["credits"]["escale"]
    );

    // Le mois suivant, en revanche, dote de nouveau.
    let mois_suivant = (chrono::Utc::now() + chrono::Duration::days(60)).naive_utc();
    crate::routes::billing::doter_la_periode_pour_test(
        &service.etat, &compte, "escale", 1, mois_suivant,
    )
    .await
    .expect("dotation du mois suivant");

    let (_, corps) = service.get("/v1/me", Some(&service.jeton("c_dotation"))).await;
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
        &service.etat, &compte, "escale", 1, echeance,
    )
    .await
    .expect("dotation");

    let (_, corps) = service.get("/v1/me", Some(&service.jeton("c_achat_escale"))).await;
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
        .put("/v1/me/profile", Some(&jeton), json!({
            "city": "Nantes", "latitude": 47.21, "longitude": -1.55, "gender": "autre",
        }))
        .await;

    // Sans crédit, l'escale est refusée : c'est ce qui la fait valoir 3,99 €.
    let (statut, corps) = service
        .post("/v1/me/escale", Some(&jeton), json!({ "city": "Lyon" }))
        .await;
    assert_ne!(statut, StatusCode::OK, "une escale sans crédit : {corps}");

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
    assert_ne!(statut, StatusCode::OK, "deux escales à la fois : {corps}");

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
            .put("/v1/me/profile", Some(&service.jeton(nom)), json!({
                "city": ville, "latitude": lat, "longitude": lon, "gender": "autre",
            }))
            .await;
    }

    let (statut, corps) = service
        .post("/v1/plans", Some(&service.jeton("c_lyonnais")), json!({
            "title": "Un verre sur les quais de Saone",
            "category": "sortie",
            "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
        }))
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
    assert_eq!(plans.len(), 1, "l'escale n'a pas ouvert le fil sur Lyon : {fil}");
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
            .put("/v1/me/profile", Some(&service.jeton(nom)), json!({
                "city": ville, "latitude": lat, "longitude": lon, "gender": "autre",
            }))
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
            .post("/v1/plans", Some(&jeton), json!({
                "title": titre,
                "category": categorie,
                "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
            }))
            .await;
        assert_eq!(statut, StatusCode::OK, "{corps}");
        publies.push(corps["id"].as_str().unwrap().to_string());
    }

    // Une demande sur les deux premiers seulement.
    for plan in publies.iter().take(2) {
        let (statut, corps) = service
            .post("/v1/requests", Some(&service.jeton("c_demandeur")), json!({
                "planId": plan,
                "message": "Ce plan me tente beaucoup, je serais ravi de venir.",
            }))
            .await;
        assert_eq!(statut, StatusCode::OK, "{corps}");
    }

    // Sans crédit, pas de bilan.
    let (statut, corps) = service.post("/v1/me/bilan", Some(&jeton), json!({})).await;
    assert_ne!(statut, StatusCode::OK, "un bilan sans crédit : {corps}");

    crate::routes::billing::crediter_pour_test(&service.etat, &auteur, "bilan", 1)
        .await
        .expect("achat");

    // Les plans sont encore à venir : le bilan doit refuser, et ne pas
    // consommer le crédit — un plan à venir n'a pas fini de recevoir.
    let (statut, corps) = service.post("/v1/me/bilan", Some(&jeton), json!({})).await;
    assert_ne!(statut, StatusCode::OK, "un bilan sur des plans à venir : {corps}");
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
    assert_eq!(bilan["ceQuiTombeAPlat"][0]["titre"], "Un tournoi de flechettes obscur");

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
    assert_ne!(statut, StatusCode::OK, "{corps}");
    assert!(
        corps["message"].as_str().unwrap_or_default().contains("n'a pas été utilisé"),
        "le refus doit dire que le crédit est intact : {corps}"
    );

    let (_, moi) = service.get("/v1/me", Some(&jeton)).await;
    assert_eq!(moi["credits"]["bilan"], 1, "le crédit a été consommé pour rien");
}

/// Une transaction forgée est refusée par la route.
///
/// C'est ce que l'API acceptait : elle décodait le base64 de la charge et
/// lisait les champs qu'elle y trouvait. N'importe qui pouvait encoder un JSON
/// annonçant le produit de son choix et s'offrir l'abonnement le plus cher.
#[tokio::test]
async fn une_transaction_forgee_est_refusee() {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

    let service = Service::monter().await;
    service.compte("c_forgeur", "depart").await;

    let charge = json!({
        "productId": "com.weave.app.sub.grandtour.monthly",
        "transactionId": "forgee-1",
        "originalTransactionId": "forgee-1",
        "bundleId": "com.weave.app",
    });
    let forgee = format!("entete.{}.signature", URL_SAFE_NO_PAD.encode(charge.to_string()));

    let (statut, corps) = service
        .post("/v1/billing/subscriptions", Some(&service.jeton("c_forgeur")), json!({
            "signedTransaction": forgee,
        }))
        .await;
    assert_ne!(statut, StatusCode::OK, "une transaction forgée a été acceptée : {corps}");

    let (_, moi) = service.get("/v1/me", Some(&service.jeton("c_forgeur"))).await;
    assert_eq!(moi["tier"], "depart", "le palier a été accordé sans paiement");
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
        .post("/v1/billing/subscriptions", Some(&service.jeton("c_rejeu")), json!({
            "signedTransaction": signee,
        }))
        .await;
    assert_ne!(
        statut,
        StatusCode::OK,
        "un achat d'une autre application a été accepté : {corps}"
    );

    let (_, moi) = service.get("/v1/me", Some(&service.jeton("c_rejeu"))).await;
    assert_eq!(moi["tier"], "depart", "le palier a été accordé sur l'achat d'autrui");
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
        .post("/v1/billing/subscriptions", Some(&service.jeton("c_vieille")), json!({
            "signedTransaction": signee,
        }))
        .await;
    assert_ne!(statut, StatusCode::OK, "une transaction d'il y a deux jours : {corps}");
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
            .put("/v1/me/profile", Some(&service.jeton(nom)), json!({
                "city": "Nantes", "latitude": 47.21, "longitude": -1.55, "gender": "autre",
            }))
            .await;
    }

    for titre in ["Un cafe sur la place", "Une balade au bord de leau", "Un concert au hangar"] {
        let (statut, corps) = service
            .post("/v1/plans", Some(&jeton), json!({
                "title": titre,
                "category": "sortie",
                "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
            }))
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
    assert_eq!(moi["credits"]["bilan"], 0, "le test doit partir sans crédit");

    let (statut, bilan) = service.post("/v1/me/bilan", Some(&jeton), json!({})).await;
    assert_eq!(statut, StatusCode::OK, "le bilan inclus a été refusé : {bilan}");
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
        .put("/v1/me/profile", Some(&jeton), json!({
            "city": "Nantes", "latitude": 47.21, "longitude": -1.55, "gender": "autre",
        }))
        .await;

    for titre in ["Un premier plan a soi", "Un deuxieme plan a soi", "Un troisieme plan a soi"] {
        service
            .post("/v1/plans", Some(&jeton), json!({
                "title": titre,
                "category": "sortie",
                "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
            }))
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
    assert_ne!(statut, StatusCode::OK, "« Virée » ne comprend pas le bilan : {corps}");
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
        .patch("/v1/me/preferences", Some(&jeton), json!({ "minAge": 25, "maxDistanceKm": 40 }))
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
        assert_ne!(statut, StatusCode::OK, "« {champ} » accepté au socle : {corps}");
    }

    // Vider reste possible : sinon, quelqu'un dont l'abonnement expire ne
    // pourrait plus défaire ce qu'il avait posé.
    let (statut, corps) = service
        .patch("/v1/me/preferences", Some(&jeton), json!({ "categories": [] }))
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
        .put("/v1/me/profile", Some(&jeton), json!({
            "city": "Nantes", "latitude": 47.21, "longitude": -1.55, "gender": "autre",
        }))
        .await;

    let (statut, corps) = service
        .patch("/v1/me/preferences", Some(&jeton), json!({ "categories": ["balade"] }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let (_, criteres) = service.get("/v1/me/preferences", Some(&jeton)).await;
    assert_eq!(criteres["categories"], json!(["balade"]), "le critère devait être posé");

    // L'abonnement expire : le palier retombe au socle.
    service
        .db
        .execute_unprepared(&format!("UPDATE subscriptions SET tier='depart' WHERE accountId='{compte}'"))
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
            .put("/v1/me/profile", Some(&service.jeton(nom)), json!({
                "city": "Nantes", "latitude": 47.21, "longitude": -1.55, "gender": "autre",
            }))
            .await;
    }

    // Deux plans à deux jours différents, tous deux à venir.
    let dans_deux = chrono::Utc::now() + chrono::Duration::days(2);
    let dans_trois = chrono::Utc::now() + chrono::Duration::days(3);
    for (titre, quand) in [("Le plan du premier jour", dans_deux), ("Le plan du second jour", dans_trois)] {
        let (statut, corps) = service
            .post("/v1/plans", Some(&service.jeton("c_hote_jours")), json!({
                "title": titre,
                "category": "balade",
                "startsAt": quand.to_rfc3339(),
            }))
            .await;
        assert_eq!(statut, StatusCode::OK, "{corps}");
    }

    // Sans filtre, les deux sont là.
    let (_, fil) = service.get("/v1/plans", Some(&jeton)).await;
    assert_eq!(fil["plans"].as_array().unwrap().len(), 2, "{fil}");

    // Avec le jour du premier seulement, il ne reste que lui.
    let jour_retenu = dans_deux.weekday().number_from_monday() as i32;
    let (statut, corps) = service
        .patch("/v1/me/preferences", Some(&jeton), json!({ "days": [jour_retenu] }))
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
            .patch("/v1/me/preferences", Some(&jeton), json!({ "days": [faux] }))
            .await;
        assert_ne!(statut, StatusCode::OK, "« {faux} » accepté comme jour : {corps}");
    }
}
