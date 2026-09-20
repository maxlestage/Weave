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

use crate::{AppState, auth::emettre_jeton, cache, construire_routeur, env::*};
use axum::{
    body::Body,
    http::{Request, StatusCode, header::AUTHORIZATION},
};
use http_body_util::BodyExt;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

const SECRET: &str = "un-secret-de-test-assez-long-pour-passer-la-validation";
const SECRET_MEDIA: &str = "un-autre-secret-de-test-assez-long-pour-les-medias";

/// Efface les bases de test des passages précédents, une fois par processus.
///
/// Les fichiers survivaient à la suite. C'est le prix du choix expliqué plus
/// bas — une base sur disque plutôt qu'en mémoire — mais rien ne le payait :
/// chaque passage laissait une base par test dans le dossier temporaire, et
/// SQLite y ajoute ses deux annexes `-wal` et `-shm`. Une seule session de
/// travail, où la suite tourne des dizaines de fois, y a déposé cinquante-
/// quatre mille fichiers et dix-huit gigaoctets. Sur une machine de
/// développement le disque finit par se remplir sans qu'on sache de quoi, et
/// l'écriture qui échoue alors n'a plus aucun rapport avec sa cause.
///
/// Un `Drop` sur la connexion serait le geste naturel, mais il ne tient pas :
/// `DatabaseConnection` traverse des bornes génériques (`C: ConnectionTrait`)
/// qu'aucune déréférence ne franchit, et il faudrait toucher les trente sites
/// d'appel. Surtout, il ne nettoierait rien d'un passage interrompu — or c'est
/// exactement ce qui laisse le plus de fichiers derrière lui.
///
/// Le balayage, lui, ramasse aussi ceux-là. Il ne tourne qu'au premier appel :
/// il ne voit donc que des fichiers antérieurs au passage en cours, jamais les
/// siens. La borne d'une heure et l'exclusion du processus courant ne
/// protègent qu'un second passage lancé en parallèle.
fn balayer_les_passages_precedents() {
    static UNE_FOIS: std::sync::Once = std::sync::Once::new();
    UNE_FOIS.call_once(|| {
        balayer(&std::env::temp_dir(), std::process::id(), UNE_HEURE);
    });
}

/// L'âge au-delà duquel un fichier de test n'appartient plus à personne.
///
/// Un test dure des secondes. Une heure ne protège donc rien du passage en
/// cours — le balayage ne tourne qu'à son premier appel, et ne peut voir que
/// des fichiers antérieurs. Elle protège un SECOND passage lancé en parallèle,
/// dont les fichiers sont, eux aussi, tout frais.
const UNE_HEURE: std::time::Duration = std::time::Duration::from_secs(3600);

/// Le balayage proprement dit, sur un dossier et un âge donnés.
///
/// Séparé de son appelant pour qu'un test puisse le lancer sur son propre
/// dossier : le viser sur le dossier temporaire réel effacerait les bases des
/// autres tests en train de tourner à côté.
fn balayer(dossier: &std::path::Path, notre_processus: u32, age_minimum: std::time::Duration) {
    let notre = format!("-{notre_processus}-");
    let Ok(entrees) = std::fs::read_dir(dossier) else {
        return;
    };
    for entree in entrees.flatten() {
        let nom = entree.file_name();
        let Some(nom) = nom.to_str() else { continue };
        let notre_forme = ["weave-test-", "weave-dist-", "weave-partage-"]
            .iter()
            .any(|prefixe| nom.starts_with(prefixe));
        if !notre_forme || nom.contains(&notre) {
            continue;
        }
        let assez_vieux = entree
            .metadata()
            .and_then(|m| m.modified())
            .and_then(|t| t.elapsed().map_err(std::io::Error::other))
            .is_ok_and(|age| age >= age_minimum);
        if !assez_vieux {
            continue;
        }
        let chemin = entree.path();
        // `weave-dist-*` est un dossier, les deux autres des fichiers.
        let _ = if chemin.is_dir() {
            std::fs::remove_dir_all(&chemin)
        } else {
            std::fs::remove_file(&chemin)
        };
    }
}

