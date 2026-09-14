//! Ce que le fil retient, et ce qu'il écarte.
//!
//! Le fil est le produit : c'est la seule page où quelqu'un décide d'écrire à
//! quelqu'un d'autre. Ses filtres étaient pourtant les moins gardés du dépôt —
//! l'âge, la distance, les plans complets et les comptes en pause pouvaient
//! tous être retirés sans qu'un seul des trois cent onze tests ne bronche.
//!
//! Chaque test ci-dessous a été éprouvé en retirant le filtre qu'il garde.

use super::Service;
use axum::http::StatusCode;
use sea_orm::ConnectionTrait;
use serde_json::{Value, json};

/// Hors des bornes d'âge, un plan ne paraît pas.
#[tokio::test]
async fn le_fil_ecarte_les_auteurs_hors_des_bornes_d_age() {
    let service = Service::monter().await;
    let lecteur = service.compte("c_lecteur_age", "depart").await;
    let jeune = service.compte("c_jeune", "depart").await;
    let age = service.compte("c_age", "depart").await;
    let dans_les_bornes = service.compte("c_pile", "depart").await;

    // Le lecteur cherche entre 30 et 40 ans.
    service
        .db
        .execute_unprepared(&format!(
            "UPDATE preferences SET minAge=30, maxAge=40 WHERE accountId='{lecteur}'"
        ))
        .await
        .unwrap();

    // Vingt ans, soixante ans, et trente-cinq ans.
    for (compte, naissance) in [
        (&jeune, "2006-01-15"),
        (&age, "1966-01-15"),
        (&dans_les_bornes, "1991-01-15"),
    ] {
        service
            .db
            .execute_unprepared(&format!(
                "UPDATE accounts SET birthDate='{naissance} 00:00:00' WHERE id='{compte}'"
            ))
            .await
            .unwrap();
    }

    let trop_jeune = plan_de(&service, "c_jeune", "Un verre après les cours").await;
    let trop_age = plan_de(&service, "c_age", "Une partie de belote").await;
    let retenu = plan_de(&service, "c_pile", "Un concert au petit théâtre").await;

    let fil = titres_du_fil(&service, "c_lecteur_age").await;
    assert!(
        fil.contains(&retenu),
        "le plan dans les bornes manque : {fil:?}"
    );
    assert!(
        !fil.contains(&trop_jeune),
        "un auteur de vingt ans passe des bornes 30–40 : {fil:?}"
    );
    assert!(
        !fil.contains(&trop_age),
        "un auteur de soixante ans passe des bornes 30–40 : {fil:?}"
    );
}

/// Au-delà du rayon, un plan ne paraît pas — y compris juste au-delà.
///
/// Deux gardes se suivent : la requête borne d'abord une BOÎTE autour du
/// lecteur, puis la distance réelle est calculée en mémoire. Un plan lointain
/// ne prouve donc rien — la boîte suffit à l'écarter, et le second contrôle
/// pourrait disparaître sans qu'on le voie. C'est arrivé : retirer la ligne
/// laissait toute la suite au vert.
///
/// Le troisième compte est placé dans le COIN de la boîte : à l'intérieur de
/// ses bornes, mais à plus de soixante kilomètres à vol d'oiseau, pour un
/// rayon de cinquante. Lui seul éprouve le calcul de distance.
#[tokio::test]
async fn le_fil_ecarte_ce_qui_est_trop_loin() {
    let service = Service::monter().await;
    let lecteur = service.compte("c_lecteur_loin", "depart").await;
    service.compte("c_proche", "depart").await;
    let lointain = service.compte("c_lointain", "depart").await;
    let coin = service.compte("c_coin", "depart").await;

    // Cinquante kilomètres. Les comptes de test naissent à Lyon (45.75, 4.85).
    service
        .db
        .execute_unprepared(&format!(
            "UPDATE preferences SET maxDistanceKm=50 WHERE accountId='{lecteur}'"
        ))
        .await
        .unwrap();
    // Paris : hors de la boîte, écarté par la requête.
    service
        .db
        .execute_unprepared(&format!(
            "UPDATE profiles SET latRounded=48.86, lonRounded=2.35 WHERE accountId='{lointain}'"
        ))
        .await
        .unwrap();
    // Le coin de la boîte : dedans, mais à 61 km.
    service
        .db
        .execute_unprepared(&format!(
            "UPDATE profiles SET latRounded=46.15, lonRounded=5.40 WHERE accountId='{coin}'"
        ))
        .await
        .unwrap();

    let ici = plan_de(&service, "c_proche", "Un café sur les pentes").await;
    let la_bas = plan_de(&service, "c_lointain", "Une expo au Grand Palais").await;
    let au_coin = plan_de(&service, "c_coin", "Une rando dans le Revermont").await;

    let fil = titres_du_fil(&service, "c_lecteur_loin").await;
    assert!(fil.contains(&ici), "le plan proche manque : {fil:?}");
    assert!(
        !fil.contains(&la_bas),
        "un plan à trois cents kilomètres passe un rayon de cinquante : {fil:?}"
    );
    assert!(
        !fil.contains(&au_coin),
        "un plan à soixante et un kilomètres passe un rayon de cinquante : \
         seule la boîte l'aurait retenu, et elle ne suffit pas ({fil:?})"
    );
}

