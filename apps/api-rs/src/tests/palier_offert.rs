//! Le palier offert à l'équipe par `COMPTES_GRANDTOUR`.

use super::Service;
use crate::env::liste_d_adresses;
use axum::http::StatusCode;

#[test]
fn la_liste_se_lit_comme_les_adresses_a_la_connexion() {
    assert_eq!(
        liste_d_adresses(" Max@iCloud.com, lea@exemple.fr;  ;pas-une-adresse "),
        vec!["max@icloud.com".to_string(), "lea@exemple.fr".to_string()]
    );
}

#[tokio::test]
async fn une_adresse_de_la_liste_a_le_palier_le_plus_haut_sans_abonnement() {
    let service = Service::monter_configure(|config, suffixe| {
        config.auth.comptes_offerts = vec![format!("alice_{suffixe}@exemple.fr")];
    })
    .await;
    service.compte("alice", "depart").await;
    service.compte("bob", "depart").await;

    let (statut, alice) = service.get("/v1/me", Some(&service.jeton("alice"))).await;
    assert_eq!(statut, StatusCode::OK, "{alice}");
    assert_eq!(alice["tier"], "grandtour", "{alice}");

    // Les autres gardent le leur.
    let (_, bob) = service.get("/v1/me", Some(&service.jeton("bob"))).await;
    assert_eq!(bob["tier"], "depart", "{bob}");
}
