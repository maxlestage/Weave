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
        assert_eq!(
            statut,
            StatusCode::UNAUTHORIZED,
            "{chemin} avec un faux jeton"
        );
    }
}

#[tokio::test]
async fn un_jeton_valable_pour_un_compte_inexistant_est_refuse() {
    // Un compte supprimé laisse des jetons valides en circulation : ils ne
    // doivent plus ouvrir la porte.
    let service = Service::monter().await;
    let (statut, _) = service
        .get("/v1/me", Some(&service.jeton("compte_fantome")))
        .await;
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
            .post(
                "/v1/plans",
                Some(&jeton),
                plan(&format!("Un plan numero {n} pour tester")),
            )
            .await;
        assert_eq!(statut, StatusCode::OK, "le plan {n} devait passer");
    }

    // Le palier le plus cher ne desserre pas l'invariant : c'est tout l'objet
    // de la règle.
    let (statut, corps) = service
        .post(
            "/v1/plans",
            Some(&jeton),
            plan("Le plan de trop pour cet essai"),
        )
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
            .post(
                "/v1/plans",
                Some(&jeton),
                plan(&format!("Un plan deja la numero {n}")),
            )
            .await;
        assert_eq!(statut, StatusCode::OK, "{corps}");
    }

    let (a, b) = tokio::join!(
        service.post(
            "/v1/plans",
            Some(&jeton),
            plan("La troisieme place a prendre")
        ),
        service.post(
            "/v1/plans",
            Some(&jeton),
            plan("La quatrieme qui doit tomber")
        ),
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
        .post(
            "/v1/plans",
            Some(&jeton),
            json!({ "title": "Bof", "category": "sortie", "startsAt": dans_deux_jours }),
        )
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);

    // Dans dix minutes : personne n'aurait le temps de le voir.
    let (statut, _) = service
        .post(
            "/v1/plans",
            Some(&jeton),
            json!({
                "title": "Un titre parfaitement valable",
                "category": "sortie",
                "startsAt": (chrono::Utc::now() + chrono::Duration::minutes(10)).to_rfc3339(),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);

    // Catégorie inventée.
    let (statut, _) = service
        .post(
            "/v1/plans",
            Some(&jeton),
            json!({
                "title": "Un titre parfaitement valable",
                "category": "teleportation",
                "startsAt": dans_deux_jours,
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn une_demande_exige_un_message_ecrit() {
    let service = Service::monter().await;
    service.compte("c_hote", "depart").await;
    service.compte("c_invite", "depart").await;

    let (statut, plan) = service
        .post(
            "/v1/plans",
            Some(&service.jeton("c_hote")),
            json!({
                "title": "Un plan a rejoindre pour cet essai",
                "category": "balade",
                "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);
    let plan_id = plan["id"].as_str().unwrap();

    // « Salut » n'est pas une demande : c'est un geste.
    let (statut, _) = service
        .post(
            "/v1/requests",
            Some(&service.jeton("c_invite")),
            json!({ "planId": plan_id, "message": "Salut" }),
        )
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);

    let (statut, corps) = service
        .post(
            "/v1/requests",
            Some(&service.jeton("c_invite")),
            json!({
                "planId": plan_id,
                "message": "Cette balade me tente beaucoup, je serais ravi de venir.",
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);
    // Le quota du palier de départ est de cinq, une vient d'être dépensée.
    assert_eq!(corps["requestsLeftToday"], 4);

    // On ne redemande pas deux fois.
    let (statut, corps) = service
        .post(
            "/v1/requests",
            Some(&service.jeton("c_invite")),
            json!({
                "planId": plan_id,
                "message": "Je retente ma chance avec un message different.",
            }),
        )
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
        .post(
            "/v1/plans",
            Some(&jeton),
            json!({
                "title": "Mon propre plan pour cet essai",
                "category": "repas",
                "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
            }),
        )
        .await;

    let (statut, _) = service
        .post(
            "/v1/requests",
            Some(&jeton),
            json!({
                "planId": plan["id"].as_str().unwrap(),
                "message": "Je voudrais venir a mon propre plan, ce qui na pas de sens.",
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn le_fil_ecarte_ses_propres_plans() {
    let service = Service::monter().await;
    service.compte("c_fil", "depart").await;
    service.compte("c_autre", "depart").await;

    let demain = (chrono::Utc::now() + chrono::Duration::days(1)).to_rfc3339();
    service
        .post(
            "/v1/plans",
            Some(&service.jeton("c_fil")),
            json!({
                "title": "Le plan de celui qui regarde", "category": "sortie", "startsAt": demain,
            }),
        )
        .await;
    service
        .post(
            "/v1/plans",
            Some(&service.jeton("c_autre")),
            json!({
                "title": "Le plan de quelquun dautre", "category": "sortie", "startsAt": demain,
            }),
        )
        .await;

    let (statut, corps) = service
        .get("/v1/plans", Some(&service.jeton("c_fil")))
        .await;
    assert_eq!(statut, StatusCode::OK);

    let plans = corps["plans"].as_array().unwrap();
    assert_eq!(
        plans.len(),
        1,
        "le fil devait ne garder que le plan d'autrui"
    );
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
        assert!(
            droits.get("boost").is_none(),
            "{} porte un droit de mise en avant",
            offre["tier"]
        );
        assert!(droits["requestsPerDay"].as_i64().unwrap() >= 5);
    }
}

#[tokio::test]
async fn une_url_de_media_ne_se_deflouted_pas_en_la_modifiant() {
    use crate::entities::media_objects;
    use sea_orm::{ActiveModelTrait, Set};

    let service = Service::monter().await;
    let compte = service.compte("c_media", "depart").await;

    // Une vraie image en base : le service rend désormais des octets, plus un
    // objet JSON expliquant que le stockage « est branché au déploiement ».
    let octets = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x01, 0x02, 0x03];
    media_objects::ActiveModel {
        key: Set("photo-secrete".to_string()),
        account_id: Set(compte.clone()),
        content_type: Set("image/jpeg".to_string()),
        byte_size: Set(octets.len() as i32),
        bytes: Set(octets.clone()),
        created_at: Set(chrono::Utc::now().naive_utc()),
    }
    .insert(&service.db)
    .await
    .expect("média inséré");

    service
        .db
        .execute_unprepared(&format!(
            "UPDATE profiles SET photoKey='photo-secrete' WHERE accountId='{compte}'"
        ))
        .await
        .unwrap();

    let (_, fiche) = service.get("/v1/me", Some(&service.jeton("c_media"))).await;
    let url = fiche["photoUrl"].as_str().expect("une photo signée");
    let chemin = url.strip_prefix("https://exemple.test").unwrap();

    let (statut, entetes, corps) = service.get_brut(chemin).await;
    assert_eq!(
        statut,
        StatusCode::OK,
        "l'URL d'origine doit passer : {corps}"
    );
    assert_eq!(
        entetes
            .get(axum::http::header::CONTENT_TYPE)
            .map(|v| v.to_str().unwrap()),
        Some("image/jpeg"),
        "le type de contenu doit être celui déduit des octets"
    );

    // Le flou fait partie de la charge signée.
    let deflouté = chemin.replace("blur=0", "blur=8");
    let (statut, _, _) = service.get_brut(&deflouté).await;
    assert_eq!(
        statut,
        StatusCode::FORBIDDEN,
        "changer le flou doit invalider"
    );
}

/// Un flou correctement signé est refusé, jamais servi net.
///
/// Le niveau de flou est inscrit dans la signature pour qu'il ne puisse pas
/// être changé côté client. Mais il n'est pas encore appliqué : servir net une
/// photo dont la signature réclamait un flou ouvrirait exactement ce que la
/// signature protège. Le jour où la révélation progressive s'écrira, elle
/// butera ici plutôt que de découvrir quelqu'un sans le vouloir.
#[tokio::test]
async fn un_flou_demande_mais_non_applique_est_refuse() {
    use crate::crypto::signer_url_media;

    let service = Service::monter().await;
    // La configuration du service fait foi : recopier l'adresse et le secret
    // ici les ferait diverger au premier changement, et le test signerait
    // pour un service qui n'est plus celui qu'on éprouve.
    let media = &service.etat.config.media;
    let url = signer_url_media(
        &media.base_url,
        &media.signing_secret,
        "photo-quelconque",
        600,
        8,
    );
    let chemin = url.strip_prefix("https://exemple.test").unwrap();

    let (statut, _, corps) = service.get_brut(chemin).await;
    assert_eq!(
        statut,
        StatusCode::FORBIDDEN,
        "une photo a été servie nette alors qu'un flou était signé : {corps}"
    );
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
        .post(
            "/v1/plans",
            Some(&jeton),
            json!({
                "title": "Un plan qui doit survivre a la pause",
                "category": "balade",
                "startsAt": (chrono::Utc::now() + chrono::Duration::days(3)).to_rfc3339(),
            }),
        )
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

/// Un signalement de minorité suspend le compte tout de suite.
///
/// La politique de confidentialité s'y engage publiquement — « entraîne la
/// suspension immédiate du compte ». Elle le disait sans que rien ne le fasse :
/// le signalement n'écrivait qu'une ligne, et le compte continuait de publier
/// et d'écrire en attendant qu'une personne ouvre le dossier.
#[tokio::test]
async fn un_signalement_de_minorite_suspend_le_compte_signale() {
    let service = Service::monter().await;
    let vigilant = service.compte("c_vigilant", "depart").await;
    let signale = service.compte("c_signale_mineur", "depart").await;

    let (statut, corps) = service
        .post(
            "/v1/reports",
            Some(&service.jeton("c_vigilant")),
            json!({
                "accountId": signale,
                "reason": "mineur",
                "details": "Le profil indique etre au lycee en seconde.",
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    let _ = vigilant;

    // Le compte n'ouvre plus rien : c'est ce que « suspendu » doit vouloir dire.
    let (statut, corps) = service
        .get("/v1/me", Some(&service.jeton("c_signale_mineur")))
        .await;
    assert_eq!(
        statut,
        StatusCode::UNAUTHORIZED,
        "le compte signalé comme mineur répond encore : {corps}"
    );
}

/// Les autres motifs, eux, ne suspendent personne.
///
/// Un signalement n'est pas une preuve. Suspendre sur n'importe quel motif
/// ferait de la fonction une arme : il suffirait de signaler pour faire taire.
/// La minorité est la seule exception, parce que le délai y coûte plus cher
/// que l'erreur.
#[tokio::test]
async fn un_signalement_ordinaire_ne_suspend_pas() {
    let service = Service::monter().await;
    service.compte("c_plaignant", "depart").await;
    let vise = service.compte("c_vise_ordinaire", "depart").await;

    let (statut, corps) = service
        .post(
            "/v1/reports",
            Some(&service.jeton("c_plaignant")),
            json!({
                "accountId": vise,
                "reason": "arnaque",
                "details": "Demande de l argent des le premier message.",
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let (statut, corps) = service
        .get("/v1/me", Some(&service.jeton("c_vise_ordinaire")))
        .await;
    assert_eq!(
        statut,
        StatusCode::OK,
        "un signalement ordinaire a suffi à couper un compte : {corps}"
    );
}

/// Une inscription complète doit mener à un compte qui fonctionne.
///
/// L'inscription ne demandait que le prénom et la date de naissance. Le compte
/// restait donc « onboarding », sans fiche — et sans fiche, le fil est vide et
/// la publication refusée. Tout le monde arrivait sur une application morte,
/// sans rien pour le dire.
///
/// Le test déroule le parcours réel : code, inscription, dépôt de la fiche.
/// C'est cette dernière étape que l'application ne faisait pas.
#[tokio::test]
async fn deposer_sa_fiche_ouvre_le_compte() {
    let service = Service::monter().await;
    let email = service.email("c_nouvelle");

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
    let acces = corps["session"]["accessToken"]
        .as_str()
        .expect("un accès")
        .to_string();

    // Sans fiche : le compte est né, et il ne sert à rien.
    let (statut, moi) = service.get("/v1/me", Some(&acces)).await;
    assert_eq!(statut, StatusCode::OK, "{moi}");
    assert_eq!(
        moi["status"], "onboarding",
        "le compte devrait attendre sa fiche"
    );

    let plan = json!({
        "title": "Un cafe pour faire connaissance",
        "category": "repas",
        "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
    });
    let (statut, corps) = service.post("/v1/plans", Some(&acces), plan.clone()).await;
    assert_ne!(
        statut,
        StatusCode::OK,
        "publier sans fiche devrait être refusé : {corps}"
    );

    // La fiche déposée : c'est l'appel que l'application ne faisait nulle part.
    let (statut, corps) = service
        .put(
            "/v1/me/profile",
            Some(&acces),
            json!({
                "city": "Nantes",
                "latitude": 47.2184,
                "longitude": -1.5536,
                "gender": "femme",
                "bio": "Je connais tous les bars a chats de la ville.",
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let (_, moi) = service.get("/v1/me", Some(&acces)).await;
    assert_eq!(moi["status"], "active", "la fiche devait ouvrir le compte");
    assert_eq!(moi["city"], "Nantes");

    let (statut, corps) = service.post("/v1/plans", Some(&acces), plan).await;
    assert_eq!(statut, StatusCode::OK, "publier après la fiche : {corps}");
}

/// Un genre hors vocabulaire est refusé, des deux côtés.
///
/// Le fil compare le genre de l'auteur d'un plan à ceux que le lecteur
/// cherche, caractère par caractère. Une valeur libre ne rendait pas une
/// erreur : elle rendait un fil vide, sans rien dire à personne.
#[tokio::test]
async fn un_genre_hors_vocabulaire_est_refuse() {
    let service = Service::monter().await;
    service.compte("c_genre", "escapade").await;
    let jeton = service.jeton("c_genre");

    let fiche = |genre: &str| {
        json!({
            "city": "Lyon",
            "latitude": 45.76,
            "longitude": 4.84,
            "gender": genre,
        })
    };

    let (statut, corps) = service
        .put("/v1/me/profile", Some(&jeton), fiche("Femme"))
        .await;
    refuse(
        statut,
        &corps,
        "validation",
        &format!("« Femme » majuscule devrait être refusé : {corps}"),
    );

    let (statut, corps) = service
        .put("/v1/me/profile", Some(&jeton), fiche("femme"))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    // Et les critères de recherche suivent le même vocabulaire.
    service.consentir(&jeton).await;
    let (statut, corps) = service
        .patch(
            "/v1/me/preferences",
            Some(&jeton),
            json!({ "seeking": ["Homme"] }),
        )
        .await;
    refuse(
        statut,
        &corps,
        "validation",
        &format!("un genre cherché hors liste : {corps}"),
    );

    let (statut, corps) = service
        .patch(
            "/v1/me/preferences",
            Some(&jeton),
            json!({ "seeking": ["homme", "non_binaire"] }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
}

/// Un téléphone qui change de mains ne notifie plus son ancien propriétaire.
///
/// L'index unique porte sur (compte, appareil) : le même téléphone peut donc
/// figurer sous plusieurs comptes, et la déconnexion ne touche pas la ligne
/// d'appareil — elle ne révoque que les jetons de session. La ligne du premier
/// survivait donc avec le même jeton de poussée, puisque ce jeton appartient à
/// l'appareil et non au compte.
///
/// Les alertes du premier continuaient d'arriver sur un téléphone qui n'était
/// plus le sien : « Nouveau message », sur l'écran verrouillé d'un inconnu.
#[tokio::test]
async fn un_telephone_qui_change_de_mains_cesse_de_notifier_le_precedent() {
    use crate::entities::devices;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let service = Service::monter().await;
    let premier = service.compte("c_premier_tel", "depart").await;
    service.compte("c_second_tel", "depart").await;

    // Le même appareil, déclaré par l'un puis par l'autre.
    let appareil = format!("vendor-{premier}");
    let declaration = json!({
        "vendorId": appareil,
        "platform": "ios",
        "apnsToken": "jeton-du-telephone",
        "pushToStartToken": "jeton-de-banniere",
        "apnsEnvironment": "sandbox",
    });

    let (statut, corps) = service
        .put(
            "/v1/devices",
            Some(&service.jeton("c_premier_tel")),
            declaration.clone(),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let (statut, corps) = service
        .put(
            "/v1/devices",
            Some(&service.jeton("c_second_tel")),
            declaration,
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    // La ligne du premier existe encore — c'est voulu — mais elle ne vise plus
    // rien.
    let ancienne = devices::Entity::find()
        .filter(devices::Column::AccountId.eq(premier.as_str()))
        .filter(devices::Column::VendorId.eq(appareil.as_str()))
        .one(&service.db)
        .await
        .unwrap()
        .expect("la ligne du premier compte");
    assert!(
        ancienne.apns_token.is_none() && ancienne.push_to_start_token.is_none(),
        "l'ancien propriétaire vise encore ce téléphone : ses alertes y arriveraient"
    );

    // Et le nouveau, lui, reçoit bien.
    let nouvelle = devices::Entity::find()
        .filter(devices::Column::AccountId.eq(service.id("c_second_tel").as_str()))
        .filter(devices::Column::VendorId.eq(appareil.as_str()))
        .one(&service.db)
        .await
        .unwrap()
        .expect("la ligne du second compte");
    assert_eq!(nouvelle.apns_token.as_deref(), Some("jeton-du-telephone"));
}

/// Déposer une photo, la relire, la remplacer.
///
/// `photoKey` existait depuis le début et n'a jamais été écrit autrement qu'à
/// NULL : aucune route ne permettait d'envoyer une photo, et le service des
/// médias rendait un objet JSON expliquant que le relais vers le stockage
/// « est branché au déploiement ». Il ne l'a jamais été — personne ne pouvait
/// avoir de photo, sur une application de rencontre.
#[tokio::test]
async fn une_photo_se_depose_se_relit_et_se_remplace() {
    use crate::entities::media_objects;
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

    let service = Service::monter().await;
    let compte = service.compte("c_photo", "depart").await;
    let jeton = service.jeton("c_photo");

    // La fiche d'abord : une photo sans ville n'aurait nulle part où aller.
    let (statut, corps) = service
        .put(
            "/v1/me/profile",
            Some(&jeton),
            json!({
                "city": "Bordeaux",
                "latitude": 44.84,
                "longitude": -0.58,
                "gender": "autre",
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    // Un JPEG minimal : ce sont ses octets de tête qui le font reconnaître.
    let jpeg = |teinte: u8| {
        let mut octets = vec![0xFF, 0xD8, 0xFF, 0xE0];
        octets.extend_from_slice(&[teinte; 64]);
        octets
    };

    let (statut, corps) = service.put_octets("/v1/me/photo", &jeton, jpeg(1)).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    let url = corps["photoUrl"]
        .as_str()
        .expect("une URL signée")
        .to_string();

    // Et elle se relit, par l'URL signée, avec son type déduit des octets.
    let chemin = url.strip_prefix("https://exemple.test").unwrap();
    let (statut, entetes, _) = service.get_brut(chemin).await;
    assert_eq!(statut, StatusCode::OK);
    assert_eq!(
        entetes
            .get(axum::http::header::CONTENT_TYPE)
            .map(|v| v.to_str().unwrap()),
        Some("image/jpeg")
    );

    // La remplacer ne doit pas laisser l'ancienne derrière : une image
    // orpheline reste en base sans que rien ne la désigne plus.
    let (statut, corps) = service.put_octets("/v1/me/photo", &jeton, jpeg(2)).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let restants = media_objects::Entity::find()
        .filter(media_objects::Column::AccountId.eq(compte.as_str()))
        .count(&service.db)
        .await
        .unwrap();
    assert_eq!(restants, 1, "l'ancienne photo est restée en base");
}

/// Ce qui n'est pas une image est refusé, quel que soit l'en-tête annoncé.
#[tokio::test]
async fn un_fichier_qui_n_est_pas_une_image_est_refuse() {
    let service = Service::monter().await;
    service.compte("c_faux_media", "depart").await;
    let jeton = service.jeton("c_faux_media");

    let (statut, corps) = service
        .put_octets(
            "/v1/me/photo",
            &jeton,
            b"<?php system($_GET[0]); ?>".to_vec(),
        )
        .await;
    refuse(
        statut,
        &corps,
        "validation",
        &format!("un fichier arbitraire a été stocké : {corps}"),
    );
}
