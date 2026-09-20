//! Changer son adresse e-mail.
//!
//! L'adresse EST le compte : il n'y a ni mot de passe, ni question de secours.
//! Rien ne permettait d'en changer, si bien que perdre l'accès à sa boîte
//! rendait le compte définitivement injoignable — avec ses conversations, son
//! abonnement et ses achats. On ne pouvait même plus le supprimer, puisque
//! supprimer demande d'être connecté.

use super::Service;
use axum::http::StatusCode;
use sea_orm::ConnectionTrait;
use serde_json::json;

/// Demande un code pour une nouvelle adresse, et le rend.
async fn demander(service: &Service, compte: &str, adresse: &str) -> String {
    let (statut, corps) = service
        .post(
            "/v1/me/email",
            Some(&service.jeton(compte)),
            json!({ "email": adresse }),
        )
        .await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    corps["devCode"]
        .as_str()
        .expect("le code de développement est rendu hors production")
        .to_string()
}

async fn confirmer(
    service: &Service,
    compte: &str,
    adresse: &str,
    code: &str,
) -> (StatusCode, serde_json::Value) {
    service
        .post(
            "/v1/me/email/verify",
            Some(&service.jeton(compte)),
            json!({ "email": adresse, "code": code }),
        )
        .await
}

async fn adresse_en_base(service: &Service, compte: &str) -> String {
    let ligne = service
        .db
        .query_one_raw(sea_orm::Statement::from_string(
            service.db.get_database_backend(),
            format!("SELECT email FROM accounts WHERE id = '{compte}'"),
        ))
        .await
        .expect("compte lu")
        .expect("compte présent");
    ligne
        .try_get::<String>("", "email")
        .expect("adresse lisible")
}

#[tokio::test]
async fn changer_d_adresse_avec_le_code_recu() {
    let service = Service::monter().await;
    let compte = service.compte("c_adresse", "depart").await;
    let avant = adresse_en_base(&service, &compte).await;

    let nouvelle = service.email("nouvelle");
    let code = demander(&service, "c_adresse", &nouvelle).await;
    let (statut, corps) = confirmer(&service, "c_adresse", &nouvelle, &code).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let apres = adresse_en_base(&service, &compte).await;
    assert_ne!(apres, avant, "l'adresse n'a pas changé");
    assert_eq!(apres, nouvelle.to_lowercase());
}

#[tokio::test]
async fn l_adresse_est_normalisee_comme_a_la_connexion() {
    let service = Service::monter().await;
    let compte = service.compte("c_adresse_norm", "depart").await;

    // Espaces et majuscules : la connexion les ramène à la même adresse, et
    // ce changement doit écrire ce que la connexion cherchera. Une adresse
    // enregistrée en « Max@Exemple.FR » ne se retrouverait plus.
    let brute = format!("  {}  ", service.email("MiXtE").to_uppercase());
    let code = demander(&service, "c_adresse_norm", &brute).await;
    let (statut, corps) = confirmer(&service, "c_adresse_norm", &brute, &code).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let ecrite = adresse_en_base(&service, &compte).await;
    assert_eq!(
        ecrite,
        ecrite.trim().to_lowercase(),
        "adresse non normalisée"
    );
    assert!(!ecrite.contains(' '));
}

#[tokio::test]
async fn un_code_faux_ne_change_rien() {
    let service = Service::monter().await;
    let compte = service.compte("c_adresse_faux", "depart").await;
    let avant = adresse_en_base(&service, &compte).await;

    let nouvelle = service.email("refusee");
    demander(&service, "c_adresse_faux", &nouvelle).await;
    let (statut, corps) = confirmer(&service, "c_adresse_faux", &nouvelle, "000000").await;
    assert_eq!(statut, StatusCode::UNAUTHORIZED, "{corps}");
    assert_eq!(adresse_en_base(&service, &compte).await, avant);
}
#[tokio::test]
async fn un_code_consomme_est_mort_pour_tout_le_monde() {
    let service = Service::monter().await;
    service.compte("c_adresse_rejeu", "depart").await;
    let second = service.compte("c_adresse_second", "depart").await;
    let avant_second = adresse_en_base(&service, &second).await;

    let convoitee = service.email("rejeu");
    let code = demander(&service, "c_adresse_rejeu", &convoitee).await;
    let (statut, corps) = confirmer(&service, "c_adresse_rejeu", &convoitee, &code).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    // Le premier compte LIBÈRE l'adresse en s'en allant ailleurs.
    //
    // Sans cela, ce test ne prouve rien : l'adresse appartiendrait au premier
    // compte, et le refus opposé au second viendrait de l'unicité — pas de la
    // consommation du code. Ma première version s'arrêtait là, et retirer la
    // consommation ne la faisait pas tomber. C'est la mutation qui me l'a
    // appris, et c'est exactement le genre de trou qu'un test de sécurité ne
    // peut pas se permettre.
    let ailleurs = service.email("ailleurs");
    let code_ailleurs = demander(&service, "c_adresse_rejeu", &ailleurs).await;
    let (statut, _) = confirmer(&service, "c_adresse_rejeu", &ailleurs, &code_ailleurs).await;
    assert_eq!(statut, StatusCode::OK);

    // L'adresse est libre, et le premier code a été consommé. Le second compte
    // ne peut donc l'obtenir qu'en demandant le sien.
    let (rejeu, corps) = confirmer(&service, "c_adresse_second", &convoitee, &code).await;
    assert_ne!(
        rejeu,
        StatusCode::OK,
        "un code déjà consommé a resservi sur une adresse libre : {corps}"
    );
    assert_eq!(adresse_en_base(&service, &second).await, avant_second);
}

