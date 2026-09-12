//! Configuration d'exécution, lue une seule fois au démarrage et validée
//! strictement : une variable manquante en production arrête le processus
//! plutôt que de laisser tourner un service à moitié configuré.
//!
//! Les problèmes sont TOUS rassemblés avant d'arrêter, et non levés au premier
//! rencontré. Sur un hébergeur, chaque démarrage raté coûte un déploiement et
//! une lecture de journal : apprendre les variables manquantes une par une
//! fait autant d'allers-retours qu'il en manque.

use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Development,
    Test,
    Production,
}

impl Mode {
    pub fn is_production(self) -> bool {
        self == Mode::Production
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Driver {
    Postgres,
    Sqlite,
}

impl Driver {
    pub fn as_str(self) -> &'static str {
        match self {
            Driver::Postgres => "postgres",
            Driver::Sqlite => "sqlite",
        }
    }
}

/// Ce qui empêche de démarrer, accumulé puis rapporté d'un bloc.
struct Probleme {
    variable: &'static str,
    raison: String,
    remede: &'static str,
}

/// Comment obtenir chaque valeur, dit une fois plutôt que cherché dans un guide.
fn remede(nom: &str) -> &'static str {
    match nom {
        "JWT_SECRET" => "une chaîne aléatoire d'au moins 32 caractères — `openssl rand -base64 32`",
        "MEDIA_SIGNING_SECRET" => "une autre chaîne aléatoire — `openssl rand -base64 32`",
        "DATABASE_URL" => "fournie par l'add-on Heroku Postgres, à attacher dans Resources",
        "REDIS_URL" => "fournie par l'add-on Heroku Key-Value Store, à attacher dans Resources",
        _ => "à définir dans la configuration de l'application",
    }
}

/// Lit une variable obligatoire, ou note ce qui manque pour le rapport final.
fn exiger(
    nom: &'static str,
    defaut_dev: Option<&str>,
    mode: Mode,
    problemes: &mut Vec<Probleme>,
) -> String {
    if let Some(v) = lire(nom) {
        return v;
    }
    if let Some(d) = defaut_dev {
        if mode != Mode::Production {
            return d.to_string();
        }
    }
    problemes.push(Probleme {
        variable: nom,
        raison: "absente".to_string(),
        remede: remede(nom),
    });
    String::new()
}

fn entier(nom: &str, defaut: i64) -> i64 {
    lire(nom).and_then(|v| v.parse().ok()).unwrap_or(defaut)
}

