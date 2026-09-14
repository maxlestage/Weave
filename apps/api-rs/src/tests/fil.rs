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

/// On n'annule pas le plan de quelqu'un d'autre.
///
/// Le contrôle existait ; rien ne le tenait. Le retirer laissait les trois
/// cent dix-huit tests au vert, et n'importe quel compte connecté pouvait
/// alors annuler n'importe quel plan — et, avec lui, faire expirer toutes les
/// demandes qui l'attendaient.
#[tokio::test]
async fn on_n_annule_pas_le_plan_d_un_autre() {
    let service = Service::monter().await;
    service.compte("c_auteur_plan", "depart").await;
    service.compte("c_intrus", "depart").await;

    plan_de(&service, "c_auteur_plan", "Un ciné-club le jeudi soir").await;
    let plan = dernier_plan(&service, "c_auteur_plan").await;

    let (statut, corps) = service
        .delete(
            &format!("/v1/plans/{plan}"),
            Some(&service.jeton("c_intrus")),
        )
        .await;
    assert_eq!(
        statut,
        StatusCode::FORBIDDEN,
        "un compte étranger a annulé ce plan : {corps}"
    );

    // Et le plan est toujours là, pour son auteur comme pour le fil.
    assert!(
        titres_du_fil(&service, "c_intrus")
            .await
            .contains(&"Un ciné-club le jeudi soir".to_string()),
        "le plan a disparu du fil malgré le refus"
    );

    // Son auteur, lui, peut.
    let (statut, corps) = service
        .delete(
            &format!("/v1/plans/{plan}"),
            Some(&service.jeton("c_auteur_plan")),
        )
        .await;
    assert_eq!(
        statut,
        StatusCode::OK,
        "l'auteur ne peut plus annuler : {corps}"
    );
}

/// On n'accepte pas une demande adressée à quelqu'un d'autre.
///
/// Refuser était gardé ; accepter ne l'était pas. Un compte étranger pouvait
/// donc donner une place sur le plan d'autrui — et ouvrir la conversation qui
/// va avec, entre deux personnes dont aucune ne l'a voulu.
#[tokio::test]
async fn on_n_accepte_pas_une_demande_adressee_a_un_autre() {
    let service = Service::monter().await;
    service.compte("c_hote_accept", "depart").await;
    service.compte("c_invite_accept", "depart").await;
    service.compte("c_intrus_accept", "depart").await;

    plan_de(&service, "c_hote_accept", "Un atelier de poterie").await;
    let plan = dernier_plan(&service, "c_hote_accept").await;
    let demande = demander(&service, "c_invite_accept", &plan).await;

    let (statut, corps) = service
        .post(
            &format!("/v1/requests/{demande}/accept"),
            Some(&service.jeton("c_intrus_accept")),
            json!({}),
        )
        .await;
    assert_eq!(
        statut,
        StatusCode::FORBIDDEN,
        "un compte étranger a accepté cette demande : {corps}"
    );

    // Aucune conversation n'a été ouverte.
    let (_, conversations) = service
        .get("/v1/conversations", Some(&service.jeton("c_invite_accept")))
        .await;
    let vide = conversations
        .as_array()
        .map(|c| c.is_empty())
        .unwrap_or(true);
    assert!(vide, "une conversation a été ouverte : {conversations}");

    // L'hôte, lui, peut.
    let (statut, corps) = service
        .post(
            &format!("/v1/requests/{demande}/accept"),
            Some(&service.jeton("c_hote_accept")),
            json!({}),
        )
        .await;
    assert_eq!(
        statut,
        StatusCode::OK,
        "l'hôte ne peut plus accepter : {corps}"
    );
}

