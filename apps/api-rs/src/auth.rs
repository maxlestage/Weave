//! Authentification par jeton porteur.
//!
//! Weave n'utilise pas de mot de passe : la connexion se fait par code à usage
//! unique envoyé par e-mail, puis par un couple accès/rafraîchissement.
//! L'accès est court ; le rafraîchissement est rotatif et stocké haché.

use crate::messages::Msg;
use crate::{
    AppState, cache,
    entities::{accounts, subscriptions},
    error::{AppError, non_autorise},
};
use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

/// Durée de vie du résumé d'identité : assez court pour qu'un changement de
/// palier ou de statut se propage vite, assez long pour épargner un
/// aller-retour base à chaque requête.
const TTL_RESUME_IDENTITE: u64 = 15 * 60;

const EMETTEUR: &str = "weave";
const AUDIENCE: &str = "weave-app";

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub exp: i64,
    pub iat: i64,
}

/// Résumé d'identité, mis en cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompteAuthentifie {
    pub id: String,
    pub handle: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub status: String,
    pub timezone: String,
    pub locale: String,
    pub tier: String,
    pub verified: bool,
}

pub fn emettre_jeton(secret: &str, compte_id: &str, ttl_secondes: i64) -> Result<String, AppError> {
    let maintenant = Utc::now().timestamp();
    let claims = Claims {
        sub: compte_id.to_string(),
        iss: EMETTEUR.to_string(),
        aud: AUDIENCE.to_string(),
        exp: maintenant + ttl_secondes,
        iat: maintenant,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|erreur| {
        tracing::error!(erreur = %erreur, "émission du jeton impossible");
        AppError::new(crate::error::Code::Internal, Msg::ErreurInterne.t())
    })
}

fn lire_jeton(secret: &str, jeton: &str) -> Option<Claims> {
    let mut regles = Validation::default();
    regles.set_issuer(&[EMETTEUR]);
    regles.set_audience(&[AUDIENCE]);
    decode::<Claims>(jeton, &DecodingKey::from_secret(secret.as_bytes()), &regles)
        .ok()
        .map(|donnees| donnees.claims)
}

/// Charge le résumé d'identité, du cache ou de la base.
async fn charger_compte(state: &AppState, compte_id: &str) -> Option<CompteAuthentifie> {
    let cle = cache::cles::identite(compte_id);
    if let Some(en_cache) = cache::lire_json::<CompteAuthentifie>(&state.cache, &cle).await {
        return Some(en_cache);
    }

    // Une panne de base ne doit pas se déguiser en « non authentifié » : le
    // client réessaierait avec un jeton valable, sans jamais comprendre. On la
    // journalise pour qu'elle soit lisible, puis on refuse.
    let compte = match accounts::Entity::find_by_id(compte_id.to_string())
        .one(&state.db)
        .await
    {
        Ok(Some(ligne)) => ligne,
        Ok(None) => return None,
        Err(erreur) => {
            tracing::error!(erreur = %erreur, compte = compte_id, "compte illisible");
            return None;
        }
    };

    let abonnement = match subscriptions::Entity::find()
        .filter(subscriptions::Column::AccountId.eq(compte_id))
        .one(&state.db)
        .await
    {
        Ok(ligne) => ligne,
        Err(erreur) => {
            // Un abonnement illisible ne doit pas fermer la porte : le compte
            // retombe au palier de départ, ce qui est le repli sûr.
            tracing::error!(erreur = %erreur, compte = compte_id, "abonnement illisible");
            None
        }
    };

    // Un abonnement sans échéance ne se périme pas ; sinon il faut qu'elle
    // soit devant nous. Hors de ces deux cas, le compte retombe au palier de
    // départ — jamais au palier payant par défaut.
    let palier = abonnement
        .filter(|a| {
            a.expires_at
                .map(|echeance| echeance.and_utc() > Utc::now())
                .unwrap_or(true)
        })
        .map(|a| a.tier)
        .unwrap_or_else(|| "depart".to_string());

    let resume = CompteAuthentifie {
        id: compte.id,
        handle: compte.handle,
        display_name: compte.display_name,
        status: compte.status,
        timezone: compte.timezone,
        locale: compte.locale,
        tier: palier,
        verified: compte.verified,
    };

    // Un cache indisponible ne doit pas refuser la requête : il fait gagner un
    // aller-retour, il ne conditionne pas l'authentification.
    if let Err(erreur) = cache::ecrire_json(&state.cache, &cle, &resume, TTL_RESUME_IDENTITE).await
    {
        tracing::warn!(erreur = %erreur, "résumé d'identité non mis en cache");
    }

    Some(resume)
}

