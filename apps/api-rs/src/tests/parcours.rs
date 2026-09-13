//! Le parcours de Weave, exercé de bout en bout.
//!
//! Ces tests ne vérifient pas que le code compile — le compilateur le fait
//! déjà. Ils vérifient ce qu'aucune compilation ne peut dire : que les
//! invariants du produit tiennent quand on passe par HTTP.

use super::*;
use serde_json::json;

#[tokio::test]
async fn la_sonde_de_sante_dit_ce_qui_repond() {
    let service = Service::monter().await;
    let (statut, corps) = service.get("/health", None).await;

    assert_eq!(statut, StatusCode::OK);
    assert_eq!(corps["status"], "ok");
    assert_eq!(corps["database"]["ok"], true);
    // Le cache n'est pas un confort : il se déclare requis.
    assert_eq!(corps["cache"]["required"], true);
}

#[tokio::test]
async fn sans_jeton_valable_rien_n_est_accessible() {
    let service = Service::monter().await;

    for chemin in ["/v1/me", "/v1/plans"] {
        let (statut, _) = service.get(chemin, None).await;
        assert_eq!(statut, StatusCode::UNAUTHORIZED, "{chemin} sans jeton");

        let (statut, _) = service.get(chemin, Some("pas-un-jeton")).await;
        assert_eq!(statut, StatusCode::UNAUTHORIZED, "{chemin} avec un faux jeton");
    }
}