/// Une « Escale » fait basculer le fil sur une autre ville.
///
/// C'est ce qui se vend 3,99 € : « publier et voir depuis une autre ville
/// pendant sept jours ». Le crédit était éprouvé — son achat, son débit, son
/// solde — mais jamais ce qu'il achète. Retirer la bascule du fil laissait
/// toute la suite au vert : on aurait pu vendre l'escale et ne rien livrer.
///
/// C'est exactement le défaut qu'`escaleCity` avait déjà eu : un mécanisme
/// complet à la ligne près qui le rend utile.
#[tokio::test]
async fn une_escale_fait_basculer_le_fil_sur_l_autre_ville() {
    let service = Service::monter().await;
    let voyageur = service.compte("c_voyageur", "depart").await;
    service.compte("c_reste_a_lyon", "depart").await;
    let parisien = service.compte("c_parisien", "depart").await;

    // Le Parisien vit à Paris ; les comptes de test naissent à Lyon.
    service
        .db
        .execute_unprepared(&format!(
            "UPDATE profiles SET city='Paris', latRounded=48.86, lonRounded=2.35 \
             WHERE accountId='{parisien}'"
        ))
        .await
        .unwrap();

    let a_lyon = plan_de(&service, "c_reste_a_lyon", "Un bouchon rue Mercière").await;
    let a_paris = plan_de(&service, "c_parisien", "Un concert à la Bellevilloise").await;

    // Avant l'escale : Lyon oui, Paris non — trois cents kilomètres.
    let avant = titres_du_fil(&service, "c_voyageur").await;
    assert!(
        avant.contains(&a_lyon),
        "le plan lyonnais manque : {avant:?}"
    );
    assert!(
        !avant.contains(&a_paris),
        "le plan parisien paraît sans escale : {avant:?}"
    );

    crate::routes::billing::crediter_pour_test(&service.etat, &voyageur, "escale", 1)
        .await
        .expect("achat de l'escale");

    let (statut, corps) = service
        .post(
            "/v1/me/escale",
            Some(&service.jeton("c_voyageur")),
            json!({ "city": "Paris" }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    // Après : le fil est celui de Paris.
    let apres = titres_du_fil(&service, "c_voyageur").await;
    assert!(
        apres.contains(&a_paris),
        "l'escale n'a pas ouvert le fil parisien : {apres:?}"
    );
    assert!(
        !apres.contains(&a_lyon),
        "l'escale n'a pas refermé le fil lyonnais : {apres:?}"
    );

    // Et le crédit a bien été dépensé.
    let (_, fiche) = service
        .get("/v1/me", Some(&service.jeton("c_voyageur")))
        .await;
    assert_eq!(
        fiche["credits"]["escale"], 0,
        "l'escale n'a rien coûté : {fiche}"
    );
}

/// Les plans de groupe se paient — par le palier, ou par « Tablée ».
///
/// C'est la deuxième chose que le catalogue vend et que rien n'éprouvait :
/// retirer la garde donnait les plans de groupe à tout le monde, et les trois
/// cent vingt tests restaient au vert.
///
/// Le test tient les trois chemins : refusé sans rien, accepté avec le crédit
/// — qui se dépense —, et accepté sans crédit sur un palier qui les comprend.
#[tokio::test]
async fn un_plan_de_groupe_se_paie() {
    let service = Service::monter().await;
    let solo = service.compte("c_solo", "depart").await;
    service.compte("c_escapade", "escapade").await;

    let groupe = |titre: &str| {
        json!({
            "title": titre,
            "category": "repas",
            "capacity": 4,
            "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
        })
    };

    // Sans rien : refusé, et l'on dit quoi acheter.
    let (statut, corps) = service
        .post(
            "/v1/plans",
            Some(&service.jeton("c_solo")),
            groupe("Une tablée de quatre au comptoir"),
        )
        .await;
    assert_ne!(
        statut,
        StatusCode::OK,
        "un plan de groupe est passé sans rien : {corps}"
    );
    assert_eq!(corps["details"]["sku"], "tablee", "{corps}");

    // Avec le crédit : accepté, et le crédit part.
    crate::routes::billing::crediter_pour_test(&service.etat, &solo, "tablee", 1)
        .await
        .expect("achat de la tablée");
    let (statut, corps) = service
        .post(
            "/v1/plans",
            Some(&service.jeton("c_solo")),
            groupe("Une tablée de quatre au comptoir"),
        )
        .await;
    assert_eq!(
        statut,
        StatusCode::OK,
        "le crédit n'a pas ouvert le groupe : {corps}"
    );

    let (_, fiche) = service.get("/v1/me", Some(&service.jeton("c_solo"))).await;
    assert_eq!(
        fiche["credits"]["tablee"], 0,
        "la tablée n'a rien coûté : {fiche}"
    );

    // Et un palier qui les comprend n'a pas besoin de crédit.
    let (statut, corps) = service
        .post(
            "/v1/plans",
            Some(&service.jeton("c_escapade")),
            groupe("Un dîner à six chez moi"),
        )
        .await;
    assert_eq!(
        statut,
        StatusCode::OK,
        "un palier qui comprend les plans de groupe en redemande le crédit : {corps}"
    );
}
