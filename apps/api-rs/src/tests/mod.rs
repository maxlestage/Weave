//! Tests d'intégration : le service entier, monté et interrogé par HTTP.
//!
//! Ces tests montent le routeur complet sur une base SQLite en mémoire, dont
//! le schéma est celui que Prisma applique réellement. Ils exercent donc le
//! routage, les extracteurs, la sérialisation et la base — et non des
//! gestionnaires isolés, qu'un routage faux laisserait passer pour bons.
//!
//! Le cache, lui, est une vraie dépendance : ces tests exigent un Redis local.
//! Le quota de demandes n'a pas d'autre source de vérité, et le simuler
//! reviendrait à ne pas tester l'invariant central du produit.

use crate::{
    auth::emettre_jeton, cache, construire_routeur, env::*, AppState,
};
use axum::{
    body::Body,
    http::{header::AUTHORIZATION, Request, StatusCode},
};
use http_body_util::BodyExt;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

const SECRET: &str = "un-secret-de-test-assez-long-pour-passer-la-validation";
const SECRET_MEDIA: &str = "un-autre-secret-de-test-assez-long-pour-les-medias";

/// Le schéma tel que Prisma l'applique. Le lire depuis le dépôt plutôt que de
/// le recopier garantit que les tests portent sur les tables réelles : une
/// colonne ajoutée au schéma sans l'être ici ferait échouer le test, ce qui
/// est exactement ce qu'on veut.
const SCHEMA: &str = include_str!("../../migrations-sqlite/0_init/migration.sql");

