//! Modifier un plan déjà publié.
//!
//! On pouvait publier et annuler, rien entre les deux. Une faute dans le
//! titre, une heure décalée : il fallait annuler et republier — ce qui
//! consomme l'un des trois plans ouverts, perd les personnes acceptées, et
//! leur annonce maintenant « un plan est annulé » pour une coquille.

use super::Service;
use axum::http::StatusCode;
use sea_orm::ConnectionTrait;
use serde_json::json;

async fn plan_de(service: &Service, hote: &str, titre: &str, capacite: i32) -> String {
    let (statut, corps) = service
        .post(
            "/v1/plans",
            Some(&service.jeton(hote)),
            json!({
                "title": titre,
                "category": "balade",
                "capacity": capacite,
                "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    corps["id"].as_str().unwrap().to_string()
}

async fn accepte(service: &Service, hote: &str, invite: &str, plan: &str) -> String {
    let (statut, corps) = service
        .post(
            "/v1/requests",
            Some(&service.jeton(invite)),
            json!({ "planId": plan, "message": "Cette balade me tente beaucoup, je viendrais volontiers." }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    let demande = corps["id"].as_str().unwrap().to_string();
    let (statut, corps) = service
        .post(
            &format!("/v1/requests/{demande}/accept"),
            Some(&service.jeton(hote)),
            json!({}),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    demande
}

async fn colonne(service: &Service, plan: &str, nom: &str) -> Option<String> {
    let ligne = service
        .db
        .query_one_raw(sea_orm::Statement::from_string(
            service.db.get_database_backend(),
            format!("SELECT \"{nom}\" AS v FROM plans WHERE id = '{plan}'"),
        ))
        .await
        .expect("plan lu")
        .expect("plan présent");
    ligne.try_get::<Option<String>>("", "v").unwrap_or(None)
}

#[tokio::test]
async fn corriger_un_titre_ne_previent_personne() {
    let service = Service::monter().await;
    service.compte("c_hote_titre", "depart").await;
    let attendu = service.compte("c_attendu_titre", "depart").await;
    // L'appareil peut recevoir une bannière : l'acceptation passe donc par
    // elle, et non par l'alerte de repli. Sans cela, ce test verrait le oui
    // et croirait que le titre corrigé a prévenu quelqu'un.
    let telephone = service.appareil_avec(&attendu, true).await;
    let plan = plan_de(&service, "c_hote_titre", "Une balade au bord de l eau", 1).await;
    accepte(&service, "c_hote_titre", "c_attendu_titre", &plan).await;

    let (statut, corps) = service
        .patch(
            &format!("/v1/plans/{plan}"),
            Some(&service.jeton("c_hote_titre")),
            json!({ "title": "Une balade au bord de l eau, cote parc" }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let (statut, corps) = service
        .get("/v1/plans/mine", Some(&service.jeton("c_hote_titre")))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps[0]["title"], "Une balade au bord de l eau, cote parc");

    // Une coquille corrigée ne réveille personne. C'est tout l'intérêt : le
    // contournement d'avant — annuler et republier — leur annonçait « un plan
    // est annulé ».
    assert!(
        service.alertes_vers(&telephone).is_empty(),
        "un titre corrigé a fait vibrer le téléphone de quelqu'un"
    );
}

#[tokio::test]
async fn changer_l_heure_previent_les_personnes_attendues() {
    let service = Service::monter().await;
    service.compte("c_hote_heure", "depart").await;
    let attendu = service.compte("c_attendu_heure", "depart").await;
    let telephone = service.appareil_avec(&attendu, true).await;
    let plan = plan_de(&service, "c_hote_heure", "Un concert au parc", 1).await;
    accepte(&service, "c_hote_heure", "c_attendu_heure", &plan).await;

    let (statut, corps) = service
        .patch(
            &format!("/v1/plans/{plan}"),
            Some(&service.jeton("c_hote_heure")),
            json!({ "startsAt": (chrono::Utc::now() + chrono::Duration::days(3)).to_rfc3339() }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    // Elles ont noté une heure. La changer sans le dire est une façon de les
    // faire venir pour rien.
    let alertes = service.alertes_vers(&telephone);
    assert_eq!(
        alertes.len(),
        1,
        "l'heure a changé en silence : {alertes:?}"
    );
    assert!(alertes[0].0.contains("heure"), "{:?}", alertes[0]);
    // Et l'alerte ne dit pas la nouvelle heure : avec la date du jour, elle
    // dirait à qui regarde par-dessus l'épaule où l'on sera ce soir.
    let assemble = format!("{} {}", alertes[0].0, alertes[0].1);
    assert!(
        !assemble.contains(':'),
        "l'alerte affiche une heure : {assemble}"
    );
}

#[tokio::test]
async fn changer_l_heure_rouvre_le_rappel() {
    let service = Service::monter().await;
    let hote = service.compte("c_hote_rerap", "depart").await;
    let telephone = service.appareil(&hote).await;
    let plan = plan_de(&service, "c_hote_rerap", "Un cafe pres du canal", 1).await;

    // Le plan approche, il est rappelé, et il est ensuite repoussé.
    let proche = (chrono::Utc::now() + chrono::Duration::minutes(60))
        .naive_utc()
        .format("%Y-%m-%d %H:%M:%S");
    service
        .db
        .execute_unprepared(&format!(
            "UPDATE plans SET startsAt = '{proche}' WHERE id = '{plan}'"
        ))
        .await
        .expect("heure rapprochée");
    crate::rappels::executer(&service.etat)
        .await
        .expect("passage");
    assert_eq!(service.alertes_vers(&telephone).len(), 1);
    assert!(colonne(&service, &plan, "remindedAt").await.is_some());

    let (statut, corps) = service
        .patch(
            &format!("/v1/plans/{plan}"),
            Some(&service.jeton("c_hote_rerap")),
            json!({ "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339() }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    // Sans la remise à zéro, le plan garderait la marque de son ancien rappel
    // et personne ne serait prévenu de la nouvelle heure.
    assert!(
        colonne(&service, &plan, "remindedAt").await.is_none(),
        "la marque de rappel a survécu au changement d'heure"
    );
}

#[tokio::test]
async fn la_capacite_ne_descend_pas_sous_les_places_accordees() {
    let service = Service::monter().await;
    service.compte("c_hote_cap", "escapade").await;
    service.compte("c_un_cap", "depart").await;
    service.compte("c_deux_cap", "depart").await;
    let plan = plan_de(&service, "c_hote_cap", "Une partie de cartes au bar", 3).await;
    accepte(&service, "c_hote_cap", "c_un_cap", &plan).await;
    accepte(&service, "c_hote_cap", "c_deux_cap", &plan).await;

    // Deux oui donnés : redescendre à une place reviendrait à en décommander
    // un, et il n'existe aucune bonne façon de le lui apprendre.
    let (statut, corps) = service
        .patch(
            &format!("/v1/plans/{plan}"),
            Some(&service.jeton("c_hote_cap")),
            json!({ "capacity": 1 }),
        )
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY, "{corps}");

    // Descendre JUSQU'À ce qui est accordé reste permis : cela ferme le plan
    // aux suivants sans reprendre la parole à personne.
    let (statut, corps) = service
        .patch(
            &format!("/v1/plans/{plan}"),
            Some(&service.jeton("c_hote_cap")),
            json!({ "capacity": 2 }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(
        colonne(&service, &plan, "state").await.as_deref(),
        Some("complet")
    );
}

#[tokio::test]
async fn ajouter_une_place_rouvre_un_plan_complet() {
    let service = Service::monter().await;
    service.compte("c_hote_place", "escapade").await;
    service.compte("c_invite_place", "depart").await;
    let plan = plan_de(&service, "c_hote_place", "Un marche le dimanche", 1).await;
    accepte(&service, "c_hote_place", "c_invite_place", &plan).await;
    assert_eq!(
        colonne(&service, &plan, "state").await.as_deref(),
        Some("complet")
    );

    let (statut, corps) = service
        .patch(
            &format!("/v1/plans/{plan}"),
            Some(&service.jeton("c_hote_place")),
            json!({ "capacity": 2 }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(
        colonne(&service, &plan, "state").await.as_deref(),
        Some("ouvert"),
        "la place ajoutée ne remet pas le plan au fil"
    );

    // Et elle se prend vraiment : rouvrir sans place libre ne servirait à rien.
    service.compte("c_autre_place", "depart").await;
    accepte(&service, "c_hote_place", "c_autre_place", &plan).await;
}

#[tokio::test]
async fn les_bornes_de_publication_valent_aussi_a_la_modification() {
    let service = Service::monter().await;
    service.compte("c_hote_bornes", "depart").await;
    let plan = plan_de(&service, "c_hote_bornes", "Une expo le samedi", 1).await;
    let jeton = service.jeton("c_hote_bornes");
    let chemin = format!("/v1/plans/{plan}");

    // Sans quoi « publier puis modifier » serait le moyen de ne respecter
    // aucune des bornes que la publication impose.
    for (corps, quoi) in [
        (
            json!({ "startsAt": chrono::Utc::now().to_rfc3339() }),
            "le délai minimum",
        ),
        (
            json!({ "startsAt": (chrono::Utc::now() + chrono::Duration::days(3650)).to_rfc3339() }),
            "l'horizon du palier",
        ),
        (json!({ "title": "court" }), "la longueur du titre"),
        (json!({ "capacity": 99 }), "la capacité maximale"),
        (
            json!({ "startsAt": "pas une date" }),
            "le format de la date",
        ),
    ] {
        let (statut, rendu) = service.patch(&chemin, Some(&jeton), corps).await;
        assert_eq!(
            statut,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{quoi} n'est pas tenu à la modification : {rendu}"
        );
    }
}

#[tokio::test]
async fn seul_l_auteur_modifie_son_plan() {
    let service = Service::monter().await;
    service.compte("c_hote_seul", "depart").await;
    service.compte("c_tiers_seul", "depart").await;
    let plan = plan_de(&service, "c_hote_seul", "Un brunch dimanche matin", 1).await;

    let (statut, corps) = service
        .patch(
            &format!("/v1/plans/{plan}"),
            Some(&service.jeton("c_tiers_seul")),
            json!({ "title": "Un titre qui n est pas le sien" }),
        )
        .await;
    assert_eq!(statut, StatusCode::FORBIDDEN, "{corps}");
}

#[tokio::test]
async fn un_plan_annule_ou_passe_ne_se_modifie_plus() {
    let service = Service::monter().await;
    service.compte("c_hote_fini", "depart").await;
    let annule = plan_de(&service, "c_hote_fini", "Une expo le samedi", 1).await;
    let passe = plan_de(&service, "c_hote_fini", "Un concert au parc", 1).await;
    let jeton = service.jeton("c_hote_fini");

    let (statut, _) = service
        .delete(&format!("/v1/plans/{annule}"), Some(&jeton))
        .await;
    assert_eq!(statut, StatusCode::OK);
    service
        .db
        .execute_unprepared(&format!(
            "UPDATE plans SET startsAt = '2020-01-01 00:00:00' WHERE id = '{passe}'"
        ))
        .await
        .expect("heure reculée");

    // L'un n'aura pas lieu, l'autre a déjà eu lieu.
    for (plan, quoi) in [(annule, "annulé"), (passe, "passé")] {
        let (statut, corps) = service
            .patch(
                &format!("/v1/plans/{plan}"),
                Some(&jeton),
                json!({ "title": "Un titre tout neuf pour ce plan" }),
            )
            .await;
        assert_eq!(
            statut,
            StatusCode::UNPROCESSABLE_ENTITY,
            "un plan {quoi} s'est laissé modifier : {corps}"
        );
    }
}

#[tokio::test]
async fn rouvrir_un_plan_complet_ne_contourne_pas_le_plafond() {
    let service = Service::monter().await;
    service.compte("c_hote_plaf", "escapade").await;
    service.compte("c_invite_plaf", "depart").await;

    // Un plan rempli : complet, donc hors du fil, donc hors du plafond.
    let plein = plan_de(&service, "c_hote_plaf", "Un cafe pres du canal", 1).await;
    accepte(&service, "c_hote_plaf", "c_invite_plaf", &plein).await;
    assert_eq!(
        colonne(&service, &plein, "state").await.as_deref(),
        Some("complet")
    );

    // Puis le plafond atteint avec trois autres.
    for titre in [
        "Une expo le samedi",
        "Un concert au parc",
        "Un marche dimanche",
    ] {
        plan_de(&service, "c_hote_plaf", titre, 1).await;
    }

    // Ajouter une place au plan complet le remettrait au fil : quatre plans
    // visibles, alors que trois est la borne. C'est le contournement que la
    // route que je venais d'écrire offrait — trouvé en relisant ce qu'elle
    // touchait, pas par un test qui existait.
    let (statut, corps) = service
        .patch(
            &format!("/v1/plans/{plein}"),
            Some(&service.jeton("c_hote_plaf")),
            json!({ "capacity": 2 }),
        )
        .await;
    assert_ne!(
        statut,
        StatusCode::OK,
        "un quatrième plan est revenu au fil : {corps}"
    );
    assert_eq!(
        colonne(&service, &plein, "state").await.as_deref(),
        Some("complet")
    );
}

#[tokio::test]
async fn corriger_un_titre_reste_possible_au_plafond() {
    let service = Service::monter().await;
    service.compte("c_hote_titre_plaf", "depart").await;
    let mut plans = Vec::new();
    for titre in [
        "Une expo le samedi",
        "Un concert au parc",
        "Un marche dimanche",
    ] {
        plans.push(plan_de(&service, "c_hote_titre_plaf", titre, 1).await);
    }

    // Le contrôle ne porte que sur la RÉOUVERTURE. Un plan déjà ouvert qu'on
    // corrige ne revient pas au fil : il y est. Refuser ici rendrait toute
    // correction impossible à qui a trois plans — c'est-à-dire à qui se sert
    // le plus du produit.
    let (statut, corps) = service
        .patch(
            &format!("/v1/plans/{}", plans[0]),
            Some(&service.jeton("c_hote_titre_plaf")),
            json!({ "title": "Une expo le samedi apres midi" }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
}

#[tokio::test]
async fn changer_la_capacite_d_un_plan_deja_ouvert_reste_possible_au_plafond() {
    let service = Service::monter().await;
    service.compte("c_hote_cap_plaf", "escapade").await;
    let mut plans = Vec::new();
    for titre in [
        "Une expo le samedi",
        "Un concert au parc",
        "Un marche dimanche",
    ] {
        plans.push(plan_de(&service, "c_hote_cap_plaf", titre, 1).await);
    }

    // La correction du titre ne passe même pas par le bloc de capacité : elle
    // ne pouvait donc pas éprouver la précision de la garde. Ma première
    // version s'arrêtait là, et vérifier le plafond SANS condition laissait
    // tous les tests au vert.
    //
    // Ici, le plan est déjà ouvert et sa capacité change. Il est au fil, il y
    // reste : le plafond n'a rien à redire.
    let (statut, corps) = service
        .patch(
            &format!("/v1/plans/{}", plans[0]),
            Some(&service.jeton("c_hote_cap_plaf")),
            json!({ "capacity": 2 }),
        )
        .await;
    assert_eq!(
        statut,
        StatusCode::OK,
        "ajouter une place à un plan DÉJÀ au fil a été refusé : {corps}"
    );
    assert_eq!(
        colonne(&service, &plans[0], "state").await.as_deref(),
        Some("ouvert")
    );
}
