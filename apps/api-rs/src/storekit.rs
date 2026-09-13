//! Vérification cryptographique des transactions StoreKit 2.
//!
//! ## Ce qui tenait lieu de vérification
//!
//! La route de facturation décodait le base64 de la charge utile et lisait les
//! champs qu'elle y trouvait. Décoder n'est pas vérifier : n'importe qui
//! pouvait encoder un JSON annonçant le produit de son choix. Un garde-fou
//! nommé `VERIFICATION_JWS_IMPLEMENTEE` refusait donc tout achat en
//! production, et c'est ce module qui lui donne de quoi passer à `true`.
//!
//! ## Ce qu'Apple signe, et comment
//!
//! Une transaction StoreKit 2 est un JWS : `en-tête.charge.signature`, signé
//! en ES256. L'en-tête porte `x5c`, la chaîne de certificats qui mène du
//! certificat ayant signé jusqu'à la racine d'Apple.
//!
//! Vérifier, c'est donc quatre choses, et il faut les quatre :
//!
//!   1. la chaîne se tient — chaque certificat est signé par le suivant ;
//!   2. elle **se termine à la racine épinglée**, et pas à une racine
//!      quelconque. C'est le point unique sur lequel tout repose : une chaîne
//!      qu'on fabrique soi-même est parfaitement valide, elle ne mène
//!      simplement pas chez Apple ;
//!   3. aucun certificat n'est périmé ;
//!   4. la signature du JWS est celle de la clé du certificat de tête.
//!
//! ## La racine est injectée, pas codée en dur ici
//!
//! La production épingle `AppleRootCA-G3.cer`, joint au binaire. Les tests
//! épinglent la leur, ce qui permet d'éprouver le chemin complet avec de
//! vraies signatures — et surtout d'éprouver le **rejet** d'une chaîne qui ne
//! mène pas à la racine attendue, qui est la seule propriété dont dépend tout
//! le reste.

use base64::{engine::general_purpose::STANDARD, Engine};
use p256::ecdsa::{signature::Verifier, Signature, VerifyingKey};
use serde_json::Value;
use x509_parser::prelude::*;

/// La racine d'Apple, jointe au binaire.
///
/// Récupérée sur `https://www.apple.com/certificateauthority/`. Son empreinte
/// SHA-256 est celle qu'Apple publie :
/// `63:34:3A:BF:B8:9A:6A:03:EB:B5:7E:9B:3F:5F:A7:BE:7C:4F:5C:75:6F:30:17:B3:A8:C4:88:C3:65:3E:91:79`
pub const RACINE_APPLE: &[u8] = include_bytes!("AppleRootCA-G3.cer");

/// Pourquoi une transaction est refusée.
///
/// Chaque cas est distinct pour que le journal dise ce qui a échoué. Aucun
/// n'est rendu tel quel à l'appelant : on ne renseigne pas qui essaie de
/// forger sur ce qui l'a trahi.
#[derive(Debug, PartialEq, Eq)]
pub enum Refus {
    /// Le jeton n'a pas trois parties, ou elles ne se décodent pas.
    Malforme,
    /// L'en-tête n'annonce pas ES256.
    AlgorithmeInattendu,
    /// Pas de chaîne de certificats, ou trop courte pour mener quelque part.
    ChaineAbsente,
    /// Un certificat ne se lit pas.
    CertificatIllisible,
    /// Un maillon n'est pas signé par le suivant.
    ChaineRompue,
    /// La chaîne se tient, mais ne mène pas à la racine épinglée.
    RacineInconnue,
    /// Un certificat de la chaîne est périmé, ou pas encore valide.
    CertificatPerime,
    /// La signature ne correspond pas à la clé du certificat de tête.
    SignatureInvalide,
    /// La charge utile n'est pas un objet JSON.
    ChargeIllisible,
}

impl Refus {
    /// Ce qu'on inscrit au journal.
    pub fn motif(&self) -> &'static str {
        match self {
            Refus::Malforme => "jeton malformé",
            Refus::AlgorithmeInattendu => "algorithme inattendu",
            Refus::ChaineAbsente => "chaîne de certificats absente",
            Refus::CertificatIllisible => "certificat illisible",
            Refus::ChaineRompue => "chaîne rompue",
            Refus::RacineInconnue => "racine inconnue",
            Refus::CertificatPerime => "certificat périmé",
            Refus::SignatureInvalide => "signature invalide",
            Refus::ChargeIllisible => "charge illisible",
        }
    }
}