#[test]
fn le_balayage_n_emporte_que_les_restes_des_passages_precedents() {
    // Un dossier à nous : viser le dossier temporaire réel effacerait les
    // bases des tests qui tournent en parallèle de celui-ci.
    let dossier = std::env::temp_dir().join(format!("weave-balayage-{}", std::process::id()));
    std::fs::create_dir_all(&dossier).expect("dossier d'épreuve");

    let poser = |nom: &str| {
        let chemin = dossier.join(nom);
        std::fs::write(&chemin, b"x").expect("fichier d'épreuve");
        chemin
    };

    // Les restes d'un passage mort : la base, ses deux annexes, un partage.
    let base = poser("weave-test-424242-7.sqlite");
    let wal = poser("weave-test-424242-7.sqlite-wal");
    let shm = poser("weave-test-424242-7.sqlite-shm");
    let partage = poser("weave-partage-424242-plan");
    // Le site rendu est un DOSSIER, pas un fichier : `remove_file` ne suffit
    // pas, et c'est lui qui pèse le plus lourd.
    let dist = dossier.join("weave-dist-424242-3");
    std::fs::create_dir_all(dist.join("fr")).expect("dossier rendu");
    std::fs::write(dist.join("fr/index.html"), b"<!doctype html>").expect("page rendue");

    // Ce que le balayage doit épargner : un fichier qui n'est pas à nous, et
    // les nôtres tant qu'ils sont frais.
    let etranger = poser("autre-outil-424242-0.tmp");
    let a_nous = poser(&format!("weave-test-{}-0.sqlite", std::process::id()));

    // Un âge nul rend tout fichier « assez vieux » : le tri ne se joue plus
    // que sur le nom, et l'on éprouve les deux règles séparément.
    balayer(&dossier, std::process::id(), std::time::Duration::ZERO);

    for reste in [&base, &wal, &shm, &partage] {
        assert!(!reste.exists(), "{} aurait dû être balayé", reste.display());
    }
    assert!(!dist.exists(), "le rendu du site aurait dû être balayé");
    assert!(etranger.exists(), "un fichier qui n'est pas à nous reste");
    assert!(a_nous.exists(), "le passage en cours garde ses fichiers");

    // Et maintenant la règle d'âge, seule. Le fichier est posé ICI et non
    // plus haut : le premier balayage, d'âge nul, l'aurait emporté comme les
    // autres — ma première version du test l'avait posé avec eux, et c'est
    // ce test-ci qui me l'a appris.
    let frais = poser("weave-test-424243-0.sqlite");
    balayer(&dossier, std::process::id(), UNE_HEURE);
    assert!(frais.exists(), "un fichier récent d'un autre passage reste");

    std::fs::remove_dir_all(&dossier).expect("dossier d'épreuve effacé");
}