/// Un plan sans place sort du fil, pour tout le monde.
///
/// Y compris pour qui y a demandé sa place : le code portait une exemption
/// contraire, mais elle était inatteignable — la requête du fil ne retient que
/// les plans « ouvert », et un plan passe à « complet » dès sa dernière place
/// prise. Ce test dit donc ce que le produit fait aujourd'hui. L'élargir à
/// « complet » pour qui a déjà demandé reste possible ; c'est une décision,
/// pas une correction.
#[tokio::test]
async fn un_plan_complet_sort_du_fil_pour_tout_le_monde() {
    let service = Service::monter().await;
    service.compte("c_hote_complet", "depart").await;
    service.compte("c_demandeur_complet", "depart").await;
    service.compte("c_temoin_complet", "depart").await;

    let titre = plan_de(&service, "c_hote_complet", "Un dîner pour deux").await;
    let plan = dernier_plan(&service, "c_hote_complet").await;

    // Avant : les deux le voient.
    assert!(
        titres_du_fil(&service, "c_demandeur_complet")
            .await
            .contains(&titre)
    );
    assert!(
        titres_du_fil(&service, "c_temoin_complet")
            .await
            .contains(&titre)
    );

    let demande = demander(&service, "c_demandeur_complet", &plan).await;
    accepter(&service, "c_hote_complet", &demande).await;

    assert!(
        !titres_du_fil(&service, "c_temoin_complet")
            .await
            .contains(&titre),
        "un plan complet paraît encore à qui n'a rien demandé"
    );
    assert!(
        !titres_du_fil(&service, "c_demandeur_complet")
            .await
            .contains(&titre),
        "le fil retient un plan complet : l'exemption est-elle redevenue \
         atteignable ? Si c'est voulu, c'est ce test qu'il faut changer."
    );
}

/// Les plans d'un compte en pause ou en suppression sortent du fil.
#[tokio::test]
async fn le_fil_ecarte_les_comptes_en_pause_et_en_suppression() {
    let service = Service::monter().await;
    service.compte("c_lecteur_statut", "depart").await;
    let en_pause = service.compte("c_en_pause", "depart").await;
    let partant = service.compte("c_partant", "depart").await;
    service.compte("c_actif", "depart").await;

    let pause = plan_de(&service, "c_en_pause", "Un tour au marché").await;
    let depart = plan_de(&service, "c_partant", "Une lecture à voix haute").await;
    let actif = plan_de(&service, "c_actif", "Un tournoi de tarot").await;

    // Tant que tout le monde est actif, les trois paraissent.
    let avant = titres_du_fil(&service, "c_lecteur_statut").await;
    for titre in [&pause, &depart, &actif] {
        assert!(
            avant.contains(titre),
            "{titre} manque au départ : {avant:?}"
        );
    }

    service
        .db
        .execute_unprepared(&format!(
            "UPDATE accounts SET status='paused' WHERE id='{en_pause}'"
        ))
        .await
        .unwrap();
    service
        .db
        .execute_unprepared(&format!(
            "UPDATE accounts SET deletionRequestedAt='2026-09-12 10:00:00' WHERE id='{partant}'"
        ))
        .await
        .unwrap();

    let apres = titres_du_fil(&service, "c_lecteur_statut").await;
    assert!(
        apres.contains(&actif),
        "le plan actif a disparu : {apres:?}"
    );
    assert!(
        !apres.contains(&pause),
        "le plan d'un compte en pause paraît encore : {apres:?}"
    );
    assert!(
        !apres.contains(&depart),
        "le plan d'un compte en suppression paraît encore : {apres:?}"
    );
}