fn lire(nom: &str) -> Option<String> {
    match env::var(nom) {
        Ok(v) if !v.is_empty() => Some(v),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub struct Db {
    pub driver: Driver,
    pub url: String,
    /// Heroku présente un certificat auto-signé sur son réseau interne : la
    /// connexion reste chiffrée, l'identité du serveur n'est pas vérifiée.
    pub ssl_insecure: bool,
}

#[derive(Debug, Clone)]
pub struct Cache {
    pub url: String,
    pub tls_insecure: bool,
}

#[derive(Debug, Clone)]
pub struct Media {
    pub signing_secret: String,
    pub base_url: String,
    /// Les URL de médias sont courtes par conception : une adresse qui fuite
    /// ne doit pas rester lisible longtemps.
    pub ttl_url_signee_secondes: i64,
}

#[derive(Debug, Clone)]
pub struct Auth {
    pub jwt_secret: String,
    /// Court : l'accès expire vite, le rafraîchissement prend le relais.
    pub access_ttl_secondes: i64,
    pub refresh_ttl_jours: i64,
}

#[derive(Debug, Clone)]
pub struct Apns {
    pub key_id: Option<String>,
    pub team_id: Option<String>,
    pub key_path: Option<String>,
    pub bundle_id: String,
    pub environnement: String,
    /// Sans ces trois valeurs, aucune notification ne peut être signée : les
    /// envois sont alors simulés plutôt que de faire échouer les requêtes.
    pub configure: bool,
}

#[derive(Debug, Clone)]
pub struct AppStore {
    pub environnement: String,
    /// Sans ces trois valeurs, aucune transaction ne peut être vérifiée
    /// auprès d'Apple — et en production, aucune ne doit être acceptée.
    pub configure: bool,
}

#[derive(Debug, Clone)]
pub struct Env {
    pub mode: Mode,
    pub port: u16,
    pub db: Db,
    pub cache: Cache,
    pub media: Media,
    pub auth: Auth,
    pub apns: Apns,
    pub app_store: AppStore,
    pub web_origin: String,
    pub web_dist: Option<String>,
}

impl Env {
    pub fn is_production(&self) -> bool {
        self.mode.is_production()
    }
}

/// Lit la configuration, ou décrit d'un bloc tout ce qui l'empêche.
pub fn charger() -> Result<Env, String> {
    // Sur un hébergeur, l'absence de NODE_ENV ne doit surtout pas valoir
    // « développement ». Heroku définit `DYNO` sur chaque dyno : s'y fier
    // écarte le pire des cas — un service qui démarre en ligne avec les
    // réglages du développement.
    let sur_un_hebergeur = lire("DYNO").is_some();
    let mode = match lire("NODE_ENV").as_deref() {
        Some("production") => Mode::Production,
        Some("test") => Mode::Test,
        Some(_) => Mode::Development,
        None if sur_un_hebergeur => Mode::Production,
        None => Mode::Development,
    };

    let mut problemes: Vec<Probleme> = Vec::new();
    let mut avertissements: Vec<Probleme> = Vec::new();

    let jwt_secret = exiger("JWT_SECRET", Some("secret-de-developpement-non-utilisable"), mode, &mut problemes);
    let signing_secret = exiger("MEDIA_SIGNING_SECRET", Some("secret-media-developpement"), mode, &mut problemes);

    let weave_db = lire("WEAVE_DB").map(|v| v.to_lowercase());
    let driver = match weave_db.as_deref() {
        Some("postgres") => Driver::Postgres,
        Some("sqlite") => Driver::Sqlite,
        Some(autre) => {
            return Err(format!(
                "WEAVE_DB doit valoir \"postgres\" ou \"sqlite\" (reçu : {autre})"
            ));
        }
        None if mode.is_production() => Driver::Postgres,
        None => Driver::Sqlite,
    };

    if driver == Driver::Sqlite && mode.is_production() {
        problemes.push(Probleme {
            variable: "WEAVE_DB",
            raison: "vaut « sqlite », réservé au développement".to_string(),
            remede: "définir WEAVE_DB=postgres",
        });
    }

    let db_url = match driver {
        Driver::Postgres => exiger("DATABASE_URL", None, mode, &mut problemes),
        Driver::Sqlite => lire("DATABASE_URL_SQLITE")
            .unwrap_or_else(|| "sqlite://./weave-dev.sqlite?mode=rwc".to_string()),
    };

    let redis_url = exiger("REDIS_URL", Some("redis://127.0.0.1:6379"), mode, &mut problemes);

    let ssl_insecure = lire("DATABASE_SSL_INSECURE").is_some_and(|v| v == "true");
    let tls_insecure = lire("REDIS_TLS_INSECURE").is_some_and(|v| v == "true");

    // Chez Heroku, ces deux réglages ne sont pas un confort : sans eux la base
    // et le cache restent injoignables derrière leurs certificats auto-signés.
    if mode.is_production() && sur_un_hebergeur {
        if !ssl_insecure {
            avertissements.push(Probleme {
                variable: "DATABASE_SSL_INSECURE",
                raison: "la base d'Heroku présente un certificat auto-signé sur son réseau interne"
                    .to_string(),
                remede: "définir DATABASE_SSL_INSECURE=true, sans quoi la base restera injoignable",
            });
        }
        if !tls_insecure {
            avertissements.push(Probleme {
                variable: "REDIS_TLS_INSECURE",
                raison: "le magasin clé-valeur d'Heroku présente lui aussi un certificat auto-signé"
                    .to_string(),
                remede: "définir REDIS_TLS_INSECURE=true, sans quoi le cache restera injoignable",
            });
        }
    }

    let web_origin = lire("PUBLIC_WEB_ORIGIN").unwrap_or_else(|| "http://localhost:5173".to_string());
    let media_base = lire("MEDIA_BASE_URL").unwrap_or_else(|| "http://localhost:3000/media".to_string());

    if mode.is_production() {
        if web_origin.contains("localhost") {
            avertissements.push(Probleme {
                variable: "PUBLIC_WEB_ORIGIN",
                raison: "pointe encore sur localhost".to_string(),
                remede: "définir l'adresse publique du site, sinon les appels depuis le navigateur seront refusés",
            });
        }
        if media_base.contains("localhost") {
            avertissements.push(Probleme {
                variable: "MEDIA_BASE_URL",
                raison: "pointe encore sur localhost".to_string(),
                remede: "définir l'adresse publique des médias, sinon les photos ne s'afficheront pas",
            });
        }
    }

    if !problemes.is_empty() {
        return Err(rapport(
            "Weave ne peut pas démarrer : la configuration est incomplète.",
            &problemes,
        ));
    }

    if !avertissements.is_empty() {
        eprint!(
            "{}",
            rapport(
                "Weave démarre, mais la configuration est incomplète :",
                &avertissements
            )
        );
    }

    let port = lire("PORT")
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000_u16);

    Ok(Env {
        mode,
        port,
        db: Db {
            driver,
            url: db_url,
            ssl_insecure,
        },
        cache: Cache {
            url: redis_url,
            tls_insecure,
        },
        media: Media {
            signing_secret,
            base_url: media_base,
            ttl_url_signee_secondes: entier("MEDIA_URL_TTL_SECONDS", 600),
        },
        apns: Apns {
            key_id: lire("APNS_KEY_ID"),
            team_id: lire("APNS_TEAM_ID"),
            key_path: lire("APNS_KEY_PATH"),
            bundle_id: lire("APNS_BUNDLE_ID").unwrap_or_else(|| "com.weave.app".to_string()),
            environnement: lire("APNS_ENVIRONMENT").unwrap_or_else(|| "sandbox".to_string()),
            configure: lire("APNS_KEY_ID").is_some()
                && lire("APNS_TEAM_ID").is_some()
                && lire("APNS_KEY_PATH").is_some(),
        },
        app_store: AppStore {
            environnement: lire("APPSTORE_ENVIRONMENT").unwrap_or_else(|| "sandbox".to_string()),
            configure: lire("APPSTORE_ISSUER_ID").is_some()
                && lire("APPSTORE_KEY_ID").is_some()
                && lire("APPSTORE_KEY_PATH").is_some(),
        },
        auth: Auth {
            jwt_secret,
            access_ttl_secondes: entier("ACCESS_TOKEN_TTL_SECONDS", 900),
            refresh_ttl_jours: entier("REFRESH_TOKEN_TTL_DAYS", 60),
        },
        web_origin,
        web_dist: lire("WEB_DIST_PATH"),
    })
}

fn rapport(titre: &str, problemes: &[Probleme]) -> String {
    let mut s = format!("\n  {titre}\n\n");
    for p in problemes {
        s.push_str(&format!("  • {} — {}\n      {}\n", p.variable, p.raison, p.remede));
    }
    s.push_str("\n  Sur Heroku : tableau de bord → Settings → Config Vars,\n");
    s.push_str("  ou le workflow « Heroku — configurer les variables » depuis GitHub.\n\n");
    s
}