/// Invalide le résumé d'identité (changement de palier, de statut, de fuseau).
pub async fn oublier_compte(state: &AppState, compte_id: &str) {
    if let Err(erreur) = cache::oublier(&state.cache, &cache::cles::identite(compte_id)).await {
        tracing::warn!(erreur = %erreur, "résumé d'identité non invalidé");
    }
}

/// Extracteur : exige un compte authentifié, ou refuse la requête.
pub struct Authentifie(pub CompteAuthentifie);

impl FromRequestParts<AppState> for Authentifie {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let entete = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| non_autorise(Msg::AuthentificationRequise))?;

        let jeton = entete
            .strip_prefix("Bearer ")
            .or_else(|| entete.strip_prefix("bearer "))
            .ok_or_else(|| non_autorise(Msg::AuthentificationRequise))?
            .trim();

        let claims = lire_jeton(&state.config.auth.jwt_secret, jeton)
            .ok_or_else(|| non_autorise(Msg::AuthentificationRequise))?;

        let compte = charger_compte(state, &claims.sub)
            .await
            .ok_or_else(|| non_autorise(Msg::AuthentificationRequise))?;

        if compte.status == "suspended" {
            return Err(non_autorise(Msg::CompteSuspendu));
        }

        // Un compte en cours de suppression ne doit plus rien pouvoir faire.
        //
        // La suppression révoque les jetons de renouvellement, mais le jeton
        // d'accès déjà émis vit encore un quart d'heure : ce seul portier
        // laissait donc publier, demander et écrire pendant ce temps, alors
        // que la page publique promet un compte « invisible et inutilisable
        // dans l'intervalle ». Il n'existe aucune route pour revenir en
        // arrière : le délai de trente jours sert à traiter une suppression
        // demandée par erreur, et cela passe par l'assistance, pas par le
        // jeton qu'on avait encore en poche.
        if compte.status == "deleting" {
            return Err(non_autorise(Msg::CompteEnSuppression));
        }

        Ok(Authentifie(compte))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "un-secret-de-test-assez-long-pour-etre-credible";

    /// Ce chemin a paniqué une fois : `jsonwebtoken` 11 exige qu'un
    /// fournisseur cryptographique soit choisi par les features, faute de quoi
    /// il s'arrête net à la première vérification. Une panique dans
    /// l'authentification est un déni de service, pas un bogue mineur.
    #[test]
    fn un_jeton_emis_est_relu_sans_paniquer() {
        let jeton = emettre_jeton(SECRET, "cmp_001", 900).expect("émission");
        let claims = lire_jeton(SECRET, &jeton).expect("relecture");
        assert_eq!(claims.sub, "cmp_001");
        assert_eq!(claims.iss, EMETTEUR);
        assert_eq!(claims.aud, AUDIENCE);
    }

    #[test]
    fn un_jeton_signe_d_un_autre_secret_est_refuse() {
        let jeton = emettre_jeton(SECRET, "cmp_001", 900).expect("émission");
        assert!(lire_jeton("un-autre-secret-tout-aussi-long-mais-different", &jeton).is_none());
    }

    #[test]
    fn un_jeton_expire_est_refuse() {
        let jeton = emettre_jeton(SECRET, "cmp_001", -3600).expect("émission");
        assert!(lire_jeton(SECRET, &jeton).is_none());
    }

    /// `Validation` tolère 60 secondes de dérive d'horloge, et exige `exp`.
    /// C'est le comportement voulu — deux machines ne sont jamais à la
    /// milliseconde —, mais il vaut d'être écrit : un jeton tout juste expiré
    /// passe encore, et c'est normal.
    #[test]
    fn la_tolerance_d_horloge_est_d_une_minute() {
        let a_peine_expire = emettre_jeton(SECRET, "cmp_001", -30).expect("émission");
        assert!(lire_jeton(SECRET, &a_peine_expire).is_some());

        let franchement_expire = emettre_jeton(SECRET, "cmp_001", -120).expect("émission");
        assert!(lire_jeton(SECRET, &franchement_expire).is_none());
    }

    /// Un jeton qu'on n'a pas signé est refusé, quelle qu'en soit la forme.
    ///
    /// Le test qui gardait ce terrain interrogeait `Validation::default()` —
    /// le réglage par défaut de la BIBLIOTHÈQUE, pas le nôtre. Il serait resté
    /// vert si `lire_jeton` s'était mis à employer d'autres règles : il
    /// décrivait une dépendance, il ne gardait pas un chemin.
    ///
    /// Celui-ci forge trois jetons — signature vide, absente, quelconque — et
    /// les passe à `lire_jeton`, qui est ce qu'un attaquant atteindrait.
    ///
    /// Ce qu'il prouve exactement : c'est la VÉRIFICATION DE SIGNATURE qui les
    /// refuse. Vérifié en la retirant — les trois formes passaient alors.
    /// L'en-tête « alg: none » n'y est pour rien : `jsonwebtoken` n'a pas de
    /// variante `None` dans son énumération d'algorithmes, et ne pourrait donc
    /// pas l'accepter même mal réglé. Le dire évite de croire ce test plus
    /// large qu'il n'est.
    #[test]
    fn un_jeton_sans_signature_est_refuse() {
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

        let entete = URL_SAFE_NO_PAD.encode(br#"{"alg":"none","typ":"JWT"}"#);
        let charge = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&Claims {
                sub: "cmp_001".to_string(),
                exp: (Utc::now() + chrono::Duration::seconds(900)).timestamp(),
                iat: Utc::now().timestamp(),
                iss: EMETTEUR.to_string(),
                aud: AUDIENCE.to_string(),
            })
            .expect("charge sérialisable"),
        );

        // Trois formes de la même attaque : signature vide, signature absente,
        // et une signature quelconque.
        for forge in [
            format!("{entete}.{charge}."),
            format!("{entete}.{charge}"),
            format!("{entete}.{charge}.nimportequoi"),
        ] {
            assert!(
                lire_jeton(SECRET, &forge).is_none(),
                "un jeton « alg: none » a été accepté : {forge}"
            );
        }
    }

    /// Et les règles employées sont bien celles qu'on croit.
    ///
    /// Complément du test ci-dessus, pas son remplaçant : celui-là dit
    /// POURQUOI la forgerie échoue, celui-ci dit QU'ELLE échoue.
    #[test]
    fn les_regles_exigent_hs256_et_une_expiration() {
        let regles = Validation::default();
        assert_eq!(regles.algorithms, vec![jsonwebtoken::Algorithm::HS256]);
        assert!(regles.required_spec_claims.contains("exp"));
    }

    #[test]
    fn ce_qui_n_est_pas_un_jeton_est_refuse_sans_paniquer() {
        for entree in [
            "",
            "pas-un-jeton",
            "a.b.c",
            "..",
            "eyJhbGciOiJub25lIn0..",
            "�",
        ] {
            assert!(lire_jeton(SECRET, entree).is_none(), "refusé : {entree}");
        }
    }
}