pub async fn base_de_test() -> DatabaseConnection {
    // Ni `sqlite::memory:` ni une base nommée en cache partagé ne conviennent :
    // la première donne une base DISTINCTE par connexion du pool, la seconde
    // s'évapore dès que le pool ferme sa dernière connexion. Un fichier tient
    // aussi longtemps que le test, sans ambiguïté.
    balayer_les_passages_precedents();
    static COMPTEUR: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = COMPTEUR.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let chemin = std::env::temp_dir().join(format!("weave-test-{}-{n}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&chemin);

    let db = Database::connect(format!("sqlite:{}?mode=rwc", chemin.display()))
        .await
        .expect("base SQLite de test");

    // Les migrations sont appliquées par le moteur qui les applique en
    // production, et non recopiées ici.
    //
    // Le harnais incluait `0_init` en dur. Toute migration suivante était donc
    // invisible des tests : la suite entière tournait contre un schéma figé
    // au premier jour, et personne ne l'aurait su avant la production. Passer
    // par `migrations::appliquer` fait d'une pierre deux coups — les tests
    // voient le schéma réel, et le moteur de migrations est exercé par chacun
    // d'eux.
    crate::migrations::appliquer(&db)
        .await
        .expect("migrations appliquées");
    db
}

fn configuration() -> Env {
    Env {
        mode: Mode::Test,
        port: 0,
        db: Db {
            driver: Driver::Sqlite,
            url: String::new(),
            ssl_insecure: false,
        },
        cache: Cache {
            url: String::new(),
            tls_insecure: false,
        },
        media: Media {
            signing_secret: SECRET_MEDIA.to_string(),
            base_url: "https://exemple.test/media".to_string(),
            ttl_url_signee_secondes: 600,
        },
        apns: Apns {
            key_id: None,
            team_id: None,
            key_path: None,
            bundle_id: "com.weave.app".to_string(),
            environnement: "sandbox".to_string(),
            configure: false,
        },
        app_store: AppStore {
            environnement: "sandbox".to_string(),
            configure: false,
        },
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
            // Une racine de test, pour que les achats éprouvés ici portent de
            // vraies signatures plutôt que de contourner la vérification.
            racine_storekit: Arc::new(crate::tests::storekit::racine_de_test()),
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

    /// Enregistre un appareil joignable par alerte, et rend son jeton APNs.
    ///
    /// La ligne est insérée directement : c'est la POUSSÉE qu'on veut éprouver,
    /// pas l'enregistrement d'un appareil, qui a ses propres tests.
    pub async fn appareil(&self, compte_id: &str) -> String {
        use crate::entities::devices;
        use sea_orm::{ActiveModelTrait, Set};

        let jeton = format!("apns-{compte_id}");
        devices::ActiveModel {
            id: Set(format!("dev-{compte_id}")),
            account_id: Set(compte_id.to_string()),
            platform: Set("ios".to_string()),
            vendor_id: Set(format!("vendor-{compte_id}")),
            model: Set(None),
            os_version: Set(None),
            app_version: Set(None),
            apns_token: Set(Some(jeton.clone())),
            push_to_start_token: Set(None),
            apns_environment: Set("sandbox".to_string()),
            last_seen_at: Set(chrono::Utc::now().naive_utc()),
            created_at: Set(chrono::Utc::now().naive_utc()),
        }
        .insert(&self.db)
        .await
        .expect("appareil inséré");
        jeton
    }

    /// Les alertes poussées à un appareil, titre et corps.
    ///
    /// Les Live Activities sont écartées : elles passent par le même client,
    /// et ce qu'on éprouve ici est ce qui s'affiche en bannière.
    pub fn alertes_vers(&self, jeton: &str) -> Vec<(String, String)> {
        self.etat
            .apns
            .traces()
            .into_iter()
            .filter(|trace| trace.jeton_appareil == jeton && trace.type_envoi == "alert")
            .map(|trace| {
                let alerte = &trace.charge["aps"]["alert"];
                (
                    alerte["title"].as_str().unwrap_or_default().to_string(),
                    alerte["body"].as_str().unwrap_or_default().to_string(),
                )
            })
            .collect()
    }

    /// Une adresse e-mail propre à ce test.
    ///
    /// Les demandes de code sont limitées en débit par empreinte d'adresse, et
    /// ce compteur vit dans le cache partagé : une adresse fixe ferait passer
    /// la suite une fois, puis rendrait 429 à l'exécution suivante.
    pub fn email(&self, nom: &str) -> String {
        format!("{nom}-{}@exemple.fr", self.suffixe)
    }

    /// Donne le consentement aux données sensibles.
    ///
    /// Nécessaire avant tout `seeking` : le genre recherché relève de
    /// l'article 9, et la route le refuse sans consentement en cours de
    /// validité. C'est une méthode plutôt qu'un appel recopié pour que la
    /// version du texte ne vive qu'à un endroit dans les tests.
    pub async fn consentir(&self, jeton: &str) {
        let (statut, corps) = self
            .post(
                "/v1/me/consents",
                Some(jeton),
                serde_json::json!({
                    "kind": crate::routes::consentements::DONNEES_SENSIBLES,
                    "version": crate::routes::consentements::VERSION_POLITIQUE,
                }),
            )
            .await;
        assert_eq!(statut, StatusCode::OK, "consentement refusé : {corps}");
    }

    /// Un jeton d'accès pour un compte nommé dans un test.
    pub fn jeton(&self, nom: &str) -> String {
        jeton_pour(&self.id(nom))
    }

    pub async fn get(&self, chemin: &str, jeton: Option<&str>) -> (StatusCode, Value) {
        self.appeler("GET", chemin, jeton, None).await
    }

    pub async fn post(
        &self,
        chemin: &str,
        jeton: Option<&str>,
        corps: Value,
    ) -> (StatusCode, Value) {
        self.appeler("POST", chemin, jeton, Some(corps)).await
    }

    pub async fn delete(&self, chemin: &str, jeton: Option<&str>) -> (StatusCode, Value) {
        self.appeler("DELETE", chemin, jeton, None).await
    }

    /// Envoie des octets bruts — pour les routes qui reçoivent un fichier.
    pub async fn put_octets(
        &self,
        chemin: &str,
        jeton: &str,
        octets: Vec<u8>,
    ) -> (StatusCode, Value) {
        let requete = Request::builder()
            .method("PUT")
            .uri(chemin)
            .extension(adresse_appelante())
            .header(AUTHORIZATION, format!("Bearer {jeton}"))
            .header("content-type", "application/octet-stream")
            .body(Body::from(octets))
            .expect("requête bien formée");

        let reponse = self
            .routeur
            .clone()
            .oneshot(requete)
            .await
            .expect("réponse");
        let statut = reponse.status();
        let corps = http_body_util::BodyExt::collect(reponse.into_body())
            .await
            .expect("corps lu")
            .to_bytes();
        let valeur = serde_json::from_slice(&corps).unwrap_or(Value::Null);
        (statut, valeur)
    }

    pub async fn patch(
        &self,
        chemin: &str,
        jeton: Option<&str>,
        corps: Value,
    ) -> (StatusCode, Value) {
        self.appeler("PATCH", chemin, jeton, Some(corps)).await
    }

    pub async fn put(
        &self,
        chemin: &str,
        jeton: Option<&str>,
        corps: Value,
    ) -> (StatusCode, Value) {
        self.appeler("PUT", chemin, jeton, Some(corps)).await
    }

    /// Un POST dans une langue donnée, en-têtes de la réponse compris.
    ///
    /// La langue d'une erreur ne se lit ni dans le corps seul ni dans les
    /// en-têtes seuls : le message est dans l'un, `Content-Language` dans
    /// l'autre, et c'est leur accord qu'il faut pouvoir vérifier.
    pub async fn post_dans_la_langue(
        &self,
        chemin: &str,
        jeton: Option<&str>,
        corps: Value,
        accept_language: &str,
    ) -> (StatusCode, axum::http::HeaderMap, Value) {
        let mut requete = Request::builder()
            .method("POST")
            .uri(chemin)
            .extension(adresse_appelante())
            .header("content-type", "application/json")
            .header(axum::http::header::ACCEPT_LANGUAGE, accept_language);
        if let Some(j) = jeton {
            requete = requete.header(AUTHORIZATION, format!("Bearer {j}"));
        }
        let requete = requete
            .body(Body::from(corps.to_string()))
            .expect("requête bien formée");

        let reponse = self
            .routeur
            .clone()
            .oneshot(requete)
            .await
            .expect("le service répond");
        let statut = reponse.status();
        let entetes = reponse.headers().clone();
        let octets = reponse
            .into_body()
            .collect()
            .await
            .expect("corps lisible")
            .to_bytes();
        let json = serde_json::from_slice(&octets).unwrap_or(Value::Null);
        (statut, entetes, json)
    }

    /// La réponse entière, en-têtes compris : ce que `get` jette est
    /// précisément ce que le cache du navigateur lit.
    pub async fn get_brut(&self, chemin: &str) -> (StatusCode, axum::http::HeaderMap, String) {
        self.get_brut_dans_la_langue(chemin, None).await
    }

    /// Le même GET, en annonçant une langue préférée.
    pub async fn get_brut_dans_la_langue(
        &self,
        chemin: &str,
        accept_language: Option<&str>,
    ) -> (StatusCode, axum::http::HeaderMap, String) {
        let mut requete = Request::builder()
            .method("GET")
            .uri(chemin)
            .extension(adresse_appelante());
        if let Some(langue) = accept_language {
            requete = requete.header(axum::http::header::ACCEPT_LANGUAGE, langue);
        }
        let requete = requete.body(Body::empty()).expect("requête bien formée");

        let reponse = self
            .routeur
            .clone()
            .oneshot(requete)
            .await
            .expect("le service répond");
        let statut = reponse.status();
        let entetes = reponse.headers().clone();
        let octets = reponse
            .into_body()
            .collect()
            .await
            .expect("corps lisible")
            .to_bytes();
        (
            statut,
            entetes,
            String::from_utf8_lossy(&octets).into_owned(),
        )
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
        let octets = reponse
            .into_body()
            .collect()
            .await
            .expect("corps lisible")
            .to_bytes();
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

/// Vérifie qu'une requête a été REFUSÉE, et pour une raison qui vient du client.
///
/// `assert_ne!(statut, OK)` passe pour n'importe quelle erreur — y compris un
/// 500 dû à une régression, un 401 sur une session mal montée, ou un 404 sur
/// une adresse mal orthographiée. Un test qui veut dire « le palier refuse
/// ceci » passerait alors au vert le jour où la route se met à planter, ce qui
/// est exactement le contraire de ce qu'on lui demande.
///
/// Le code d'erreur attendu est exigé quand on le connaît : c'est lui qui dit
/// POURQUOI c'est refusé, et l'application s'en sert pour proposer la suite.
#[track_caller]
pub fn refuse(statut: StatusCode, corps: &Value, code_attendu: &str, contexte: &str) {
    assert!(
        statut.is_client_error(),
        "{contexte} — attendu un refus du client, obtenu {statut} : {corps}"
    );
    assert_eq!(
        corps["error"].as_str().unwrap_or_default(),
        code_attendu,
        "{contexte} — refusé, mais pas pour la raison attendue : {corps}"
    );
}

pub fn jeton_pour(compte_id: &str) -> String {
    emettre_jeton(SECRET, compte_id, 900).expect("jeton émis")
}

/// Insère un compte utilisable : profil, préférences et abonnement compris.
///
/// `verified` vaut FAUX, comme à l'inscription. Le gabarit posait « vrai », ce
/// qu'aucun parcours réel ne produit : la route d'inscription écrit faux, et
/// seul un examen humain passe le drapeau à vrai. Un gabarit plus généreux que
/// la réalité fait passer des tests sur des comptes qui n'existent pas.
/// Un compte sans ces lignes est à moitié né, et la moitié des routes le
/// refuseraient pour de mauvaises raisons.
pub async fn compte_de_test(db: &DatabaseConnection, id: &str, palier: &str) {
    let maintenant = "2026-09-12 10:00:00";
    for sql in [
        format!(
            "INSERT INTO accounts (id,email,emailHash,handle,displayName,birthDate,status,timezone,locale,verified,createdAt,updatedAt) VALUES ('{id}','{id}@exemple.fr','h_{id}','{id}','Compte {id}','2000-01-15 00:00:00','active','Europe/Paris','fr-FR',0,'{maintenant}','{maintenant}')"
        ),
        format!(
            "INSERT INTO profiles (id,accountId,city,latRounded,lonRounded,gender,bio,createdAt,updatedAt) VALUES ('prf_{id}','{id}','Lyon',45.75,4.85,'autre','','{maintenant}','{maintenant}')"
        ),
        format!(
            "INSERT INTO preferences (id,accountId,minAge,maxAge,maxDistanceKm,seekingJson,categoriesJson,updatedAt) VALUES ('pre_{id}','{id}',18,60,25,'[]','[]','{maintenant}')"
        ),
        format!(
            "INSERT INTO subscriptions (id,accountId,tier,environment,inGracePeriod,updatedAt,createdAt) VALUES ('sub_{id}','{id}','{palier}','sandbox',0,'{maintenant}','{maintenant}')"
        ),
    ] {
        db.execute_unprepared(&sql)
            .await
            .unwrap_or_else(|e| panic!("insertion refusée : {e}\n{sql}"));
    }
}

mod contrat;
mod langues;
mod parcours;
pub mod storekit;

/// Le schéma des tests est celui que la production appliquera.
///
/// Le harnais incluait `0_init` en dur : toute migration suivante restait
/// invisible des tests, qui tournaient contre un schéma figé au premier jour.
/// Ce test vérifie maintenant qu'une table née d'une migration ULTÉRIEURE est
/// bien là — c'est-à-dire que le harnais a déroulé toutes les migrations, et
/// pas seulement la première.
#[tokio::test]
async fn le_schema_de_test_est_bien_celui_du_depot() {
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

    // Et une table née d'une migration postérieure à « 0_init » : c'est elle
    // qui distingue un harnais qui migre d'un harnais qui recopie.
    db.execute_unprepared("SELECT count(*) FROM media_objects")
        .await
        .expect("les migrations postérieures à « 0_init » doivent être appliquées");
}
mod activite;
mod blocages;
mod consentements;
mod conversations;
mod demandes;
mod export;
mod fil;
mod offres;
mod profil;
mod session;
mod verification;
mod vitrine;