pub async fn base_de_test() -> DatabaseConnection {
    // Ni `sqlite::memory:` ni une base nommée en cache partagé ne conviennent :
    // la première donne une base DISTINCTE par connexion du pool, la seconde
    // s'évapore dès que le pool ferme sa dernière connexion. Un fichier tient
    // aussi longtemps que le test, sans ambiguïté.
    static COMPTEUR: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = COMPTEUR.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let chemin = std::env::temp_dir().join(format!("weave-test-{}-{n}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&chemin);

    let db = Database::connect(format!("sqlite:{}?mode=rwc", chemin.display()))
        .await
        .expect("base SQLite de test");

    // Chaque instruction du fichier est précédée d'un « -- CreateTable ».
    // Écarter une instruction parce qu'elle COMMENCE par un commentaire les
    // écartait donc toutes, en silence : la base restait vide et l'échec
    // n'apparaissait qu'au premier INSERT. Les commentaires se retirent ligne
    // à ligne, pas instruction par instruction.
    for instruction in SCHEMA.split(';') {
        let sql: String = instruction
            .lines()
            .filter(|ligne| !ligne.trim_start().starts_with("--"))
            .collect::<Vec<_>>()
            .join("\n");
        let sql = sql.trim();
        if sql.is_empty() {
            continue;
        }
        db.execute_unprepared(sql)
            .await
            .unwrap_or_else(|e| panic!("schéma refusé : {e}\n{sql}"));
    }
    db
}

fn configuration() -> Env {
    Env {
        mode: Mode::Test,
        port: 0,
        db: Db { driver: Driver::Sqlite, url: String::new(), ssl_insecure: false },
        cache: Cache { url: String::new(), tls_insecure: false },
        media: Media {
            signing_secret: SECRET_MEDIA.to_string(),
            base_url: "https://exemple.test/media".to_string(),
            ttl_url_signee_secondes: 600,
        },
        apns: Apns {
            key_id: None, team_id: None, key_path: None,
            bundle_id: "com.weave.app".to_string(),
            environnement: "sandbox".to_string(),
            configure: false,
        },
        app_store: AppStore { environnement: "sandbox".to_string(), configure: false },
        auth: Auth {
            jwt_secret: SECRET.to_string(),
            access_ttl_secondes: 900,
            refresh_ttl_jours: 60,
        },
        web_origin: "https://exemple.test".to_string(),
        web_dist: None,
    }
}

pub struct Service {
    routeur: axum::Router,
    pub db: DatabaseConnection,
    /// L'état du service, pour les tests qui appellent une fonction interne
    /// plutôt qu'une route — un envoi de notification, par exemple, n'a pas
    /// d'adresse HTTP à viser.
    pub etat: AppState,
    /// Ce qui distingue les comptes d'un test de ceux d'un autre.
    ///
    /// La base est déjà propre à chaque test, mais le cache, lui, est partagé
    /// — et le quota de demandes n'y vit que sous l'identifiant du compte, pour
    /// la journée. Deux tests qui nommeraient leurs comptes pareil se
    /// partageraient donc le même quota, entre eux et d'une exécution à la
    /// suivante. Le suffixe sépare ce que la base séparait déjà.
    suffixe: String,
}

impl Service {
    /// Monte le service. Chaque test a son propre préfixe de clés dans le
    /// cache : sans cela, deux tests exécutés en parallèle se disputeraient le
    /// même quota.
    pub async fn monter() -> Self {
        Self::monter_avec(None).await
    }

    /// Même service, avec un site vitrine à servir.
    pub async fn monter_avec(web_dist: Option<String>) -> Self {
        let db = base_de_test().await;
        let mut config = configuration();
        config.web_dist = web_dist;
        let cache = cache::connecter(&Cache {
            url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            tls_insecure: false,
        })
        .await
        .expect("Redis local requis : le quota n'a pas d'autre source de vérité");

        let state = AppState {
            db: db.clone(),
            cache,
            config: Arc::new(config),
            // APNs n'est pas configuré en test : les envois sont simulés, et
            // c'est bien ce qu'on veut éprouver — le reste du produit doit
            // tourner sans certificat Apple.
            apns: Arc::new(crate::apns::ClientApns::new()),
        };
        static COMPTEUR: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let suffixe = format!(
            "{}x{}",
            std::process::id(),
            COMPTEUR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        Self {
            routeur: construire_routeur(state.clone()),
            db,
            etat: state,
            suffixe,
        }
    }

    /// L'identifiant réel d'un compte nommé dans un test.
    pub fn id(&self, nom: &str) -> String {
        format!("{nom}_{}", self.suffixe)
    }

    /// Insère un compte utilisable et rend son identifiant.
    pub async fn compte(&self, nom: &str, palier: &str) -> String {
        let id = self.id(nom);
        compte_de_test(&self.db, &id, palier).await;
        id
    }

    /// Une adresse e-mail propre à ce test.
    ///
    /// Les demandes de code sont limitées en débit par empreinte d'adresse, et
    /// ce compteur vit dans le cache partagé : une adresse fixe ferait passer
    /// la suite une fois, puis rendrait 429 à l'exécution suivante.
    pub fn email(&self, nom: &str) -> String {
        format!("{nom}-{}@exemple.fr", self.suffixe)
    }

    /// Un jeton d'accès pour un compte nommé dans un test.
    pub fn jeton(&self, nom: &str) -> String {
        jeton_pour(&self.id(nom))
    }

    pub async fn get(&self, chemin: &str, jeton: Option<&str>) -> (StatusCode, Value) {
        self.appeler("GET", chemin, jeton, None).await
    }

    pub async fn post(&self, chemin: &str, jeton: Option<&str>, corps: Value) -> (StatusCode, Value) {
        self.appeler("POST", chemin, jeton, Some(corps)).await
    }

    pub async fn delete(&self, chemin: &str, jeton: Option<&str>) -> (StatusCode, Value) {
        self.appeler("DELETE", chemin, jeton, None).await
    }

    pub async fn patch(&self, chemin: &str, jeton: Option<&str>, corps: Value) -> (StatusCode, Value) {
        self.appeler("PATCH", chemin, jeton, Some(corps)).await
    }

    pub async fn put(&self, chemin: &str, jeton: Option<&str>, corps: Value) -> (StatusCode, Value) {
        self.appeler("PUT", chemin, jeton, Some(corps)).await
    }

    /// La réponse entière, en-têtes compris : ce que `get` jette est
    /// précisément ce que le cache du navigateur lit.
    pub async fn get_brut(&self, chemin: &str) -> (StatusCode, axum::http::HeaderMap, String) {
        let requete = Request::builder()
            .method("GET")
            .uri(chemin)
            .extension(adresse_appelante())
            .body(Body::empty())
            .expect("requête bien formée");

        let reponse = self
            .routeur
            .clone()
            .oneshot(requete)
            .await
            .expect("le service répond");
        let statut = reponse.status();
        let entetes = reponse.headers().clone();
        let octets = reponse.into_body().collect().await.expect("corps lisible").to_bytes();
        (statut, entetes, String::from_utf8_lossy(&octets).into_owned())
    }

    async fn appeler(
        &self,
        methode: &str,
        chemin: &str,
        jeton: Option<&str>,
        corps: Option<Value>,
    ) -> (StatusCode, Value) {
        // Les routes qui limitent le débit extraient l'adresse de l'appelant.
        // En production, `into_make_service_with_connect_info` la pose ; ici,
        // c'est au test de le faire, sinon l'extracteur échoue et la route
        // rend 500 sans corps — ce qui ne se voit qu'à l'exécution.
        let mut requete = Request::builder()
            .method(methode)
            .uri(chemin)
            .extension(adresse_appelante());
        if let Some(j) = jeton {
            requete = requete.header(AUTHORIZATION, format!("Bearer {j}"));
        }
        let requete = match corps {
            Some(c) => requete
                .header("content-type", "application/json")
                .body(Body::from(c.to_string())),
            None => requete.body(Body::empty()),
        }
        .expect("requête bien formée");

        let reponse = self
            .routeur
            .clone()
            .oneshot(requete)
            .await
            .expect("le service répond");
        let statut = reponse.status();
        let octets = reponse.into_body().collect().await.expect("corps lisible").to_bytes();
        let json = serde_json::from_slice(&octets).unwrap_or(Value::Null);
        (statut, json)
    }
}

/// L'adresse de l'appelant, telle que l'extracteur `ConnectInfo` l'attend.
///
/// Chaque test a la sienne : les compteurs de débit sont indexés par adresse,
/// et deux tests partageant la même se disputeraient le même seau.
fn adresse_appelante() -> axum::extract::ConnectInfo<std::net::SocketAddr> {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    static COMPTEUR: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(0);
    let n = COMPTEUR.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // 127.p.y.z — et le `p` compte autant que le reste.
    //
    // Ce compteur repart à zéro à chaque exécution, alors que les seaux de
    // débit vivent quinze minutes dans un Redis partagé entre toutes. Deux
    // exécutions rapprochées réutilisaient donc les mêmes premières adresses
    // et se disputaient leurs seaux : à partir de la cinquième, la suite
    // rendait 429 sur des appels parfaitement légitimes, et l'échec accusait
    // le code plutôt que la mécanique du test.
    //
    // L'octet du processus sépare les exécutions ; les deux suivants laissent
    // soixante-cinq mille appels à chacune.
    let processus = std::process::id() as u8;
    let [haut, bas] = n.to_be_bytes();
    axum::extract::ConnectInfo(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, processus, haut, bas)),
        4000,
    ))
}

