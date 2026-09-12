//! Primitives cryptographiques.
//!
//! Les formats sont ceux de l'API TypeScript, au caractère près : une URL de
//! média déjà signée doit rester vérifiable, et un jeton de rafraîchissement
//! déjà émis rester valable. Changer un encodage ici déconnecterait tout le
//! monde et rendrait illisibles les médias déjà servis.

use argon2::{Algorithm, Argon2, Params, Version};
use password_hash::{phc::PasswordHash, PasswordHasher, PasswordVerifier};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, KeyInit, Mac};
use rand::RngExt;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

/// SHA-256 hexadécimal, utilisé pour les index sans exposition de la valeur.
pub fn sha256_hex(valeur: &str) -> String {
    hex::encode(Sha256::digest(valeur.as_bytes()))
}

/// Normalise un e-mail avant hachage : minuscules, espaces retirés.
pub fn normaliser_email(email: &str) -> String {
    email.trim().to_lowercase()
}

pub fn hash_email(email: &str) -> String {
    sha256_hex(&normaliser_email(email))
}

/// Jeton opaque adapté aux URL, 256 bits d'entropie.
pub fn jeton_opaque() -> String {
    let mut octets = [0u8; 32];
    rand::rng().fill(&mut octets);
    URL_SAFE_NO_PAD.encode(octets)
}

/// Code de connexion à six chiffres, tiré uniformément.
///
/// L'implémentation TypeScript prenait un modulo sur 32 bits, ce qui penchait
/// très légèrement vers les petites valeurs. `random_range` tire sans ce biais ;
/// le format rendu est identique.
pub fn code_otp() -> String {
    let valeur: u32 = rand::rng().random_range(0..1_000_000);
    format!("{valeur:06}")
}

/// Paramètres Argon2id repris de l'API TypeScript : sans eux, les empreintes
/// déjà en base ne seraient plus vérifiables.
fn argon2() -> Argon2<'static> {
    let params = Params::new(19_456, 2, 1, None).expect("paramètres Argon2 valides");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

/// Hachage d'un secret court (code OTP, jeton de rafraîchissement).
pub fn hacher_secret(secret: &str) -> Result<String, password_hash::Error> {
    Ok(argon2().hash_password(secret.as_bytes())?.to_string())
}

pub fn verifier_secret(secret: &str, empreinte: &str) -> bool {
    // Une empreinte illisible n'est pas une erreur à remonter : c'est un échec
    // de vérification, et rien d'autre ne doit transparaître à l'appelant.
    let Ok(attendue) = PasswordHash::new(empreinte) else {
        return false;
    };
    argon2()
        .verify_password(secret.as_bytes(), &attendue)
        .is_ok()
}

/// Comparaison à temps constant de deux chaînes.
pub fn egal_en_temps_constant(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}

/* ------------------------------------------------------------------ */
/* URL signées pour les médias                                         */
/* ------------------------------------------------------------------ */

/// La charge signée, écrite une seule fois : la signature et sa vérification
/// doivent la construire à l'identique, sinon tout média déjà servi devient
/// illisible.
pub(crate) fn charge(cle_objet: &str, expire_le: i64, flou: u32) -> String {
    format!("{cle_objet}:{expire_le}:{flou}")
}

pub(crate) fn signer(secret: &str, charge: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepte toute clé");
    mac.update(charge.as_bytes());
    URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
}

/// Signe l'accès à un objet média. Les photos et enregistrements vocaux ne sont
/// jamais servis par une URL devinable : chaque lecture passe par une signature
/// à durée limitée, liée au niveau de révélation demandé.
pub fn signer_url_media(
    base_url: &str,
    secret: &str,
    cle_objet: &str,
    expire_dans_secondes: i64,
    flou: u32,
) -> String {
    let expire_le = chrono::Utc::now().timestamp() + expire_dans_secondes;
    let signature = signer(secret, &charge(cle_objet, expire_le, flou));
    format!(
        "{}/{}?exp={}&blur={}&sig={}",
        base_url.trim_end_matches('/'),
        encoder_composant(cle_objet),
        expire_le,
        flou,
        signature
    )
}

/// Vérifie une URL signée. Renvoie `false` si expirée ou altérée.
pub fn verifier_signature_media(
    secret: &str,
    cle_objet: &str,
    expire_le: i64,
    flou: u32,
    signature: &str,
) -> bool {
    if expire_le < chrono::Utc::now().timestamp() {
        return false;
    }
    egal_en_temps_constant(&signer(secret, &charge(cle_objet, expire_le, flou)), signature)
}