/// Vérifie un JWS signé par Apple et rend sa charge utile.
///
/// `racine` est le certificat racine attendu, en DER. `maintenant` est
/// l'instant qui sert à juger les périodes de validité — passé en paramètre
/// pour que les tests puissent se placer avant ou après.
pub fn verifier(
    jws: &str,
    racine: &[u8],
    maintenant: chrono::DateTime<chrono::Utc>,
) -> Result<Value, Refus> {
    let mut parties = jws.split('.');
    let (Some(entete_b64), Some(charge_b64), Some(signature_b64), None) = (
        parties.next(),
        parties.next(),
        parties.next(),
        parties.next(),
    ) else {
        return Err(Refus::Malforme);
    };

    let entete: Value = serde_json::from_slice(&url_safe(entete_b64)?).map_err(|_| Refus::Malforme)?;

    // ES256 et rien d'autre. Accepter « none » — ou laisser l'en-tête choisir
    // l'algorithme — est la faille classique des vérificateurs de JWS : celui
    // qui signe choisirait alors comment on le vérifie.
    if entete.get("alg").and_then(Value::as_str) != Some("ES256") {
        return Err(Refus::AlgorithmeInattendu);
    }

    // `x5c` est en base64 STANDARD, pas en base64 URL : c'est la convention de
    // la RFC 7515 pour ce champ, et elle diffère de celle des trois parties du
    // jeton lui-même.
    let chaine_b64 = entete
        .get("x5c")
        .and_then(Value::as_array)
        .ok_or(Refus::ChaineAbsente)?;
    if chaine_b64.len() < 2 {
        return Err(Refus::ChaineAbsente);
    }

    let mut chaine_der = Vec::with_capacity(chaine_b64.len());
    for element in chaine_b64 {
        let brut = element.as_str().ok_or(Refus::CertificatIllisible)?;
        chaine_der.push(STANDARD.decode(brut).map_err(|_| Refus::CertificatIllisible)?);
    }

    // La racine attendue doit être le dernier maillon, et exactement lui.
    // Comparer les octets plutôt que le sujet : un certificat qu'on fabrique
    // peut porter le nom d'Apple, il ne peut pas porter ses octets.
    if chaine_der.last().map(Vec::as_slice) != Some(racine) {
        return Err(Refus::RacineInconnue);
    }

    let mut certificats = Vec::with_capacity(chaine_der.len());
    for der in &chaine_der {
        let (reste, certificat) =
            X509Certificate::from_der(der).map_err(|_| Refus::CertificatIllisible)?;
        if !reste.is_empty() {
            return Err(Refus::CertificatIllisible);
        }
        certificats.push(certificat);
    }

    let instant = ASN1Time::from_timestamp(maintenant.timestamp())
        .map_err(|_| Refus::CertificatPerime)?;
    for certificat in &certificats {
        if !certificat.validity().is_valid_at(instant) {
            return Err(Refus::CertificatPerime);
        }
    }

    // Chaque maillon est signé par le suivant, jusqu'à la racine.
    for paire in certificats.windows(2) {
        // Celui qui signe doit avoir le droit de signer des certificats. La
        // racine épinglée rend déjà la forgerie impossible sans la clé
        // d'Apple, mais un émetteur qui n'est pas une autorité n'a rien à
        // faire au milieu d'une chaîne : le vérifier coûte une ligne.
        let est_autorite = paire[1]
            .basic_constraints()
            .ok()
            .flatten()
            .is_some_and(|extension| extension.value.ca);
        if !est_autorite {
            return Err(Refus::ChaineRompue);
        }

        paire[0]
            .verify_signature(Some(paire[1].public_key()))
            .map_err(|_| Refus::ChaineRompue)?;
    }

    // La signature du jeton, par la clé du certificat de tête.
    let cle = VerifyingKey::from_sec1_bytes(certificats[0].public_key().subject_public_key.as_ref())
        .map_err(|_| Refus::SignatureInvalide)?;
    let signature = Signature::from_slice(&url_safe(signature_b64)?)
        .map_err(|_| Refus::SignatureInvalide)?;
    let signe = format!("{entete_b64}.{charge_b64}");
    cle.verify(signe.as_bytes(), &signature)
        .map_err(|_| Refus::SignatureInvalide)?;

    let charge: Value =
        serde_json::from_slice(&url_safe(charge_b64)?).map_err(|_| Refus::ChargeIllisible)?;
    if !charge.is_object() {
        return Err(Refus::ChargeIllisible);
    }
    Ok(charge)
}

