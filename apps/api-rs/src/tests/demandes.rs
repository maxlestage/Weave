//! Les demandes vues de part et d'autre : les miennes, celles que je reçois.
//!
//! Le quota journalier est l'invariant central du produit — c'est lui qui
//! empêche d'arroser. Ces tests portent autant sur ce qu'il rend que sur ce
//! qu'il retient.

use super::Service;
use axum::http::StatusCode;
use sea_orm::ConnectionTrait;
use serde_json::json;

/// Publie un plan et rend son identifiant.
async fn plan_de(service: &Service, hote: &str, titre: &str) -> String {
    let (statut, corps) = service
        .post(
            "/v1/plans",
            Some(&service.jeton(hote)),
            json!({
                "title": titre,
                "category": "balade",
                "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    corps["id"].as_str().unwrap().to_string()
}

async fn demander(
    service: &Service,
    invite: &str,
    plan_id: &str,
) -> (StatusCode, serde_json::Value) {
    service
        .post(
            "/v1/requests",
            Some(&service.jeton(invite)),
            json!({
                "planId": plan_id,
                "message": "Cette balade me tente beaucoup, je serais ravi de venir.",
            }),
        )
        .await
}

#[tokio::test]
async fn mes_demandes_envoyees_portent_le_plan_et_son_hote() {
    let service = Service::monter().await;
    service.compte("c_hote_envoi", "depart").await;
    service.compte("c_invite_envoi", "depart").await;
    let plan = plan_de(&service, "c_hote_envoi", "Une balade au bord de l eau").await;

    let (statut, _) = demander(&service, "c_invite_envoi", &plan).await;
    assert_eq!(statut, StatusCode::OK);

    let (statut, corps) = service
        .get("/v1/requests/sent", Some(&service.jeton("c_invite_envoi")))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["requestsLeftToday"], 4);

    let demandes = corps["requests"].as_array().expect("un tableau");
    assert_eq!(demandes.len(), 1, "{corps}");
    assert_eq!(demandes[0]["planId"], plan);
    assert_eq!(demandes[0]["planTitle"], "Une balade au bord de l eau");
    assert_eq!(demandes[0]["state"], "envoyee");
    assert_eq!(demandes[0]["author"]["id"], service.id("c_hote_envoi"));
    assert_eq!(demandes[0]["conversationId"], json!(null));
    // Les dates sortent au format de `toISOString()`.
    let envoyee = demandes[0]["sentAt"].as_str().unwrap();
    assert!(envoyee.ends_with('Z'), "date rendue : {envoyee}");
}

/// Se raviser vite ne doit pas coûter la journée.
#[tokio::test]
async fn retirer_une_demande_rend_son_unite() {
    let service = Service::monter().await;
    service.compte("c_hote_retrait", "depart").await;
    service.compte("c_invite_retrait", "depart").await;
    let plan = plan_de(&service, "c_hote_retrait", "Un plan dont on se ravise").await;

    let (_, demande) = demander(&service, "c_invite_retrait", &plan).await;
    let demande_id = demande["id"].as_str().expect("un identifiant");

    let (statut, _) = service
        .delete(
            &format!("/v1/requests/{demande_id}"),
            Some(&service.jeton("c_invite_retrait")),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (_, corps) = service
        .get("/v1/me", Some(&service.jeton("c_invite_retrait")))
        .await;
    assert_eq!(
        corps["requestsLeftToday"], 5,
        "l'unité n'a pas été rendue : {corps}"
    );
}

#[tokio::test]
async fn on_ne_retire_pas_la_demande_d_un_autre() {
    let service = Service::monter().await;
    service.compte("c_hote_vol", "depart").await;
    service.compte("c_invite_vol", "depart").await;
    service.compte("c_tiers_vol", "depart").await;
    let plan = plan_de(&service, "c_hote_vol", "Un plan que l on convoite").await;

    let (_, demande) = demander(&service, "c_invite_vol", &plan).await;
    let demande_id = demande["id"].as_str().unwrap();

    let (statut, _) = service
        .delete(
            &format!("/v1/requests/{demande_id}"),
            Some(&service.jeton("c_tiers_vol")),
        )
        .await;
    assert_eq!(statut, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn une_demande_deja_tranchee_ne_se_retire_plus() {
    let service = Service::monter().await;
    service.compte("c_hote_tranche", "depart").await;
    service.compte("c_invite_tranche", "depart").await;
    let plan = plan_de(&service, "c_hote_tranche", "Un plan deja tranche ici").await;

    let (_, demande) = demander(&service, "c_invite_tranche", &plan).await;
    let demande_id = demande["id"].as_str().unwrap().to_string();

    let (statut, _) = service
        .post(
            &format!("/v1/requests/{demande_id}/accept"),
            Some(&service.jeton("c_hote_tranche")),
            json!({}),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (statut, _) = service
        .delete(
            &format!("/v1/requests/{demande_id}"),
            Some(&service.jeton("c_invite_tranche")),
        )
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);
}

/// L'unité reste dépensée : elle a été lue. Un refus ne rend rien, sans quoi
/// écrire à tout-va redeviendrait gratuit.
#[tokio::test]
async fn refuser_une_demande_ne_rend_pas_l_unite() {
    let service = Service::monter().await;
    service.compte("c_hote_refus", "depart").await;
    service.compte("c_invite_refus", "depart").await;
    let plan = plan_de(&service, "c_hote_refus", "Un plan ou l on dit non").await;

    let (_, demande) = demander(&service, "c_invite_refus", &plan).await;
    let demande_id = demande["id"].as_str().unwrap().to_string();

    let (statut, _) = service
        .post(
            &format!("/v1/requests/{demande_id}/decline"),
            Some(&service.jeton("c_hote_refus")),
            json!({}),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (_, corps) = service
        .get("/v1/me", Some(&service.jeton("c_invite_refus")))
        .await;
    assert_eq!(
        corps["requestsLeftToday"], 4,
        "le refus a rendu l'unité : {corps}"
    );
}

#[tokio::test]
async fn seul_l_hote_refuse() {
    let service = Service::monter().await;
    service.compte("c_hote_seul", "depart").await;
    service.compte("c_invite_seul", "depart").await;
    let plan = plan_de(&service, "c_hote_seul", "Un plan que l on refuse mal").await;

    let (_, demande) = demander(&service, "c_invite_seul", &plan).await;
    let demande_id = demande["id"].as_str().unwrap();

    let (statut, _) = service
        .post(
            &format!("/v1/requests/{demande_id}/decline"),
            Some(&service.jeton("c_invite_seul")),
            json!({}),
        )
        .await;
    assert_eq!(statut, StatusCode::FORBIDDEN);
}

/// Les demandes reçues ne sont pas publiques : le message qu'on écrit pour
/// rejoindre un plan n'est lu que par qui l'organise.
#[tokio::test]
async fn les_demandes_recues_ne_se_lisent_qu_en_hote() {
    let service = Service::monter().await;
    service.compte("c_hote_recu", "depart").await;
    service.compte("c_invite_recu", "depart").await;
    service.compte("c_curieux", "depart").await;
    let plan = plan_de(&service, "c_hote_recu", "Un plan aux demandes privees").await;

    let (statut, _) = demander(&service, "c_invite_recu", &plan).await;
    assert_eq!(statut, StatusCode::OK);

    let (statut, corps) = service
        .get(
            &format!("/v1/plans/{plan}/requests"),
            Some(&service.jeton("c_hote_recu")),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    let recues = corps.as_array().expect("un tableau");
    assert_eq!(recues.len(), 1);
    assert_eq!(recues[0]["author"]["id"], service.id("c_invite_recu"));
    assert!(recues[0]["message"].as_str().unwrap().contains("tente"));

    let (statut, _) = service
        .get(
            &format!("/v1/plans/{plan}/requests"),
            Some(&service.jeton("c_curieux")),
        )
        .await;
    assert_eq!(
        statut,
        StatusCode::FORBIDDEN,
        "un tiers a lu les demandes reçues"
    );
}

#[tokio::test]
async fn mes_plans_comptent_les_places_et_les_demandes() {
    let service = Service::monter().await;
    service.compte("c_hote_miens", "depart").await;
    service.compte("c_invite_miens", "depart").await;
    let plan = plan_de(
        &service,
        "c_hote_miens",
        "Un plan dont on compte les places",
    )
    .await;

    let (statut, _) = demander(&service, "c_invite_miens", &plan).await;
    assert_eq!(statut, StatusCode::OK);

    let (statut, corps) = service
        .get("/v1/plans/mine", Some(&service.jeton("c_hote_miens")))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    let miens = corps.as_array().expect("un tableau");
    assert_eq!(miens.len(), 1, "{corps}");
    assert_eq!(miens[0]["id"], plan);
    assert_eq!(miens[0]["pendingRequests"], 1);
    assert_eq!(miens[0]["seatsLeft"], 1);
    assert_eq!(miens[0]["state"], "ouvert");
}

/// Le renfort ajoute cinq demandes, et seulement si l'unité a été achetée.
#[tokio::test]
async fn un_renfort_sans_credit_est_refuse_et_ne_grignote_pas_le_plafond() {
    let service = Service::monter().await;
    let compte = service.compte("c_renfort", "depart").await;

    let (statut, corps) = service
        .post(
            "/v1/requests/renfort",
            Some(&service.jeton("c_renfort")),
            json!({}),
        )
        .await;
    assert_eq!(statut, StatusCode::PAYMENT_REQUIRED, "{corps}");

    // La place réservée a bien été rendue : le quota du jour est inchangé.
    let (_, fiche) = service
        .get("/v1/me", Some(&service.jeton("c_renfort")))
        .await;
    assert_eq!(
        fiche["requestsLeftToday"], 5,
        "le plafond a été grignoté : {fiche}"
    );

    // Avec un crédit, il passe.
    service
        .db
        .execute_unprepared(&format!(
            "INSERT INTO credit_balances (id,accountId,sku,balance,updatedAt) \
             VALUES ('cb_{compte}','{compte}','renfort',1,'2026-09-12 10:00:00')"
        ))
        .await
        .unwrap();

    let (statut, corps) = service
        .post(
            "/v1/requests/renfort",
            Some(&service.jeton("c_renfort")),
            json!({}),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["granted"], 5);
    assert_eq!(
        corps["requestsLeftToday"], 10,
        "le renfort n'a pas augmenté le quota"
    );
}

/// La dernière place d'un plan ne doit être accordée qu'une fois.
///
/// La capacité est comptée AVANT d'ouvrir la transaction. Deux acceptations
/// concurrentes de demandes DIFFÉRENTES lisent donc toutes deux le même
/// compte, et passent toutes deux : le filtre d'état sur la demande ne
/// sérialise que deux acceptations de la MÊME demande.
#[tokio::test]
async fn deux_acceptations_concurrentes_ne_donnent_pas_deux_fois_la_meme_place() {
    let service = Service::monter().await;
    service.compte("c_hote_place", "depart").await;
    service.compte("c_premier_place", "depart").await;
    service.compte("c_second_place", "depart").await;

    // Un plan en solo : une seule place à prendre.
    let plan = plan_de(&service, "c_hote_place", "Un cafe en tete a tete").await;

    let (s1, c1) = demander(&service, "c_premier_place", &plan).await;
    assert_eq!(s1, StatusCode::OK, "{c1}");
    let (s2, c2) = demander(&service, "c_second_place", &plan).await;
    assert_eq!(s2, StatusCode::OK, "{c2}");

    let premier = c1["id"].as_str().unwrap().to_string();
    let second = c2["id"].as_str().unwrap().to_string();
    let jeton = service.jeton("c_hote_place");

    let route_premier = format!("/v1/requests/{premier}/accept");
    let route_second = format!("/v1/requests/{second}/accept");
    let (a, b) = tokio::join!(
        service.post(&route_premier, Some(&jeton), json!({})),
        service.post(&route_second, Some(&jeton), json!({})),
    );

    let acceptees = [a.0, b.0].iter().filter(|s| s.is_success()).count();
    assert_eq!(
        acceptees, 1,
        "une seule place, une seule acceptation — obtenu {acceptees} (réponses : {a:?} / {b:?})"
    );
}

/// Deux retraits simultanés de la même demande ne rendent qu'une unité.
///
/// Le retrait lisait l'état, le contrôlait, puis écrivait : deux appels
/// concurrents franchissaient le contrôle ensemble et REMBOURSAIENT TOUS LES
/// DEUX. Le script Lua du cache borne le compteur à zéro — il empêche de passer
/// sous le plancher, pas de récupérer plus qu'on n'a dépensé.
///
/// Le quota journalier est l'invariant central du produit : « on ne peut pas
/// arroser, et aucun achat ne lève cette limite du jour ». Cinq retraits
/// simultanés d'une seule demande rendaient cinq unités.
#[tokio::test]
async fn deux_retraits_simultanes_ne_rendent_qu_une_unite() {
    let service = Service::monter().await;
    service.compte("c_hote_course", "depart").await;
    service.compte("c_invite_course", "depart").await;
    let jeton = service.jeton("c_invite_course");

    // Trois demandes envoyées : le compteur est à trois, loin du plancher que
    // le script Lua défend. C'est au-dessus de zéro que la course se voit.
    let mut premiere = String::new();
    for n in 0..3 {
        let plan = plan_de(
            &service,
            "c_hote_course",
            &format!("Un plan numero {n} ici"),
        )
        .await;
        let (_, demande) = demander(&service, "c_invite_course", &plan).await;
        if n == 0 {
            premiere = demande["id"].as_str().expect("un identifiant").to_string();
        }
    }

    let (_, avant) = service.get("/v1/me", Some(&jeton)).await;
    assert_eq!(
        avant["requestsLeftToday"], 2,
        "trois demandes envoyées : {avant}"
    );

    // Quatre retraits de LA MÊME demande, lancés ensemble.
    let chemin = format!("/v1/requests/{premiere}");
    let (a, b, c, d) = tokio::join!(
        service.delete(&chemin, Some(&jeton)),
        service.delete(&chemin, Some(&jeton)),
        service.delete(&chemin, Some(&jeton)),
        service.delete(&chemin, Some(&jeton)),
    );
    let reussis = [a, b, c, d]
        .iter()
        .filter(|(s, _)| *s == StatusCode::OK)
        .count();
    assert_eq!(reussis, 1, "un seul retrait doit aboutir");

    let (_, apres) = service.get("/v1/me", Some(&jeton)).await;
    assert_eq!(
        apres["requestsLeftToday"], 3,
        "un seul retrait, donc une seule unité rendue : {apres}"
    );
}

/// « Mes plans » et les demandes reçues ne mélangent pas les lignes entre elles.
///
/// Chaque plan coûtait DEUX comptages, et chaque demande reçue DEUX requêtes —
/// l'auteur et sa fiche. Tout tient maintenant en requêtes groupées, et ce
/// test tient ce qui doit y survivre : chaque plan porte SES places restantes
/// et SES demandes en attente, chaque demande SON auteur.
#[tokio::test]
async fn mes_plans_et_leurs_demandes_ne_melangent_pas_les_lignes() {
    let service = Service::monter().await;
    // Palier « virée » : les plans de groupe y sont compris, donc une capacité
    // supérieure à une place — sans quoi une acceptation remplit le plan et
    // les deux comptages ne se distinguent plus.
    service.compte("c_mel_hote", "viree").await;
    for n in 0..3 {
        service.compte(&format!("c_mel_invite{n}"), "depart").await;
    }

    let creux = plan_de_groupe(&service, "Un plan que personne ne demande").await;
    let couru = plan_de_groupe(&service, "Un plan que tout le monde demande").await;

    // Trois demandes sur le second, aucune sur le premier — et l'une acceptée.
    let mut demandes = Vec::new();
    for n in 0..3 {
        let (_, corps) = demander(&service, &format!("c_mel_invite{n}"), &couru).await;
        demandes.push(corps["id"].as_str().expect("un identifiant").to_string());
    }
    let (statut, corps) = service
        .post(
            &format!("/v1/requests/{}/accept", demandes[0]),
            Some(&service.jeton("c_mel_hote")),
            json!({}),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    // Mes plans : chacun ses propres comptes.
    let (_, miens) = service
        .get("/v1/plans/mine", Some(&service.jeton("c_mel_hote")))
        .await;
    let ligne = |id: &str| {
        miens
            .as_array()
            .expect("une liste")
            .iter()
            .find(|p| p["id"] == id)
            .unwrap_or_else(|| panic!("plan absent : {miens}"))
            .clone()
    };

    let vide = ligne(&creux);
    assert_eq!(vide["pendingRequests"], 0, "le plan sans demande : {vide}");
    assert_eq!(vide["seatsLeft"], 4, "le plan sans demande : {vide}");

    let plein = ligne(&couru);
    assert_eq!(
        plein["pendingRequests"], 2,
        "deux demandes restent en attente : {plein}"
    );
    assert_eq!(
        plein["seatsLeft"], 3,
        "une acceptation prend une place : {plein}"
    );

    // Les demandes reçues : chacune son auteur.
    let (statut, recues) = service
        .get(
            &format!("/v1/plans/{couru}/requests"),
            Some(&service.jeton("c_mel_hote")),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{recues}");
    let recues = recues.as_array().expect("une liste");
    assert_eq!(
        recues.len(),
        2,
        "seules les demandes en attente sont rendues : {recues:?}"
    );

    for demande in recues {
        let auteur = demande["author"]["id"].as_str().unwrap_or_default();
        assert!(
            auteur.starts_with("c_mel_invite"),
            "l'auteur rendu n'est pas celui de la demande : {demande}"
        );
        assert_eq!(
            demande["author"]["displayName"],
            format!("Compte {auteur}"),
            "le nom rendu n'est pas celui de l'auteur : {demande}"
        );
    }

    // Deux demandes, deux auteurs distincts.
    let auteurs: std::collections::HashSet<&str> = recues
        .iter()
        .filter_map(|d| d["author"]["id"].as_str())
        .collect();
    assert_eq!(
        auteurs.len(),
        2,
        "le même auteur rendu deux fois : {recues:?}"
    );
}

/// Un plan de groupe publié par « c_mel_hote ».
async fn plan_de_groupe(service: &Service, titre: &str) -> String {
    let (statut, corps) = service
        .post(
            "/v1/plans",
            Some(&service.jeton("c_mel_hote")),
            json!({
                "title": titre,
                "category": "balade",
                "capacity": 4,
                "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    corps["id"].as_str().expect("un identifiant").to_string()
}

/// Le plafond journalier refuse la demande de trop.
///
/// C'est l'invariant central du produit — celui qui distingue les paliers, et
/// donc ce qui se paie. Les tests suivaient bien le compteur qui descend, mais
/// aucun n'allait jusqu'au bout : retirer la ligne qui compare aux plafond
/// laissait les trois cent onze tests au vert. On peut lever le plafond de
/// tout le monde sans que rien ne le dise.
#[tokio::test]
async fn le_plafond_journalier_refuse_la_demande_de_trop() {
    let service = Service::monter().await;
    service.compte("c_plafond", "depart").await;
    let jeton = service.jeton("c_plafond");

    // Le palier de départ en accorde cinq par jour. Il faut donc au moins six
    // plans à demander — un par demande, on ne redemande pas deux fois.
    let quota = crate::droits::droits_pour("depart").demandes_par_jour as usize;
    let mut plans = Vec::new();
    for numero in 0..quota + 1 {
        let hote = format!("c_hote_plafond_{numero}");
        service.compte(&hote, "depart").await;
        let (statut, corps) = service
            .post(
                "/v1/plans",
                Some(&service.jeton(&hote)),
                json!({
                    "title": format!("Un plan de plus, le numero {numero}"),
                    "category": "sortie",
                    "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
                }),
            )
            .await;
        assert_eq!(statut, StatusCode::OK, "{corps}");
        plans.push(corps["id"].as_str().expect("identifiant").to_string());
    }

    // Les cinq premières passent, et le compteur descend jusqu'à zéro.
    for (numero, plan) in plans.iter().take(quota).enumerate() {
        let (statut, corps) = service
            .post(
                "/v1/requests",
                Some(&jeton),
                json!({ "planId": plan, "message": "Ce plan me tente beaucoup, je viendrais volontiers." }),
            )
            .await;
        assert_eq!(statut, StatusCode::OK, "demande {numero} refusée : {corps}");
        assert_eq!(
            corps["requestsLeftToday"],
            json!(quota - numero - 1),
            "le compteur ne descend pas comme annoncé : {corps}"
        );
    }

    // La sixième est refusée, et le plafond tient.
    let (statut, corps) = service
        .post(
            "/v1/requests",
            Some(&jeton),
            json!({ "planId": plans[quota], "message": "Ce plan me tente beaucoup, je viendrais volontiers." }),
        )
        .await;
    assert_ne!(
        statut,
        StatusCode::OK,
        "la demande au-delà du plafond est passée : {corps}"
    );

    // Et le compte n'a toujours rien de plus à dépenser.
    let (_, fiche) = service.get("/v1/me", Some(&jeton)).await;
    assert_eq!(
        fiche["requestsLeftToday"], 0,
        "un refus a rendu une demande : {fiche}"
    );
}

// ————————————————————————————————————————————————————————————————————————
// Rendre sa place après avoir été accepté
//
// Ce que le produit ne permettait pas. Une fois accepté, on ne pouvait plus
// reculer : la place restait prise, l'auteur attendait au café sans rien
// savoir, et la seule sortie était de ne pas venir.
// ————————————————————————————————————————————————————————————————————————

/// Publie un plan pour une seule personne, et rend son identifiant.
async fn plan_solo(service: &Service, hote: &str, titre: &str) -> String {
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

/// Demande, fait accepter, et rend l'identifiant de la demande.
async fn accepte(service: &Service, hote: &str, invite: &str, plan: &str) -> String {
    let (statut, corps) = demander(service, invite, plan).await;
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

async fn etat_du_plan(service: &Service, plan: &str) -> String {
    let ligne = service
        .db
        .query_one_raw(sea_orm::Statement::from_string(
            service.db.get_database_backend(),
            format!("SELECT state FROM plans WHERE id = '{plan}'"),
        ))
        .await
        .expect("plan lu")
        .expect("plan présent");
    ligne.try_get::<String>("", "state").expect("état lisible")
}

#[tokio::test]
async fn rendre_sa_place_rouvre_le_plan_et_le_remet_au_fil() {
    let service = Service::monter().await;
    service.compte("c_hote_rendu", "depart").await;
    service.compte("c_parti_rendu", "depart").await;
    let plan = plan_solo(&service, "c_hote_rendu", "Une balade au bord de l eau").await;
    let demande = accepte(&service, "c_hote_rendu", "c_parti_rendu", &plan).await;

    // La place prise ferme le plan : c'est l'état de départ de l'épreuve.
    assert_eq!(etat_du_plan(&service, &plan).await, "complet");

    let (statut, corps) = service
        .post(
            &format!("/v1/requests/{demande}/release"),
            Some(&service.jeton("c_parti_rendu")),
            json!({}),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    // La place est rendue, et le plan repart au fil des autres.
    assert_eq!(
        etat_du_plan(&service, &plan).await,
        "ouvert",
        "le plan reste fermé pour quelqu'un qui ne viendra pas"
    );

    // Et quelqu'un d'autre peut la prendre. C'est la seule preuve qui compte :
    // rouvrir le plan sans que la place soit réellement libre ne servirait à
    // rien, puisque l'acceptation recompte les places prises.
    service.compte("c_autre_rendu", "depart").await;
    let (statut, corps) = demander(&service, "c_autre_rendu", &plan).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    let suivante = corps["id"].as_str().unwrap().to_string();
    let (statut, corps) = service
        .post(
            &format!("/v1/requests/{suivante}/accept"),
            Some(&service.jeton("c_hote_rendu")),
            json!({}),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(etat_du_plan(&service, &plan).await, "complet");
}

#[tokio::test]
async fn rendre_sa_place_ne_rend_pas_l_unite_de_quota() {
    let service = Service::monter().await;
    service.compte("c_hote_quota_rendu", "depart").await;
    service.compte("c_parti_quota", "depart").await;
    let plan = plan_solo(&service, "c_hote_quota_rendu", "Un cafe pres du canal").await;
    let demande = accepte(&service, "c_hote_quota_rendu", "c_parti_quota", &plan).await;

    let (statut, _) = service
        .post(
            &format!("/v1/requests/{demande}/release"),
            Some(&service.jeton("c_parti_quota")),
            json!({}),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);

    // « Une demande retirée AVANT D'AVOIR ÉTÉ LUE vous est rendue », dit le
    // site. Celle-ci a été lue, et il y a été répondu : l'unité est dépensée.
    // La rendre ferait d'un désistement un moyen d'écrire sans compter.
    let (statut, corps) = service
        .get("/v1/requests/sent", Some(&service.jeton("c_parti_quota")))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(
        corps["requestsLeftToday"], 4,
        "l'unité a été rendue alors que la demande avait été lue"
    );
}

#[tokio::test]
async fn seul_celui_qui_tient_la_place_peut_la_rendre() {
    let service = Service::monter().await;
    service.compte("c_hote_vol", "depart").await;
    service.compte("c_tenant_vol", "depart").await;
    service.compte("c_tiers_vol", "depart").await;
    let plan = plan_solo(&service, "c_hote_vol", "Un concert au parc").await;
    let demande = accepte(&service, "c_hote_vol", "c_tenant_vol", &plan).await;

    // Ni un tiers, ni l'auteur du plan : rendre une place est une décision de
    // celui qui l'occupe. L'auteur, lui, annule son plan — ce n'est pas la
    // même chose, et cela se dit autrement.
    for intrus in ["c_tiers_vol", "c_hote_vol"] {
        let (statut, corps) = service
            .post(
                &format!("/v1/requests/{demande}/release"),
                Some(&service.jeton(intrus)),
                json!({}),
            )
            .await;
        assert_eq!(statut, StatusCode::FORBIDDEN, "{intrus} : {corps}");
    }
    assert_eq!(etat_du_plan(&service, &plan).await, "complet");
}

#[tokio::test]
async fn on_ne_rend_que_la_place_qu_on_a() {
    let service = Service::monter().await;
    service.compte("c_hote_sans", "depart").await;
    service.compte("c_invite_sans", "depart").await;
    let plan = plan_de(&service, "c_hote_sans", "Une expo le samedi").await;

    // Une demande encore en attente n'est pas une place : elle se RETIRE, et
    // ce retrait-là rend l'unité de quota. Confondre les deux rendrait une
    // unité à qui n'a encore reçu aucune réponse.
    let (statut, corps) = demander(&service, "c_invite_sans", &plan).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    let demande = corps["id"].as_str().unwrap().to_string();

    let (statut, corps) = service
        .post(
            &format!("/v1/requests/{demande}/release"),
            Some(&service.jeton("c_invite_sans")),
            json!({}),
        )
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY, "{corps}");

    // Et une place déjà rendue ne se rend pas deux fois.
    let (statut, _) = service
        .post(
            &format!("/v1/requests/{demande}/accept"),
            Some(&service.jeton("c_hote_sans")),
            json!({}),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);
    let chemin = format!("/v1/requests/{demande}/release");
    let (premier, _) = service
        .post(&chemin, Some(&service.jeton("c_invite_sans")), json!({}))
        .await;
    assert_eq!(premier, StatusCode::OK);
    let (second, corps) = service
        .post(&chemin, Some(&service.jeton("c_invite_sans")), json!({}))
        .await;
    assert_eq!(second, StatusCode::UNPROCESSABLE_ENTITY, "{corps}");
}

#[tokio::test]
async fn on_ne_se_desiste_plus_une_fois_l_heure_passee() {
    let service = Service::monter().await;
    service.compte("c_hote_tard", "depart").await;
    service.compte("c_parti_tard", "depart").await;
    let plan = plan_solo(&service, "c_hote_tard", "Un brunch dimanche matin").await;
    let demande = accepte(&service, "c_hote_tard", "c_parti_tard", &plan).await;

    // L'heure est reculée en base : la route refuse de publier dans le passé,
    // et c'est le passage du temps qu'on éprouve, pas la publication.
    service
        .db
        .execute_unprepared(&format!(
            "UPDATE plans SET startsAt = '2020-01-01 00:00:00' WHERE id = '{plan}'"
        ))
        .await
        .expect("heure reculée");

    let (statut, corps) = service
        .post(
            &format!("/v1/requests/{demande}/release"),
            Some(&service.jeton("c_parti_tard")),
            json!({}),
        )
        .await;
    assert_eq!(
        statut,
        StatusCode::UNPROCESSABLE_ENTITY,
        "se désister après coup ne rend plus rien à personne : {corps}"
    );
}

#[tokio::test]
async fn deux_desistements_concurrents_ne_rendent_qu_une_place() {
    let service = Service::monter().await;
    service.compte("c_hote_deux", "depart").await;
    service.compte("c_parti_deux", "depart").await;
    // Un plan de groupe se vend (« Tablée ») : le solo suffit ici, puisque
    // c'est la MÊME place que deux appels tentent de rendre.
    let plan = plan_solo(&service, "c_hote_deux", "Une partie de cartes au bar").await;
    let demande = accepte(&service, "c_hote_deux", "c_parti_deux", &plan).await;

    let chemin = format!("/v1/requests/{demande}/release");
    let jeton = service.jeton("c_parti_deux");
    let (a, b) = tokio::join!(
        service.post(&chemin, Some(&jeton), json!({})),
        service.post(&chemin, Some(&jeton), json!({}))
    );

    let reussites = [a.0, b.0].iter().filter(|s| s.is_success()).count();
    assert_eq!(
        reussites, 1,
        "deux désistements de la même place ont abouti : {:?} {:?}",
        a.1, b.1
    );
}

#[tokio::test]
async fn annuler_un_plan_laisse_les_personnes_acceptees_dans_leur_etat() {
    let service = Service::monter().await;
    service.compte("c_hote_annul", "depart").await;
    service.compte("c_attendu_annul", "depart").await;
    let plan = plan_solo(&service, "c_hote_annul", "Un marche le dimanche").await;
    let demande = accepte(&service, "c_hote_annul", "c_attendu_annul", &plan).await;

    let (statut, corps) = service
        .delete(
            &format!("/v1/plans/{plan}"),
            Some(&service.jeton("c_hote_annul")),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    // La demande acceptée n'est pas effacée : la conversation reste ouverte,
    // c'est là qu'on explique une annulation. Ce qui manquait n'était pas une
    // écriture en base, c'était de PRÉVENIR — et le plan annulé ne doit plus
    // pouvoir se rouvrir par un désistement.
    let (statut, corps) = service
        .post(
            &format!("/v1/requests/{demande}/release"),
            Some(&service.jeton("c_attendu_annul")),
            json!({}),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(
        etat_du_plan(&service, &plan).await,
        "annule",
        "un plan annulé s'est rouvert parce que quelqu'un a rendu sa place"
    );
}

#[tokio::test]
async fn rendre_sa_place_previent_l_auteur() {
    let service = Service::monter().await;
    let hote = service.compte("c_hote_prev", "depart").await;
    service.compte("c_parti_prev", "depart").await;
    let telephone = service.appareil(&hote).await;
    let plan = plan_solo(&service, "c_hote_prev", "Un cafe pres du canal").await;
    let demande = accepte(&service, "c_hote_prev", "c_parti_prev", &plan).await;

    let (statut, corps) = service
        .post(
            &format!("/v1/requests/{demande}/release"),
            Some(&service.jeton("c_parti_prev")),
            json!({}),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    // C'est la raison d'être de cette route. Sans cette poussée, elle
    // n'éviterait l'attente au café qu'à celui qui se désiste — pas à celui
    // qui attend.
    let alertes = service.alertes_vers(&telephone);
    assert_eq!(
        alertes.len(),
        1,
        "l'auteur n'a pas été prévenu : {alertes:?}"
    );
    assert!(
        alertes[0].0.contains("place"),
        "l'alerte ne dit pas ce qui a changé : {:?}",
        alertes[0]
    );
    // Et elle ne nomme personne : elle s'affiche sur un écran verrouillé.
    let assemble = format!("{} {}", alertes[0].0, alertes[0].1);
    assert!(!assemble.contains("c_parti_prev"), "{assemble}");
}

#[tokio::test]
async fn annuler_un_plan_previent_les_personnes_attendues() {
    let service = Service::monter().await;
    service.compte("c_hote_dit", "depart").await;
    let attendu = service.compte("c_attendu_dit", "depart").await;
    let telephone = service.appareil(&attendu).await;
    let plan = plan_solo(&service, "c_hote_dit", "Un marche le dimanche").await;
    accepte(&service, "c_hote_dit", "c_attendu_dit", &plan).await;

    let (statut, corps) = service
        .delete(
            &format!("/v1/plans/{plan}"),
            Some(&service.jeton("c_hote_dit")),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    // C'était le pire des silences du produit : l'auteur annulait, et les
    // personnes attendues n'en savaient rien. Elles seraient venues, et leur
    // bannière annonçait encore le rendez-vous.
    let alertes = service.alertes_vers(&telephone);
    assert_eq!(
        alertes.len(),
        1,
        "personne n'a prévenu qui était attendu : {alertes:?}"
    );
    assert!(
        alertes[0].0.contains("annul"),
        "l'alerte ne dit pas que le plan n'aura pas lieu : {:?}",
        alertes[0]
    );
}

#[tokio::test]
async fn une_demande_encore_en_attente_ne_fait_prevenir_personne_a_l_annulation() {
    let service = Service::monter().await;
    service.compte("c_hote_muet", "depart").await;
    let demandeur = service.compte("c_demandeur_muet", "depart").await;
    let telephone = service.appareil(&demandeur).await;
    let plan = plan_de(&service, "c_hote_muet", "Une expo le samedi").await;

    let (statut, _) = demander(&service, "c_demandeur_muet", &plan).await;
    assert_eq!(statut, StatusCode::OK);

    let (statut, _) = service
        .delete(
            &format!("/v1/plans/{plan}"),
            Some(&service.jeton("c_hote_muet")),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);

    // Une demande sans réponse se clôt d'elle-même, et l'application le montre
    // à l'ouverture. Faire vibrer le téléphone de quelqu'un pour lui apprendre
    // qu'un plan auquel il n'était pas encore convié n'aura pas lieu ferait de
    // chaque annulation une notification de plus à subir.
    assert!(
        service.alertes_vers(&telephone).is_empty(),
        "une demande en attente a déclenché une alerte d'annulation"
    );
}

#[tokio::test]
async fn un_desistement_rouvre_le_plan_meme_au_dela_du_plafond() {
    let service = Service::monter().await;
    service.compte("c_hote_dep", "depart").await;
    service.compte("c_parti_dep", "depart").await;

    let plein = plan_solo(&service, "c_hote_dep", "Un cafe pres du canal").await;
    let demande = accepte(&service, "c_hote_dep", "c_parti_dep", &plein).await;
    assert_eq!(etat_du_plan(&service, &plein).await, "complet");

    // L'auteur remplit son plafond avec trois autres plans. Le quatrième,
    // complet, n'y compte pas : il n'est plus au fil.
    for titre in ["Une expo le samedi", "Un concert au parc", "Un marche dimanche"] {
        plan_solo(&service, "c_hote_dep", titre).await;
    }

    // Le désistement rouvre quand même. C'est l'inverse du choix fait à la
    // modification d'un plan, et pour une raison : là, l'auteur AGIT ; ici,
    // c'est quelqu'un qui rend sa place. Refuser un désistement pour que
    // l'auteur reste sous une borne reviendrait à retenir une personne sur un
    // plan pour le confort d'une autre.
    let (statut, corps) = service
        .post(
            &format!("/v1/requests/{demande}/release"),
            Some(&service.jeton("c_parti_dep")),
            json!({}),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(etat_du_plan(&service, &plein).await, "ouvert");

    // La borne se rattrape d'elle-même : l'auteur ne publie plus tant qu'il
    // est au-dessus. Elle n'est pas abandonnée, seulement dépassée un temps.
    let (statut, corps) = service
        .post(
            "/v1/plans",
            Some(&service.jeton("c_hote_dep")),
            json!({
                "title": "Un cinquieme plan de trop",
                "category": "balade",
                "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
            }),
        )
        .await;
    assert_ne!(statut, StatusCode::OK, "le plafond a été abandonné : {corps}");
}