#[tokio::test]
async fn un_jeton_valable_pour_un_compte_inexistant_est_refuse() {
    // Un compte supprimé laisse des jetons valides en circulation : ils ne
    // doivent plus ouvrir la porte.
    let service = Service::monter().await;
    let (statut, _) = service.get("/v1/me", Some(&service.jeton("compte_fantome"))).await;
    assert_eq!(statut, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn la_fiche_rend_le_palier_et_le_quota_du_palier() {
    let service = Service::monter().await;
    service.compte("c_fiche", "escapade").await;

    let (statut, corps) = service.get("/v1/me", Some(&service.jeton("c_fiche"))).await;

    assert_eq!(statut, StatusCode::OK);
    assert_eq!(corps["handle"], service.id("c_fiche"));
    assert_eq!(corps["tier"], "escapade");
    // Escapade donne 25 demandes par jour, aucune encore dépensée.
    assert_eq!(corps["requestsLeftToday"], 25);
    // Les dates sortent au format de `toISOString()`, que lisent le site et
    // l'application iOS.
    assert!(
        corps["createdAt"].as_str().unwrap().ends_with('Z'),
        "createdAt valait {}",
        corps["createdAt"]
    );
}

#[tokio::test]
async fn trois_plans_ouverts_et_pas_un_de_plus() {
    let service = Service::monter().await;
    service.compte("c_plans", "grandtour").await;
    let jeton = &service.jeton("c_plans");

    let plan = |titre: &str| {
        json!({
            "title": titre,
            "category": "sortie",
            "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
        })
    };

    for n in 1..=3 {
        let (statut, _) = service
            .post("/v1/plans", Some(&jeton), plan(&format!("Un plan numero {n} pour tester")))
            .await;
        assert_eq!(statut, StatusCode::OK, "le plan {n} devait passer");
    }

    // Le palier le plus cher ne desserre pas l'invariant : c'est tout l'objet
    // de la règle.
    let (statut, corps) = service
        .post("/v1/plans", Some(&jeton), plan("Le plan de trop pour cet essai"))
        .await;
    assert_eq!(statut, StatusCode::CONFLICT);
    assert_eq!(corps["error"], "too_many_plans");
}

/// Le plafond de trois ne doit pas céder à deux publications simultanées.
///
/// Les plans ouverts étaient comptés hors transaction, puis le plan inséré
/// séparément. À deux plans ouverts, deux publications concurrentes lisaient
/// toutes deux « 2 », passaient toutes deux, et le compte finissait à quatre —
/// l'invariant central du module, contourné sans rien payer.
///
/// Le test part de deux plans plutôt que de trois : il reste alors exactement
/// une place, donc une seule des deux publications a le droit d'aboutir.
#[tokio::test]
async fn deux_publications_concurrentes_ne_depassent_pas_le_plafond() {
    let service = Service::monter().await;
    service.compte("c_plafond", "depart").await;
    let jeton = service.jeton("c_plafond");

    let plan = |titre: &str| {
        json!({
            "title": titre,
            "category": "sortie",
            "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
        })
    };

    for n in 1..=2 {
        let (statut, corps) = service
            .post("/v1/plans", Some(&jeton), plan(&format!("Un plan deja la numero {n}")))
            .await;
        assert_eq!(statut, StatusCode::OK, "{corps}");
    }

    let (a, b) = tokio::join!(
        service.post("/v1/plans", Some(&jeton), plan("La troisieme place a prendre")),
        service.post("/v1/plans", Some(&jeton), plan("La quatrieme qui doit tomber")),
    );

    let passees = [a.0, b.0].iter().filter(|s| s.is_success()).count();
    assert_eq!(
        passees, 1,
        "une seule place restait — obtenu {passees} (réponses : {a:?} / {b:?})"
    );

    // Et le plafond doit se vérifier dans la base, pas seulement dans les
    // codes de réponse : c'est le nombre de lignes qui compte.
    let (_, mine) = service.get("/v1/plans/mine", Some(&jeton)).await;
    let ouverts = mine
        .as_array()
        .expect("une liste de plans")
        .iter()
        .filter(|p| p["state"] == "ouvert")
        .count();
    assert_eq!(ouverts, 3, "trois plans ouverts, pas un de plus — {mine}");
}

#[tokio::test]
async fn un_plan_se_publie_a_l_avance_et_avec_un_vrai_titre() {
    let service = Service::monter().await;
    service.compte("c_valid", "depart").await;
    let jeton = &service.jeton("c_valid");

    let dans_deux_jours = (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339();

    // Trop court : un titre de trois lettres ne dit pas ce qu'on va faire.
    let (statut, _) = service
        .post("/v1/plans", Some(&jeton), json!({ "title": "Bof", "category": "sortie", "startsAt": dans_deux_jours }))
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);

    // Dans dix minutes : personne n'aurait le temps de le voir.
    let (statut, _) = service
        .post("/v1/plans", Some(&jeton), json!({
            "title": "Un titre parfaitement valable",
            "category": "sortie",
            "startsAt": (chrono::Utc::now() + chrono::Duration::minutes(10)).to_rfc3339(),
        }))
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);

    // Catégorie inventée.
    let (statut, _) = service
        .post("/v1/plans", Some(&jeton), json!({
            "title": "Un titre parfaitement valable",
            "category": "teleportation",
            "startsAt": dans_deux_jours,
        }))
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn une_demande_exige_un_message_ecrit() {
    let service = Service::monter().await;
    service.compte("c_hote", "depart").await;
    service.compte("c_invite", "depart").await;

    let (statut, plan) = service
        .post("/v1/plans", Some(&service.jeton("c_hote")), json!({
            "title": "Un plan a rejoindre pour cet essai",
            "category": "balade",
            "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
        }))
        .await;
    assert_eq!(statut, StatusCode::OK);
    let plan_id = plan["id"].as_str().unwrap();

    // « Salut » n'est pas une demande : c'est un geste.
    let (statut, _) = service
        .post("/v1/requests", Some(&service.jeton("c_invite")), json!({ "planId": plan_id, "message": "Salut" }))
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);

    let (statut, corps) = service
        .post("/v1/requests", Some(&service.jeton("c_invite")), json!({
            "planId": plan_id,
            "message": "Cette balade me tente beaucoup, je serais ravi de venir.",
        }))
        .await;
    assert_eq!(statut, StatusCode::OK);
    // Le quota du palier de départ est de cinq, une vient d'être dépensée.
    assert_eq!(corps["requestsLeftToday"], 4);

    // On ne redemande pas deux fois.
    let (statut, corps) = service
        .post("/v1/requests", Some(&service.jeton("c_invite")), json!({
            "planId": plan_id,
            "message": "Je retente ma chance avec un message different.",
        }))
        .await;
    assert_eq!(statut, StatusCode::CONFLICT);
    assert_eq!(corps["error"], "already_requested");
}

#[tokio::test]
async fn on_ne_demande_pas_a_venir_a_son_propre_plan() {
    let service = Service::monter().await;
    service.compte("c_solo", "depart").await;
    let jeton = &service.jeton("c_solo");

    let (_, plan) = service
        .post("/v1/plans", Some(&jeton), json!({
            "title": "Mon propre plan pour cet essai",
            "category": "repas",
            "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
        }))
        .await;

    let (statut, _) = service
        .post("/v1/requests", Some(&jeton), json!({
            "planId": plan["id"].as_str().unwrap(),
            "message": "Je voudrais venir a mon propre plan, ce qui na pas de sens.",
        }))
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn le_fil_ecarte_ses_propres_plans() {
    let service = Service::monter().await;
    service.compte("c_fil", "depart").await;
    service.compte("c_autre", "depart").await;

    let demain = (chrono::Utc::now() + chrono::Duration::days(1)).to_rfc3339();
    service.post("/v1/plans", Some(&service.jeton("c_fil")), json!({
        "title": "Le plan de celui qui regarde", "category": "sortie", "startsAt": demain,
    })).await;
    service.post("/v1/plans", Some(&service.jeton("c_autre")), json!({
        "title": "Le plan de quelquun dautre", "category": "sortie", "startsAt": demain,
    })).await;

    let (statut, corps) = service.get("/v1/plans", Some(&service.jeton("c_fil"))).await;
    assert_eq!(statut, StatusCode::OK);

    let plans = corps["plans"].as_array().unwrap();
    assert_eq!(plans.len(), 1, "le fil devait ne garder que le plan d'autrui");
    assert_eq!(plans[0]["title"], "Le plan de quelquun dautre");
}

#[tokio::test]
async fn le_catalogue_ne_vend_aucune_visibilite() {
    let service = Service::monter().await;
    let (statut, corps) = service.get("/v1/billing/tiers", None).await;

    assert_eq!(statut, StatusCode::OK);
    let tiers = corps["tiers"].as_array().unwrap();
    assert_eq!(tiers.len(), 5);

    for offre in tiers {
        let droits = &offre["entitlements"];
        // Aucun palier ne porte de droit de mise en avant. Si l'un venait à en
        // gagner un, ce test doit être revu en même temps que la règle.
        assert!(droits.get("boost").is_none(), "{} porte un droit de mise en avant", offre["tier"]);
        assert!(droits["requestsPerDay"].as_i64().unwrap() >= 5);
    }
}

#[tokio::test]
async fn une_url_de_media_ne_se_deflouted_pas_en_la_modifiant() {
    let service = Service::monter().await;
    service.compte("c_media", "depart").await;
    service
        .db
        .execute_unprepared(&format!(
            "UPDATE profiles SET photoKey='photos/secrete.jpg' WHERE accountId='{}'",
            service.id("c_media")
        ))
        .await
        .unwrap();

    let (_, fiche) = service.get("/v1/me", Some(&service.jeton("c_media"))).await;
    let url = fiche["photoUrl"].as_str().expect("une photo signée");
    let chemin = url.strip_prefix("https://exemple.test").unwrap();

    let (statut, _) = service.get(chemin, None).await;
    assert_eq!(statut, StatusCode::OK, "l'URL d'origine doit passer");

    // Le flou fait partie de la charge signée.
    let deflouté = chemin.replace("blur=0", "blur=8");
    let (statut, _) = service.get(&deflouté, None).await;
    assert_eq!(statut, StatusCode::FORBIDDEN, "changer le flou doit invalider");
}

/// Une pause est un aller-retour : les plans reviennent à la reprise.
///
/// La mise en pause faisait passer les plans ouverts à « suspendu » — et rien,
/// nulle part, ne les en sortait. Mettre son compte en pause revenait donc à
/// perdre ses plans pour de bon, alors que la page publique promet de
/// reprendre quand on veut, « sans rien supprimer ».
#[tokio::test]
async fn reprendre_apres_une_pause_rend_ses_plans() {
    let service = Service::monter().await;
    service.compte("c_pause_plans", "depart").await;
    let jeton = service.jeton("c_pause_plans");

    let (statut, corps) = service
        .post("/v1/plans", Some(&jeton), json!({
            "title": "Un plan qui doit survivre a la pause",
            "category": "balade",
            "startsAt": (chrono::Utc::now() + chrono::Duration::days(3)).to_rfc3339(),
        }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    async fn etats(service: &Service, jeton: &str) -> Vec<String> {
        let (_, mine) = service.get("/v1/plans/mine", Some(jeton)).await;
        mine.as_array()
            .expect("une liste")
            .iter()
            .map(|p| p["state"].as_str().unwrap_or("?").to_string())
            .collect()
    }

    let (statut, corps) = service
        .post("/v1/me/pause", Some(&jeton), json!({ "paused": true }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(
        etats(&service, &jeton).await,
        vec!["suspendu"],
        "la pause devait retirer le plan du fil"
    );

    let (statut, corps) = service
        .post("/v1/me/pause", Some(&jeton), json!({ "paused": false }))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(
        etats(&service, &jeton).await,
        vec!["ouvert"],
        "le plan n'est pas revenu : la pause aura coûté un plan"
    );
}