#[tokio::test]
async fn un_code_perime_ne_change_rien() {
    let service = Service::monter().await;
    let compte = service.compte("c_adresse_perime", "depart").await;
    let avant = adresse_en_base(&service, &compte).await;

    let nouvelle = service.email("perimee");
    let code = demander(&service, "c_adresse_perime", &nouvelle).await;

    // Le défi est vieilli en base plutôt qu'attendu : c'est la DATE qu'on
    // éprouve, et rien d'autre ne le fera tomber — ni la consommation, ni le
    // plafond de tentatives, ni l'unicité, puisque l'adresse est libre.
    service
        .db
        .execute_unprepared("UPDATE otp_challenges SET expiresAt = '2020-01-01 00:00:00'")
        .await
        .expect("défi vieilli");

    let (statut, corps) = confirmer(&service, "c_adresse_perime", &nouvelle, &code).await;
    assert_eq!(statut, StatusCode::UNAUTHORIZED, "{corps}");
    assert_eq!(adresse_en_base(&service, &compte).await, avant);
}

#[tokio::test]
async fn sans_le_code_rien_ne_bouge() {
    let service = Service::monter().await;
    let compte = service.compte("c_adresse_sans_code", "depart").await;
    let avant = adresse_en_base(&service, &compte).await;
    let nouvelle = service.email("jamais-demandee");

    // Aucune demande n'a été faite pour cette adresse : il n'existe pas de
    // défi, et confirmer ne peut pas aboutir. C'est ce qui empêche de déplacer
    // son compte vers une boîte qu'on ne contrôle pas — le cas que toute cette
    // route existe pour rendre possible, et sûr.
    let (statut, corps) = confirmer(&service, "c_adresse_sans_code", &nouvelle, "123456").await;
    assert_eq!(statut, StatusCode::UNAUTHORIZED, "{corps}");
    assert_eq!(adresse_en_base(&service, &compte).await, avant);
}

#[tokio::test]
async fn une_adresse_deja_prise_est_refusee_a_la_confirmation() {
    let service = Service::monter().await;
    let occupant = service.compte("c_adresse_occupee", "depart").await;
    let compte = service.compte("c_adresse_tentant", "depart").await;

    let prise = adresse_en_base(&service, &occupant).await;
    let avant = adresse_en_base(&service, &compte).await;

    // La DEMANDE aboutit : refuser ici dirait, pour n'importe quelle adresse,
    // si elle a un compte sur Weave. Sur une application de rencontre, c'est
    // une information qu'on ne donne pas.
    let code = demander(&service, "c_adresse_tentant", &prise).await;

    // Le refus vient à la confirmation, une fois le contrôle de la boîte
    // prouvé : l'apprendre alors ne renseigne que celui qui possède l'adresse.
    let (statut, corps) = confirmer(&service, "c_adresse_tentant", &prise, &code).await;
    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY, "{corps}");
    assert_eq!(adresse_en_base(&service, &compte).await, avant);
    assert_eq!(
        adresse_en_base(&service, &occupant).await,
        prise,
        "le compte occupant a perdu son adresse"
    );
}

#[tokio::test]
async fn reprendre_sa_propre_adresse_ne_casse_rien() {
    let service = Service::monter().await;
    let compte = service.compte("c_adresse_meme", "depart").await;
    let sienne = adresse_en_base(&service, &compte).await;

    // Ni une erreur, ni un changement. Refuser obligerait l'écran à connaître
    // l'adresse courante pour savoir s'il a le droit de demander.
    let (statut, corps) = confirmer(&service, "c_adresse_meme", &sienne, "peu-importe").await;
    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(adresse_en_base(&service, &compte).await, sienne);
}

#[tokio::test]
async fn une_adresse_sans_arobase_est_refusee() {
    let service = Service::monter().await;
    service.compte("c_adresse_invalide", "depart").await;

    for mauvaise in ["pas-une-adresse", ""] {
        let (statut, corps) = service
            .post(
                "/v1/me/email",
                Some(&service.jeton("c_adresse_invalide")),
                json!({ "email": mauvaise }),
            )
            .await;
        assert_eq!(
            statut,
            StatusCode::UNPROCESSABLE_ENTITY,
            "« {mauvaise} » a été acceptée : {corps}"
        );
    }
}

#[tokio::test]
async fn le_changement_demande_une_session() {
    let service = Service::monter().await;
    let (statut, corps) = service
        .post("/v1/me/email", None, json!({ "email": "qui@exemple.fr" }))
        .await;
    assert_eq!(statut, StatusCode::UNAUTHORIZED, "{corps}");
}

#[tokio::test]
async fn les_tentatives_sur_un_code_sont_plafonnees() {
    let service = Service::monter().await;
    let compte = service.compte("c_adresse_force", "depart").await;
    let avant = adresse_en_base(&service, &compte).await;

    let nouvelle = service.email("forcee");
    let code = demander(&service, "c_adresse_force", &nouvelle).await;

    // Un code à six chiffres ne résiste pas à un nombre libre d'essais. Le
    // plafond est par DÉFI, et non par adresse : c'est lui qui porte
    // l'invariant, pas la limitation de débit.
    let mut refus = 0;
    for _ in 0..8 {
        let (statut, _) = confirmer(&service, "c_adresse_force", &nouvelle, "000000").await;
        if statut == StatusCode::UNAUTHORIZED {
            refus += 1;
        }
    }
    assert!(refus > 0);
    assert_eq!(adresse_en_base(&service, &compte).await, avant);

    // Et le bon code ne passe plus : le défi est épuisé.
    let (statut, corps) = confirmer(&service, "c_adresse_force", &nouvelle, &code).await;
    assert_ne!(
        statut,
        StatusCode::OK,
        "le plafond de tentatives n'a rien retenu : {corps}"
    );
}