/// Équivalent de `encodeURIComponent` : les clés d'objet peuvent contenir des
/// barres obliques, qui changeraient le chemin si on les laissait passer.
fn encoder_composant(valeur: &str) -> String {
    valeur
        .bytes()
        .map(|o| match o {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~' | b'*'
            | b'\'' | b'(' | b')' => (o as char).to_string(),
            _ => format!("%{o:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vecteurs produits par l'implémentation TypeScript en service. Les
    /// empreintes et les signatures déjà en base doivent rester lisibles : un
    /// encodage qui diverge ici déconnecterait tout le monde et rendrait
    /// illisibles les médias déjà servis.
    const SHA256_DE_WEAVE: &str =
        "775e432bb7ff3e08df7ac2395c8fd4a3a1f1a8da16c5fdf856e7dd44d0c76f1a";
    const HASH_EMAIL: &str = "2e4547a1c53dc149d09a5d52e2abe2587e1a36525400f8b7e5b000948cfadd66";
    const ARGON2_DU_CODE_123456: &str = "$argon2id$v=19$m=19456,t=2,p=1$nyEhF7GMfa0qkIHq3k7mn00fSsNYGRNiuBQWxpGIZDY$tOuNIrrYyC7oXO4Wg4/4GzzD1jByiBqbfCJfMqFunZU";
    const SECRET_MEDIA: &str = "secret-media-de-reference-pour-le-test";
    const SIGNATURE_ATTENDUE: &str = "xtVLpJgjEZK-FAIIcHrNayZS9Aosq0wdxlwN_G4bNRU";

    #[test]
    fn sha256_reproduit_les_valeurs_de_l_api_typescript() {
        assert_eq!(sha256_hex("weave"), SHA256_DE_WEAVE);
    }

    #[test]
    fn l_email_est_normalise_avant_hachage() {
        // Espaces et majuscules ne doivent pas produire deux comptes distincts.
        assert_eq!(hash_email("  Maxime@Exemple.FR  "), HASH_EMAIL);
        assert_eq!(hash_email("maxime@exemple.fr"), HASH_EMAIL);
    }

    #[test]
    fn les_empreintes_argon2_deja_en_base_restent_verifiables() {
        assert!(verifier_secret("123456", ARGON2_DU_CODE_123456));
        assert!(!verifier_secret("000000", ARGON2_DU_CODE_123456));
    }

    #[test]
    fn une_empreinte_illisible_echoue_sans_paniquer() {
        assert!(!verifier_secret("123456", "pas une empreinte"));
        assert!(!verifier_secret("123456", ""));
    }

    #[test]
    fn nos_propres_empreintes_portent_les_memes_parametres() {
        let empreinte = hacher_secret("123456").expect("hachage");
        assert!(empreinte.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"));
        assert!(verifier_secret("123456", &empreinte));
    }

    #[test]
    fn la_signature_des_medias_est_celle_de_l_api_typescript() {
        // Charge figée : c'est l'algorithme qu'on compare, pas l'horloge.
        let charge = charge("photos/abc def.jpg", 1_789_215_528, 12);
        assert_eq!(signer(SECRET_MEDIA, &charge), SIGNATURE_ATTENDUE);
    }

    #[test]
    fn l_url_signee_encode_les_barres_et_les_espaces() {
        let url = signer_url_media("https://exemple.test/media/", SECRET_MEDIA, "photos/abc def.jpg", 3600, 12);
        assert!(url.starts_with("https://exemple.test/media/photos%2Fabc%20def.jpg?exp="));
        assert!(url.contains("&blur=12&sig="));
    }

    #[test]
    fn une_signature_expiree_est_refusee() {
        assert!(!verifier_signature_media(SECRET_MEDIA, "photos/abc def.jpg", 1, 12, SIGNATURE_ATTENDUE));
    }

    #[test]
    fn une_signature_alteree_est_refusee() {
        let futur = chrono::Utc::now().timestamp() + 600;
        let bonne = signer(SECRET_MEDIA, &charge("photos/x.jpg", futur, 0));
        assert!(verifier_signature_media(SECRET_MEDIA, "photos/x.jpg", futur, 0, &bonne));
        // Le niveau de flou fait partie de la charge : le changer doit invalider.
        assert!(!verifier_signature_media(SECRET_MEDIA, "photos/x.jpg", futur, 8, &bonne));
        // Comme la clé de l'objet.
        assert!(!verifier_signature_media(SECRET_MEDIA, "photos/y.jpg", futur, 0, &bonne));
    }

    #[test]
    fn le_code_otp_fait_toujours_six_chiffres() {
        for _ in 0..500 {
            let code = code_otp();
            assert_eq!(code.len(), 6);
            assert!(code.chars().all(|c| c.is_ascii_digit()));
        }
    }
}