pub fn jeton_pour(compte_id: &str) -> String {
    emettre_jeton(SECRET, compte_id, 900).expect("jeton émis")
}

/// Insère un compte utilisable : profil, préférences et abonnement compris.
/// Un compte sans ces lignes est à moitié né, et la moitié des routes le
/// refuseraient pour de mauvaises raisons.
pub async fn compte_de_test(db: &DatabaseConnection, id: &str, palier: &str) {
    let maintenant = "2026-09-12 10:00:00";
    for sql in [
        format!("INSERT INTO accounts (id,email,emailHash,handle,displayName,birthDate,status,timezone,locale,verified,createdAt,updatedAt) VALUES ('{id}','{id}@exemple.fr','h_{id}','{id}','Compte {id}','2000-01-15 00:00:00','active','Europe/Paris','fr-FR',1,'{maintenant}','{maintenant}')"),
        format!("INSERT INTO profiles (id,accountId,city,latRounded,lonRounded,gender,bio,createdAt,updatedAt) VALUES ('prf_{id}','{id}','Lyon',45.75,4.85,'autre','','{maintenant}','{maintenant}')"),
        format!("INSERT INTO preferences (id,accountId,minAge,maxAge,maxDistanceKm,seekingJson,categoriesJson,updatedAt) VALUES ('pre_{id}','{id}',18,60,25,'[]','[]','{maintenant}')"),
        format!("INSERT INTO subscriptions (id,accountId,tier,environment,inGracePeriod,updatedAt,createdAt) VALUES ('sub_{id}','{id}','{palier}','sandbox',0,'{maintenant}','{maintenant}')"),
    ] {
        db.execute_unprepared(&sql)
            .await
            .unwrap_or_else(|e| panic!("insertion refusée : {e}\n{sql}"));
    }
}

mod contrat;
mod parcours;

#[tokio::test]
async fn le_schema_de_test_est_bien_celui_du_depot() {
    assert!(!SCHEMA.trim().is_empty(), "le schéma inclus est vide");
    let creations = SCHEMA.matches("CREATE TABLE").count();
    assert!(creations >= 18, "seulement {creations} tables dans le schéma inclus");

    // Une base fraîche doit accepter une écriture dans la table centrale :
    // c'est la preuve que le schéma a réellement été appliqué, et pas
    // seulement lu.
    let db = base_de_test().await;
    db.execute_unprepared(
        "INSERT INTO accounts (id,email,emailHash,handle,displayName,birthDate,status,timezone,locale,verified,createdAt,updatedAt) \
         VALUES ('sonde','s@x.fr','h','s','S','2000-01-01','active','Europe/Paris','fr-FR',1,'2026-01-01','2026-01-01')",
    )
    .await
    .expect("la table accounts doit exister");
}
mod vitrine;
mod session;
mod profil;
mod demandes;
mod export;
mod conversations;
mod offres;
mod activite;
