//! Des transactions StoreKit signées pour de vrai, dans les tests.
//!
//! Les tests de facturation fabriquaient leurs transactions en encodant un
//! JSON en base64 — ce qui suffisait tant que la vérification n'existait pas.
//! Elle existe, et cette commodité serait devenue un trou : des tests qui
//! passent parce qu'ils contournent ce qu'ils sont censés éprouver.
//!
//! Ici, une autorité de test signe une vraie chaîne, et la charge utile porte
//! une vraie signature ES256. Le service épingle cette racine plutôt que celle
//! d'Apple ; tout le reste du chemin est celui de la production.

use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use p256::ecdsa::{Signature, SigningKey, signature::Signer};
use p256::pkcs8::DecodePrivateKey;
use rcgen::{
    BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair, KeyUsagePurpose,
    PKCS_ECDSA_P256_SHA256,
};
use serde_json::Value;
use std::sync::OnceLock;

/// L'autorité de test, fabriquée une fois pour toutes.
///
/// Une par exécution : générer trois paires de clés ECDSA par test coûterait
/// plus cher que tout le reste de la suite réunie.
struct Autorite {
    racine_der: Vec<u8>,
    intermediaire_der: Vec<u8>,
    feuille_der: Vec<u8>,
    cle_feuille: SigningKey,
}

fn autorite() -> &'static Autorite {
    static AUTORITE: OnceLock<Autorite> = OnceLock::new();
    AUTORITE.get_or_init(|| {
        let params_ca = |nom: &str| {
            let mut params = CertificateParams::new(vec![]).expect("paramètres");
            params.distinguished_name.push(DnType::CommonName, nom);
            params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
            params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
            params
        };

        let cle_racine = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).expect("clé racine");
        let params_racine = params_ca("Racine StoreKit de test");
        let racine = params_racine.self_signed(&cle_racine).expect("racine");
        let emetteur_racine = Issuer::new(params_racine, cle_racine);

        let cle_inter = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).expect("clé intermédiaire");
        let params_inter = params_ca("Intermediaire StoreKit de test");
        let intermediaire = params_inter
            .signed_by(&cle_inter, &emetteur_racine)
            .expect("intermédiaire");
        let emetteur_inter = Issuer::new(params_inter, cle_inter);

        let cle_feuille = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).expect("clé feuille");
        let mut params_feuille = CertificateParams::new(vec![]).expect("paramètres");
        params_feuille
            .distinguished_name
            .push(DnType::CommonName, "Signataire StoreKit de test");
        let feuille = params_feuille
            .signed_by(&cle_feuille, &emetteur_inter)
            .expect("feuille");

        Autorite {
            racine_der: racine.der().to_vec(),
            intermediaire_der: intermediaire.der().to_vec(),
            feuille_der: feuille.der().to_vec(),
            cle_feuille: SigningKey::from_pkcs8_der(cle_feuille.serialized_der())
                .expect("clé de signature"),
        }
    })
}

/// La racine que le service doit épingler dans les tests.
pub fn racine_de_test() -> Vec<u8> {
    autorite().racine_der.clone()
}

/// Signe une transaction comme Apple le ferait.
///
/// `claims` reçoit les champs propres au test ; `bundleId`, `environment` et
/// `signedDate` sont posés ici parce que la route les contrôle et qu'un test
/// qui les oublierait échouerait pour une raison sans rapport avec son objet.
pub fn transaction_signee(mut claims: Value) -> String {
    let objet = claims.as_object_mut().expect("un objet JSON");
    objet
        .entry("bundleId")
        .or_insert_with(|| Value::String("com.weave.app".to_string()));
    objet
        .entry("environment")
        .or_insert_with(|| Value::String("sandbox".to_string()));
    objet
        .entry("signedDate")
        .or_insert_with(|| Value::Number(chrono::Utc::now().timestamp_millis().into()));

    let ca = autorite();
    let entete = serde_json::json!({
        "alg": "ES256",
        "x5c": [
            STANDARD.encode(&ca.feuille_der),
            STANDARD.encode(&ca.intermediaire_der),
            STANDARD.encode(&ca.racine_der),
        ],
    });

    let signe = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(entete.to_string()),
        URL_SAFE_NO_PAD.encode(claims.to_string())
    );
    let signature: Signature = ca.cle_feuille.sign(signe.as_bytes());
    format!("{signe}.{}", URL_SAFE_NO_PAD.encode(signature.to_bytes()))
}