/// Le fil dit si l'on a déjà demandé sa place.
///
/// L'application s'en sert pour proposer « Demander » ou afficher « Demande
/// envoyée ». Figé à faux, elle proposerait de demander une place déjà
/// demandée, et le serveur répondrait « on ne redemande pas deux fois ».
/// Rien ne gardait ce champ.
#[tokio::test]
async fn le_fil_dit_si_l_on_a_deja_demande() {
    let service = Service::monter().await;
    service.compte("c_hote_requested", "depart").await;
    service.compte("c_demandeur_requested", "depart").await;

    // Une place suffit : la demande reste en attente, donc le plan garde la
    // sienne et ne quitte pas le fil.
    let (statut, corps) = service
        .post(
            "/v1/plans",
            Some(&service.jeton("c_hote_requested")),
            json!({
                "title": "Une soirée jeux entre inconnus",
                "category": "jeux",
                "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    let plan = corps["id"].as_str().expect("identifiant").to_string();

    assert_eq!(
        requested(&service, "c_demandeur_requested", &plan).await,
        Some(false),
        "le fil annonce une demande qui n'a pas eu lieu"
    );

    demander(&service, "c_demandeur_requested", &plan).await;

    assert_eq!(
        requested(&service, "c_demandeur_requested", &plan).await,
        Some(true),
        "le fil ne dit pas que la place a déjà été demandée"
    );
}

/// `requested` pour un plan donné, tel que le fil le rend.
async fn requested(service: &Service, nom: &str, plan: &str) -> Option<bool> {
    crate::cache::oublier(
        &service.etat.cache,
        &crate::cache::cles::fil(&service.id(nom)),
    )
    .await
    .ok();
    let (_, corps): (StatusCode, Value) = service.get("/v1/plans", Some(&service.jeton(nom))).await;
    corps["plans"]
        .as_array()?
        .iter()
        .find(|p| p["id"] == plan)?["requested"]
        .as_bool()
}

// --- outils ------------------------------------------------------------

async fn plan_de(service: &Service, nom: &str, titre: &str) -> String {
    let (statut, corps) = service
        .post(
            "/v1/plans",
            Some(&service.jeton(nom)),
            json!({
                "title": titre,
                "category": "sortie",
                "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    titre.to_string()
}

async fn dernier_plan(service: &Service, nom: &str) -> String {
    let (_, corps) = service
        .get("/v1/plans/mine", Some(&service.jeton(nom)))
        .await;
    let plans = corps
        .as_array()
        .cloned()
        .or_else(|| corps["plans"].as_array().cloned())
        .expect("une liste de plans");
    plans
        .last()
        .and_then(|p| p["id"].as_str())
        .expect("un identifiant")
        .to_string()
}

async fn demander(service: &Service, nom: &str, plan: &str) -> String {
    let (statut, corps) = service
        .post(
            "/v1/requests",
            Some(&service.jeton(nom)),
            json!({ "planId": plan, "message": "Ce plan me tente beaucoup, je viendrais volontiers." }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    corps["id"].as_str().expect("identifiant").to_string()
}

async fn accepter(service: &Service, nom: &str, demande: &str) {
    let (statut, corps) = service
        .post(
            &format!("/v1/requests/{demande}/accept"),
            Some(&service.jeton(nom)),
            json!({}),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
}

/// Les titres du fil, relu sans cache : c'est la composition qu'on éprouve,
/// pas la mémoire.
async fn titres_du_fil(service: &Service, nom: &str) -> Vec<String> {
    crate::cache::oublier(
        &service.etat.cache,
        &crate::cache::cles::fil(&service.id(nom)),
    )
    .await
    .ok();
    let (statut, corps): (StatusCode, Value) =
        service.get("/v1/plans", Some(&service.jeton(nom))).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    corps["plans"]
        .as_array()
        .expect("une liste")
        .iter()
        .filter_map(|p| p["title"].as_str().map(str::to_string))
        .collect()
}

/// On ne publie pas plus de trois plans ouverts à la fois.
///
/// La borne existe pour que le fil reste tenable : sans elle, un seul compte
/// peut le remplir. Rien ne la gardait — la retirer laissait toute la suite au
/// vert.
#[tokio::test]
async fn le_nombre_de_plans_ouverts_est_borne() {
    let service = Service::monter().await;
    service.compte("c_prolifique", "depart").await;
    let jeton = service.jeton("c_prolifique");
    let maximum = crate::routes::plans::MAX_PLANS_OUVERTS;

    for numero in 0..maximum {
        let (statut, corps) = service
            .post(
                "/v1/plans",
                Some(&jeton),
                json!({
                    "title": format!("Un plan parmi d'autres, le numero {numero}"),
                    "category": "sortie",
                    "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
                }),
            )
            .await;
        assert_eq!(
            statut,
            StatusCode::OK,
            "publication {numero} refusée : {corps}"
        );
    }

    let (statut, corps) = service
        .post(
            "/v1/plans",
            Some(&jeton),
            json!({
                "title": "Celui de trop, qui ne doit pas passer",
                "category": "sortie",
                "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
            }),
        )
        .await;
    assert_ne!(
        statut,
        StatusCode::OK,
        "un {}e plan ouvert est passé : le fil n'a plus de borne ({corps})",
        maximum + 1
    );
}
