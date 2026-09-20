//! La fiche et les critères du fil.
//!
//! Quatre routes que le portage n'avait pas : sans elles, un compte créé reste
//! en « onboarding » pour toujours — il ne peut ni publier ni demander.

use super::Service;
use axum::http::StatusCode;
use sea_orm::{ConnectionTrait, Statement};
use serde_json::json;

/// Lit une colonne d'une table, pour vérifier ce qui a réellement été écrit.
async fn colonne(service: &Service, sql: &str) -> String {
    service
        .db
        .query_one_raw(Statement::from_string(
            service.db.get_database_backend(),
            sql.to_string(),
        ))
        .await
        .unwrap()
        .map(|l| l.try_get::<String>("", "v").unwrap())
        .expect("une ligne")
}

#[tokio::test]
async fn deposer_sa_fiche_rend_le_compte_actif() {
    let service = Service::monter().await;
    let compte = service.compte("c_fiche_neuve", "depart").await;
    // Le compte créé par `compte_de_test` a déjà un profil ; on le remet en
    // onboarding pour dérouler le parcours réel.
    service
        .db
        .execute_unprepared(&format!(
            "UPDATE accounts SET status='onboarding' WHERE id='{compte}'"
        ))
        .await
        .unwrap();

    let (statut, corps) = service
        .put(
            "/v1/me/profile",
            Some(&service.jeton("c_fiche_neuve")),
            json!({
                "city": "Lyon",
                "latitude": 45.7578137,
                "longitude": 4.8320114,
                "gender": "autre",
                "bio": "On se croise sur les quais.",
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let statut_compte = colonne(
        &service,
        &format!("SELECT status AS v FROM accounts WHERE id='{compte}'"),
    )
    .await;
    assert_eq!(statut_compte, "active", "le compte est resté en onboarding");
}

/// Weave ne conserve jamais une position plus précise que le kilomètre, et
/// l'arrondi se fait au dépôt : ce qui n'est pas enregistré ne peut pas fuir.
#[tokio::test]
async fn la_position_est_arrondie_avant_d_etre_enregistree() {
    let service = Service::monter().await;
    let compte = service.compte("c_position", "depart").await;

    let (statut, _) = service
        .put(
            "/v1/me/profile",
            Some(&service.jeton("c_position")),
            json!({
                "city": "Lyon",
                "latitude": 45.7578137,
                "longitude": 4.8320114,
                "gender": "autre",
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);

    let position = colonne(
        &service,
        &format!(
            "SELECT CAST(latRounded AS TEXT) || ',' || CAST(lonRounded AS TEXT) AS v \
             FROM profiles WHERE accountId='{compte}'"
        ),
    )
    .await;
    assert_eq!(
        position, "45.76,4.83",
        "une position au dix-millième a été enregistrée"
    );
}

#[tokio::test]
async fn une_fiche_sans_ville_est_refusee() {
    let service = Service::monter().await;
    service.compte("c_sans_ville", "depart").await;

    let (statut, _) = service
        .put(
            "/v1/me/profile",
            Some(&service.jeton("c_sans_ville")),
            json!({
                "city": "   ",
                "latitude": 45.75,
                "longitude": 4.85,
                "gender": "autre",
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn une_phrase_trop_longue_est_refusee() {
    let service = Service::monter().await;
    service.compte("c_bavard", "depart").await;

    let (statut, _) = service
        .put(
            "/v1/me/profile",
            Some(&service.jeton("c_bavard")),
            json!({
                "city": "Lyon",
                "latitude": 45.75,
                "longitude": 4.85,
                "gender": "autre",
                "bio": "é".repeat(161),
            }),
        )
        .await;
    assert_eq!(
        statut,
        StatusCode::UNPROCESSABLE_ENTITY,
        "161 caractères sont passés"
    );
}

#[tokio::test]
async fn modifier_son_nom_affiche() {
    let service = Service::monter().await;
    service.compte("c_renomme", "depart").await;

    let (statut, _) = service
        .patch(
            "/v1/me",
            Some(&service.jeton("c_renomme")),
            json!({ "displayName": "Camille" }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (statut, corps) = service
        .get("/v1/me", Some(&service.jeton("c_renomme")))
        .await;
    assert_eq!(statut, StatusCode::OK);
    assert_eq!(
        corps["displayName"], "Camille",
        "le résumé en cache a survécu à la modification : {corps}"
    );
}

#[tokio::test]
async fn lire_ses_criteres() {
    let service = Service::monter().await;
    service.compte("c_criteres", "depart").await;

    let (statut, corps) = service
        .get("/v1/me/preferences", Some(&service.jeton("c_criteres")))
        .await;
    assert_eq!(statut, StatusCode::OK);
    assert_eq!(corps["minAge"], 18);
    assert_eq!(corps["maxDistanceKm"], 25);
    assert!(
        corps["seeking"].is_array(),
        "les listes sortent en tableaux : {corps}"
    );
    assert!(corps["categories"].is_array());
}

#[tokio::test]
async fn ajuster_ses_criteres() {
    let service = Service::monter().await;
    // « Escapade » vend les critères précis : c'est le palier qu'il faut
    // pour éprouver leur aller-retour.
    service.compte("c_ajuste", "escapade").await;

    let (statut, _) = service
        .patch(
            "/v1/me/preferences",
            Some(&service.jeton("c_ajuste")),
            json!({
                "minAge": 25,
                "maxAge": 45,
                "maxDistanceKm": 40,
                "categories": ["balade", "repas"],
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK);

    let (_, corps) = service
        .get("/v1/me/preferences", Some(&service.jeton("c_ajuste")))
        .await;
    assert_eq!(corps["minAge"], 25);
    assert_eq!(corps["maxAge"], 45);
    assert_eq!(corps["maxDistanceKm"], 40);
    assert_eq!(corps["categories"], json!(["balade", "repas"]));
}

#[tokio::test]
async fn un_age_minimum_au_dessus_du_maximum_est_refuse() {
    let service = Service::monter().await;
    service.compte("c_bornes", "depart").await;

    let (statut, _) = service
        .patch(
            "/v1/me/preferences",
            Some(&service.jeton("c_bornes")),
            json!({
                "minAge": 50, "maxAge": 30,
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);
}

/// Le rayon est ce qui borne la surveillance qu'un compte peut exercer : le
/// laisser dépasser cent kilomètres changerait la nature du produit.
#[tokio::test]
async fn un_rayon_hors_bornes_est_refuse() {
    let service = Service::monter().await;
    service.compte("c_rayon", "depart").await;

    for rayon in [0, 101, 5_000] {
        let (statut, _) = service
            .patch(
                "/v1/me/preferences",
                Some(&service.jeton("c_rayon")),
                json!({
                    "maxDistanceKm": rayon,
                }),
            )
            .await;
        assert_eq!(
            statut,
            StatusCode::UNPROCESSABLE_ENTITY,
            "rayon {rayon} accepté"
        );
    }
}

#[tokio::test]
async fn une_categorie_inconnue_est_refusee() {
    let service = Service::monter().await;
    service.compte("c_categorie", "depart").await;

    let (statut, _) = service
        .patch(
            "/v1/me/preferences",
            Some(&service.jeton("c_categorie")),
            json!({
                "categories": ["balade", "croisiere-en-yacht"],
            }),
        )
        .await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY);
}

/// L'envoi de photo et l'export sont bornés en débit.
///
/// Aucun des deux ne l'était. La photo écrit jusqu'à deux mégaoctets par
/// requête ; l'export est la lecture la plus lourde du service — tout le
/// compte, plans, demandes, conversations, messages, achats, et les octets de
/// la photo — et il est accessible à tout compte connecté.
///
/// La borne de l'export est volontairement large : le droit d'accès ne se
/// refuse pas. L'article 12 ne permet de s'opposer qu'aux demandes
/// « manifestement infondées ou excessives, notamment en raison de leur
/// caractère répétitif ».
#[tokio::test]
async fn l_export_et_l_envoi_de_photo_sont_bornes() {
    use crate::limitation::regles;

    let service = Service::monter().await;
    service.compte("c_debit", "depart").await;
    let jeton = service.jeton("c_debit");

    // La borne de l'export : les premiers passages répondent, celui d'après non.
    for tour in 0..regles::EXPORT.limite {
        let (statut, _) = service.get("/v1/me/export", Some(&jeton)).await;
        assert_eq!(
            statut,
            StatusCode::OK,
            "l'export a été refusé au tour {tour}"
        );
    }
    let (statut, corps) = service.get("/v1/me/export", Some(&jeton)).await;
    assert_eq!(
        statut,
        StatusCode::TOO_MANY_REQUESTS,
        "l'export n'est borné par rien : {corps}"
    );

    // Celle de la photo. La fiche d'abord : sans ville, la route refuse pour
    // une autre raison, et le test ne prouverait rien.
    let (statut, corps) = service
        .put(
            "/v1/me/profile",
            Some(&jeton),
            json!({ "city": "Lyon", "latitude": 45.76, "longitude": 4.84, "gender": "femme" }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let jpeg = [&[0xFF, 0xD8, 0xFF, 0xE0][..], &[0u8; 64][..]].concat();
    for tour in 0..regles::PHOTO.limite {
        let (statut, _) = service
            .put_octets("/v1/me/photo", &jeton, jpeg.clone())
            .await;
        assert_eq!(
            statut,
            StatusCode::OK,
            "la photo a été refusée au tour {tour}"
        );
    }
    let (statut, corps) = service.put_octets("/v1/me/photo", &jeton, jpeg).await;
    assert_eq!(
        statut,
        StatusCode::TOO_MANY_REQUESTS,
        "l'envoi de photo n'est borné par rien : {corps}"
    );
}

// ————————————————————————————————————————————————————————————————————————
// Retirer sa photo
//
// On pouvait la déposer et la remplacer, jamais la retirer. C'est la donnée
// la plus identifiante de la fiche, et la seule qu'on ne pouvait pas
// reprendre — tout le reste s'édite.
// ————————————————————————————————————————————————————————————————————————

/// Pose une fiche et une photo, et rend le jeton du compte.
async fn compte_avec_photo(service: &Service, nom: &str) -> String {
    let jeton = service.jeton(nom);
    let (statut, corps) = service
        .put(
            "/v1/me/profile",
            Some(&jeton),
            json!({ "city": "Lyon", "latitude": 45.76, "longitude": 4.84, "gender": "femme" }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let jpeg = [&[0xFF, 0xD8, 0xFF, 0xE0][..], &[0u8; 64][..]].concat();
    let (statut, corps) = service.put_octets("/v1/me/photo", &jeton, jpeg).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    jeton
}

async fn octets_en_base(service: &Service) -> u64 {
    use crate::entities::media_objects;
    use sea_orm::{EntityTrait, PaginatorTrait};
    media_objects::Entity::find()
        .count(&service.db)
        .await
        .expect("médias comptés")
}

#[tokio::test]
async fn retirer_sa_photo_efface_les_octets_et_pas_seulement_la_reference() {
    let service = Service::monter().await;
    service.compte("c_photo_off", "depart").await;
    let jeton = compte_avec_photo(&service, "c_photo_off").await;

    assert_eq!(octets_en_base(&service).await, 1);
    let (statut, corps) = service.get("/v1/me", Some(&jeton)).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert!(!corps["photoUrl"].is_null(), "la photo n'a pas été posée");

    let (statut, corps) = service.delete("/v1/me/photo", Some(&jeton)).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let (statut, corps) = service.get("/v1/me", Some(&jeton)).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert!(
        corps["photoUrl"].is_null(),
        "la fiche montre encore une photo"
    );

    // « Supprimer » ne peut pas vouloir dire « ne plus afficher ». Effacer la
    // seule référence garderait l'image en base, illisible mais présente — et
    // un octet conservé sans usage est ce que la politique s'interdit.
    assert_eq!(
        octets_en_base(&service).await,
        0,
        "les octets de la photo sont restés en base"
    );
}

#[tokio::test]
async fn retirer_une_photo_absente_ne_se_plaint_pas() {
    let service = Service::monter().await;
    service.compte("c_photo_vide", "depart").await;
    let jeton = service.jeton("c_photo_vide");
    let (statut, corps) = service
        .put(
            "/v1/me/profile",
            Some(&jeton),
            json!({ "city": "Lille", "latitude": 50.63, "longitude": 3.06, "gender": "homme" }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    // Une fiche sans photo est déjà dans l'état voulu. Rendre une erreur
    // ferait dépendre la réponse de ce qu'on ignorait — et l'appel vient d'un
    // écran qui ne sait pas toujours s'il y en avait une.
    for tour in 0..2 {
        let (statut, corps) = service.delete("/v1/me/photo", Some(&jeton)).await;
        assert_eq!(statut, StatusCode::OK, "tour {tour} : {corps}");
    }
}

#[tokio::test]
async fn on_ne_retire_que_sa_propre_photo() {
    let service = Service::monter().await;
    service.compte("c_photo_mienne", "depart").await;
    service.compte("c_photo_tiers", "depart").await;
    compte_avec_photo(&service, "c_photo_mienne").await;
    let tiers = compte_avec_photo(&service, "c_photo_tiers").await;

    assert_eq!(octets_en_base(&service).await, 2);
    let (statut, corps) = service.delete("/v1/me/photo", Some(&tiers)).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    // La route n'agit que sur la fiche du porteur du jeton. Une seule des deux
    // photos part : celle de qui l'a demandé.
    assert_eq!(
        octets_en_base(&service).await,
        1,
        "retirer sa photo a emporté celle de quelqu'un d'autre"
    );
    let (statut, corps) = service
        .get("/v1/me", Some(&service.jeton("c_photo_mienne")))
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert!(
        !corps["photoUrl"].is_null(),
        "la photo d'un tiers a disparu"
    );
}

#[tokio::test]
async fn une_photo_retiree_puis_redeposee_repart_de_zero() {
    let service = Service::monter().await;
    service.compte("c_photo_reprise", "depart").await;
    let jeton = compte_avec_photo(&service, "c_photo_reprise").await;

    let (statut, _) = service.delete("/v1/me/photo", Some(&jeton)).await;
    assert_eq!(statut, StatusCode::OK);

    // Rien n'empêche de revenir sur sa décision, et la nouvelle photo n'hérite
    // pas de la relecture de modération de l'ancienne : ce n'est pas la même
    // image, et reconduire un « déjà vu » mentirait sur ce qui a été vu.
    let jpeg = [&[0xFF, 0xD8, 0xFF, 0xE0][..], &[1u8; 64][..]].concat();
    let (statut, corps) = service.put_octets("/v1/me/photo", &jeton, jpeg).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(octets_en_base(&service).await, 1);
}