fn url_safe(brut: &str) -> Result<Vec<u8>, Refus> {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    URL_SAFE_NO_PAD.decode(brut).map_err(|_| Refus::Malforme)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use chrono::Utc;
    use p256::ecdsa::{signature::Signer, SigningKey};
    use p256::pkcs8::DecodePrivateKey;

    /// Une chaîne fabriquée pour les tests : racine, intermédiaire, feuille.
    ///
    /// De vrais certificats et de vraies signatures ECDSA — c'est ce qui
    /// permet d'éprouver le chemin complet. Ce qu'ils n'ont pas, c'est la
    /// racine d'Apple, et c'est précisément ce qu'on veut pouvoir refuser.
    struct Chaine {
        racine_der: Vec<u8>,
        intermediaire_der: Vec<u8>,
        feuille_der: Vec<u8>,
        cle_feuille: SigningKey,
    }

    /// Un instant à N jours d'ici, au format qu'attend rcgen.
    ///
    /// rcgen date ses certificats de 1975 à 4096 par défaut. Le test de
    /// péremption passait donc au vert sans jamais éprouver la vérification
    /// des dates : même cinquante ans plus tard, le certificat restait valide.
    /// Le chemin est absolu : `x509_parser::prelude::*` amène son propre
    /// `time`, et la résolution allait chercher là plutôt que la bibliothèque.
    fn date_decalee(jours: i64) -> ::time::OffsetDateTime {
        ::time::OffsetDateTime::now_utc() + ::time::Duration::days(jours)
    }

    fn fabriquer_chaine() -> Chaine {
        use rcgen::{
            BasicConstraints, CertificateParams, Issuer, IsCa, KeyPair, KeyUsagePurpose,
            PKCS_ECDSA_P256_SHA256,
        };

        let autorite = |nom: &str| {
            let mut params = CertificateParams::new(vec![]).expect("paramètres");
            params.distinguished_name.push(rcgen::DnType::CommonName, nom);
            params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
            params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
            params
        };

        let cle_racine = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).expect("clé racine");
        let params_racine = autorite("Racine de test");
        let racine = params_racine.self_signed(&cle_racine).expect("racine signée");
        let emetteur_racine = Issuer::new(params_racine, cle_racine);

        let cle_inter = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).expect("clé intermédiaire");
        let params_inter = autorite("Intermediaire de test");
        let intermediaire = params_inter
            .signed_by(&cle_inter, &emetteur_racine)
            .expect("intermédiaire signé");
        let emetteur_inter = Issuer::new(params_inter, cle_inter);

        let cle_feuille = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).expect("clé feuille");
        let mut params_feuille = CertificateParams::new(vec![]).expect("paramètres");
        params_feuille
            .distinguished_name
            .push(rcgen::DnType::CommonName, "Signataire de test");
        // Une validité courte et explicite. Sans cela, rcgen date ses
        // certificats si loin que le test de péremption ne prouvait rien :
        // il passait au vert sans jamais éprouver la vérification des dates.
        params_feuille.not_before = date_decalee(-1);
        params_feuille.not_after = date_decalee(1);
        let feuille = params_feuille
            .signed_by(&cle_feuille, &emetteur_inter)
            .expect("feuille signée");

        // La clé de signature du JWS est celle de la feuille : c'est son
        // certificat qui figure en tête de la chaîne.
        let cle_feuille = SigningKey::from_pkcs8_der(cle_feuille.serialized_der())
            .expect("clé de signature lisible");

        Chaine {
            racine_der: racine.der().to_vec(),
            intermediaire_der: intermediaire.der().to_vec(),
            feuille_der: feuille.der().to_vec(),
            cle_feuille,
        }
    }

    /// Forge un JWS avec la chaîne fournie.
    fn signer(chaine: &Chaine, charge: Value, alg: &str) -> String {
        let entete = serde_json::json!({
            "alg": alg,
            "x5c": [
                STANDARD.encode(&chaine.feuille_der),
                STANDARD.encode(&chaine.intermediaire_der),
                STANDARD.encode(&chaine.racine_der),
            ],
        });
        let entete_b64 = URL_SAFE_NO_PAD.encode(entete.to_string());
        let charge_b64 = URL_SAFE_NO_PAD.encode(charge.to_string());
        let signe = format!("{entete_b64}.{charge_b64}");
        let signature: Signature = chaine.cle_feuille.sign(signe.as_bytes());
        format!("{signe}.{}", URL_SAFE_NO_PAD.encode(signature.to_bytes()))
    }

    /// LE test : une chaîne valide qui ne mène pas à la racine attendue.
    ///
    /// Tout repose là-dessus. Les certificats sont vrais, les signatures
    /// vraies, la chaîne se tient — elle mène simplement ailleurs que chez
    /// Apple. C'est exactement ce que produirait quelqu'un qui veut s'offrir
    /// l'abonnement le plus cher, et c'est le seul rempart.
    #[test]
    fn une_chaine_qui_ne_mene_pas_a_la_racine_epinglee_est_refusee() {
        let chaine = fabriquer_chaine();
        let jws = signer(&chaine, serde_json::json!({ "productId": "grandtour" }), "ES256");

        assert_eq!(
            verifier(&jws, RACINE_APPLE, Utc::now()),
            Err(Refus::RacineInconnue),
            "une chaîne fabriquée a été acceptée contre la racine d'Apple"
        );
    }

    /// Et la même chaîne passe contre SA racine : la vérification fonctionne.
    #[test]
    fn une_chaine_complete_est_acceptee_contre_sa_propre_racine() {
        let chaine = fabriquer_chaine();
        let charge = serde_json::json!({ "productId": "viree", "transactionId": "42" });
        let jws = signer(&chaine, charge.clone(), "ES256");

        assert_eq!(
            verifier(&jws, &chaine.racine_der, Utc::now()),
            Ok(charge),
            "le chemin complet devrait passer"
        );
    }

    /// Une charge modifiée après signature ne passe plus.
    #[test]
    fn une_charge_retouchee_est_refusee() {
        let chaine = fabriquer_chaine();
        let jws = signer(&chaine, serde_json::json!({ "productId": "viree" }), "ES256");

        let mut parties: Vec<&str> = jws.split('.').collect();
        let retouchee = URL_SAFE_NO_PAD.encode(
            serde_json::json!({ "productId": "grandtour" }).to_string(),
        );
        parties[1] = &retouchee;

        assert_eq!(
            verifier(&parties.join("."), &chaine.racine_der, Utc::now()),
            Err(Refus::SignatureInvalide),
            "une charge réécrite après signature a été acceptée"
        );
    }

    /// « alg » ne décide pas de la vérification.
    ///
    /// Laisser l'en-tête choisir l'algorithme est la faille classique des
    /// vérificateurs de JWS : celui qui signe choisirait alors comment on le
    /// vérifie, et « none » lui suffirait.
    #[test]
    fn un_algorithme_annonce_autrement_est_refuse() {
        let chaine = fabriquer_chaine();
        for alg in ["none", "HS256", "RS256", "ES384"] {
            let jws = signer(&chaine, serde_json::json!({ "productId": "viree" }), alg);
            assert_eq!(
                verifier(&jws, &chaine.racine_der, Utc::now()),
                Err(Refus::AlgorithmeInattendu),
                "« {alg} » a été accepté"
            );
        }
    }

    /// Un certificat périmé ne signe plus rien.
    #[test]
    fn une_chaine_perimee_est_refusee() {
        let chaine = fabriquer_chaine();
        let jws = signer(&chaine, serde_json::json!({ "productId": "viree" }), "ES256");

        // La feuille vaut un jour de part et d'autre : deux jours plus tard,
        // elle est périmée.
        let apres = Utc::now() + chrono::Duration::days(2);
        assert_eq!(
            verifier(&jws, &chaine.racine_der, apres),
            Err(Refus::CertificatPerime),
            "une chaîne périmée a été acceptée"
        );

        // Et avant son entrée en vigueur, elle ne vaut pas davantage.
        let avant = Utc::now() - chrono::Duration::days(2);
        assert_eq!(
            verifier(&jws, &chaine.racine_der, avant),
            Err(Refus::CertificatPerime),
            "une chaîne pas encore valide a été acceptée"
        );
    }

    /// Sans chaîne, il n'y a rien à quoi se fier.
    #[test]
    fn un_jeton_sans_chaine_est_refuse() {
        let entete = URL_SAFE_NO_PAD.encode(serde_json::json!({ "alg": "ES256" }).to_string());
        let charge = URL_SAFE_NO_PAD.encode(serde_json::json!({ "productId": "x" }).to_string());
        let jws = format!("{entete}.{charge}.{}", URL_SAFE_NO_PAD.encode([0u8; 64]));

        assert_eq!(
            verifier(&jws, RACINE_APPLE, Utc::now()),
            Err(Refus::ChaineAbsente)
        );
    }

    /// Ce qui n'a pas la forme d'un jeton est refusé avant tout le reste.
    #[test]
    fn un_jeton_malforme_est_refuse() {
        for brut in ["", "a", "a.b", "a.b.c.d", "pas du tout un jeton"] {
            assert!(
                verifier(brut, RACINE_APPLE, Utc::now()).is_err(),
                "« {brut} » a été accepté"
            );
        }
    }

    /// La racine jointe au binaire est bien celle d'Apple.
    ///
    /// Son empreinte est publique : si le fichier était remplacé, ce test
    /// tomberait avant que la production ne fasse confiance à autre chose.
    #[test]
    fn la_racine_jointe_est_celle_d_apple() {
        use sha2::{Digest, Sha256};
        let empreinte = hex::encode(Sha256::digest(RACINE_APPLE));
        assert_eq!(
            empreinte,
            "63343abfb89a6a03ebb57e9b3f5fa7be7c4f5c756f3017b3a8c488c3653e9179",
            "la racine jointe n'est plus celle d'Apple"
        );

        let (_, certificat) = X509Certificate::from_der(RACINE_APPLE).expect("racine lisible");
        assert!(
            certificat.subject().to_string().contains("Apple Root CA - G3"),
            "sujet inattendu : {}",
            certificat.subject()
        );
    }
}
