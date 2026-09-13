//! Les conversations : ce qui s'ouvre quand une demande est acceptée.
//!
//! Une conversation est toujours à deux, même sur un plan de groupe — on parle
//! à quelqu'un, pas à une salle. Elle naît d'une acceptation et se ferme à la
//! main ; il n'existe aucun moyen d'en ouvrir une autrement, ni d'écrire à
//! quelqu'un qui n'a pas dit oui.

use super::Service;
use axum::http::StatusCode;
use serde_json::json;

/// Déroule un plan, une demande et son acceptation, et rend l'identifiant de
/// la conversation ouverte.
async fn conversation_ouverte(service: &Service, hote: &str, invite: &str) -> String {
    service.compte(hote, "depart").await;
    service.compte(invite, "depart").await;

    let (statut, plan) = service
        .post(
            "/v1/plans",
            Some(&service.jeton(hote)),
            json!({
                "title": "Un plan dont on parlera ensuite",
                "category": "balade",
                "startsAt": (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339(),
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{plan}");

    let (statut, demande) = service
        .post(
            "/v1/requests",
            Some(&service.jeton(invite)),
            json!({
                "planId": plan["id"].as_str().unwrap(),
                "message": "Cette balade me tente beaucoup, je serais ravi de venir.",
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{demande}");

    let (statut, accepte) = service
        .post(
            &format!("/v1/requests/{}/accept", demande["id"].as_str().unwrap()),
            Some(&service.jeton(hote)),
            json!({}),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{accepte}");
    accepte["conversationId"]
        .as_str()
        .expect("une conversation ouverte par l'acceptation")
        .to_string()
}

#[tokio::test]
async fn mes_conversations_montrent_l_autre_et_le_plan() {
    let service = Service::monter().await;
    let conversation = conversation_ouverte(&service, "c_hote_conv", "c_invite_conv").await;

    let (statut, corps) = service
        .get("/v1/conversations", Some(&service.jeton("c_invite_conv")))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let liste = corps.as_array().expect("un tableau");
    assert_eq!(liste.len(), 1, "{corps}");
    assert_eq!(liste[0]["id"], conversation);
    assert_eq!(liste[0]["planTitle"], "Un plan dont on parlera ensuite");
    // L'autre, vu de l'invité, c'est l'hôte.
    assert_eq!(liste[0]["other"]["id"], service.id("c_hote_conv"));
    assert_eq!(liste[0]["lastMessage"], json!(null));
    assert_eq!(liste[0]["unread"], 0);
    assert_eq!(liste[0]["closed"], false);
}

#[tokio::test]
async fn lire_une_conversation_rend_les_messages_dans_l_ordre() {
    let service = Service::monter().await;
    let conversation = conversation_ouverte(&service, "c_hote_lit", "c_invite_lit").await;

    for texte in ["Bonjour, rendez-vous ou ?", "Au pont, vers 14h."] {
        let (statut, _) = service
            .post(
                &format!("/v1/conversations/{conversation}/messages"),
                Some(&service.jeton("c_hote_lit")),
                json!({ "body": texte }),
            )
            .await;
        assert_eq!(statut, StatusCode::OK);
    }

    let (statut, corps) = service
        .get(
            &format!("/v1/conversations/{conversation}/messages"),
            Some(&service.jeton("c_invite_lit")),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let messages = corps["messages"].as_array().expect("un tableau");
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["body"], "Bonjour, rendez-vous ou ?");
    assert_eq!(messages[1]["body"], "Au pont, vers 14h.");
    // Vue de l'invité, les deux viennent de l'autre.
    assert_eq!(messages[0]["author"], "autre");
}

/// Ouvrir la conversation marque comme lus les messages de l'autre — jamais
/// les siens, sans quoi le compteur de non-lus ne voudrait plus rien dire.
#[tokio::test]
async fn ouvrir_une_conversation_solde_les_non_lus_de_l_autre() {
    let service = Service::monter().await;
    let conversation = conversation_ouverte(&service, "c_hote_lu", "c_invite_lu").await;

    let (statut, _) = service
        .post(
            &format!("/v1/conversations/{conversation}/messages"),
            Some(&service.jeton("c_hote_lu")),
            json!({ "body": "Un message que l'autre n'a pas encore lu." }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (_, avant) = service
        .get("/v1/conversations", Some(&service.jeton("c_invite_lu")))
        .await;
    assert_eq!(avant[0]["unread"], 1, "{avant}");
    // L'expéditeur, lui, n'a rien de non lu : son propre message ne compte pas.
    let (_, cote_hote) = service
        .get("/v1/conversations", Some(&service.jeton("c_hote_lu")))
        .await;
    assert_eq!(cote_hote[0]["unread"], 0, "{cote_hote}");

    let (statut, _) = service
        .get(
            &format!("/v1/conversations/{conversation}/messages"),
            Some(&service.jeton("c_invite_lu")),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (_, apres) = service
        .get("/v1/conversations", Some(&service.jeton("c_invite_lu")))
        .await;
    assert_eq!(
        apres[0]["unread"], 0,
        "les non-lus n'ont pas été soldés : {apres}"
    );
    assert_eq!(
        apres[0]["lastMessage"],
        "Un message que l'autre n'a pas encore lu."
    );
}

#[tokio::test]
async fn une_conversation_close_n_accepte_plus_de_message() {
    let service = Service::monter().await;
    let conversation = conversation_ouverte(&service, "c_hote_clos", "c_invite_clos").await;

    let (statut, _) = service
        .delete(
            &format!("/v1/conversations/{conversation}"),
            Some(&service.jeton("c_invite_clos")),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (statut, _) = service
        .post(
            &format!("/v1/conversations/{conversation}/messages"),
            Some(&service.jeton("c_hote_clos")),
            json!({ "body": "Est-ce que quelqu'un m'entend encore ?" }),
        )
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);

    let (_, liste) = service
        .get("/v1/conversations", Some(&service.jeton("c_hote_clos")))
        .await;
    assert_eq!(liste[0]["closed"], true);
}

/// Clore ce qui est clos n'est pas une erreur : le client peut réessayer.
#[tokio::test]
async fn clore_deux_fois_ne_change_rien() {
    let service = Service::monter().await;
    let conversation = conversation_ouverte(&service, "c_hote_deux", "c_invite_deux").await;

    for _ in 0..2 {
        let (statut, _) = service
            .delete(
                &format!("/v1/conversations/{conversation}"),
                Some(&service.jeton("c_hote_deux")),
            )
            .await;
        assert_eq!(statut, StatusCode::OK);
    }
}

#[tokio::test]
async fn une_conversation_d_autrui_ne_se_lit_pas() {
    let service = Service::monter().await;
    let conversation = conversation_ouverte(&service, "c_hote_prive", "c_invite_prive").await;
    service.compte("c_curieux_conv", "depart").await;

    for chemin in [format!("/v1/conversations/{conversation}/messages")] {
        let (statut, _) = service
            .get(&chemin, Some(&service.jeton("c_curieux_conv")))
            .await;
        assert_eq!(statut, StatusCode::FORBIDDEN, "{chemin} s'est laissé lire");
    }

    let (statut, _) = service
        .delete(
            &format!("/v1/conversations/{conversation}"),
            Some(&service.jeton("c_curieux_conv")),
        )
        .await;
    assert_eq!(statut, StatusCode::FORBIDDEN);
}

/// La liste ne montre que les siennes : c'est une requête, pas un filtrage
/// côté client.
#[tokio::test]
async fn la_liste_ne_montre_que_les_siennes() {
    let service = Service::monter().await;
    conversation_ouverte(&service, "c_hote_tiers", "c_invite_tiers").await;
    service.compte("c_etranger", "depart").await;

    let (statut, corps) = service
        .get("/v1/conversations", Some(&service.jeton("c_etranger")))
        .await;
    assert_eq!(statut, StatusCode::OK);
    assert_eq!(corps.as_array().expect("un tableau").len(), 0, "{corps}");
}

/// Ouvre une conversation de plus pour un hôte DÉJÀ créé.
///
/// `conversation_ouverte` crée les deux comptes : s'en servir trois fois donne
/// trois hôtes d'une conversation chacun, ce qui ne met jamais deux lignes
/// dans la même liste. C'est précisément ce qu'il faut pour éprouver un
/// regroupement.
async fn conversation_de_plus(service: &Service, hote: &str, invite: &str, titre: &str) -> String {
    service.compte(invite, "depart").await;

    let (statut, plan) = service
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
    assert_eq!(statut, StatusCode::OK, "{plan}");

    let (statut, demande) = service
        .post(
            "/v1/requests",
            Some(&service.jeton(invite)),
            json!({
                "planId": plan["id"].as_str().unwrap(),
                "message": "Cette balade me tente beaucoup, je serais ravi de venir.",
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{demande}");

    let (statut, accepte) = service
        .post(
            &format!("/v1/requests/{}/accept", demande["id"].as_str().unwrap()),
            Some(&service.jeton(hote)),
            json!({}),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{accepte}");
    accepte["conversationId"]
        .as_str()
        .expect("une conversation")
        .to_string()
}

/// La liste dit vrai sur PLUSIEURS conversations d'un même compte.
///
/// Elle interrogeait la base cinq fois par conversation — le plan, l'autre
/// personne, sa fiche pour la photo, le dernier message, le compte des
/// non-lus. Cent conversations sont rendues au plus, soit jusqu'à cinq cents
/// allers-retours pour ouvrir l'écran des messages.
///
/// Tout est chargé en cinq requêtes. Ce test tient ce qui doit survivre au
/// regroupement : chaque ligne porte SON dernier message, SON compte de
/// non-lus et SON plan — et non ceux de sa voisine.
///
/// Ma première version donnait un hôte différent à chaque conversation : les
/// listes n'en contenaient qu'une, et prendre n'importe laquelle donnait la
/// bonne réponse. Elle passait avec le mélange en place.
#[tokio::test]
async fn la_liste_ne_melange_pas_les_conversations_d_un_meme_compte() {
    let service = Service::monter().await;
    service.compte("c_liste_hote", "depart").await;

    let mut ouvertes = Vec::new();
    for n in 0..3 {
        let invite = format!("c_liste_invite{n}");
        let titre = format!("Un plan numero {n} dont on parle");
        ouvertes.push((
            conversation_de_plus(&service, "c_liste_hote", &invite, &titre).await,
            invite,
            titre,
        ));
    }

    // Un nombre et un texte différents par conversation : un mélange se voit.
    for (rang, (conversation, invite, _)) in ouvertes.iter().enumerate() {
        for message in 0..=rang {
            let (statut, corps) = service
                .post(
                    &format!("/v1/conversations/{conversation}/messages"),
                    Some(&service.jeton(invite)),
                    json!({ "body": format!("Message {message} de la conversation {rang}") }),
                )
                .await;
            assert_eq!(statut, StatusCode::OK, "{corps}");
        }
    }

    let (statut, liste) = service
        .get("/v1/conversations", Some(&service.jeton("c_liste_hote")))
        .await;
    assert_eq!(statut, StatusCode::OK, "{liste}");
    let lignes = liste.as_array().expect("une liste");
    assert_eq!(
        lignes.len(),
        3,
        "les trois conversations doivent être là : {liste}"
    );

    for (rang, (conversation, _, titre)) in ouvertes.iter().enumerate() {
        let ligne = lignes
            .iter()
            .find(|c| c["id"] == conversation.as_str())
            .unwrap_or_else(|| panic!("conversation {rang} absente : {liste}"));

        assert_eq!(
            ligne["unread"],
            rang as i64 + 1,
            "non-lus de la conversation {rang} : {ligne}"
        );
        assert_eq!(
            ligne["lastMessage"],
            format!("Message {rang} de la conversation {rang}"),
            "dernier message de la conversation {rang} : {ligne}"
        );
        assert_eq!(
            ligne["planTitle"],
            titre.as_str(),
            "plan de la conversation {rang} : {ligne}"
        );
    }
}

/// Une conversation sans aucun message ne disparaît pas de la liste.
///
/// Le dernier message et les non-lus viennent de deux requêtes groupées : une
/// conversation absente de leur résultat doit rendre « aucun message » et zéro,
/// pas sortir de la liste.
#[tokio::test]
async fn une_conversation_sans_message_figure_dans_la_liste() {
    let service = Service::monter().await;
    let conversation = conversation_ouverte(&service, "c_muette_hote", "c_muette_invite").await;

    let (statut, liste) = service
        .get("/v1/conversations", Some(&service.jeton("c_muette_hote")))
        .await;
    assert_eq!(statut, StatusCode::OK, "{liste}");

    let ligne = liste
        .as_array()
        .expect("une liste")
        .iter()
        .find(|c| c["id"] == conversation.as_str())
        .unwrap_or_else(|| panic!("la conversation muette a disparu : {liste}"));
    assert_eq!(ligne["unread"], 0, "{ligne}");
    assert!(ligne["lastMessage"].is_null(), "{ligne}");
}
