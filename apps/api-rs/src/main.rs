//! Point d'entrée de l'API Weave.
//!
//! Pile : Rust + Axum + SeaORM + Redis.

mod alerte;
mod apns;
mod auth;
mod cache;
mod console;
mod console_cli;
mod crypto;
mod db;
mod droits;
mod error;
mod langue;
mod limitation;
mod live_activity;
mod messages;
mod migrations;
mod partage;
mod purge;
mod rappels;
mod routes;
mod storekit;
mod temps;

mod env;
#[cfg(test)]
mod tests;

// Les 18 entités sont engendrées d'un bloc depuis la base ; celles qu'aucune
// route ne consomme encore signaleraient du code mort à chaque compilation.
#[allow(dead_code)]
mod entities;

use axum::{
    Json, Router,
    extract::{Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::get,
};
use redis::aio::ConnectionManager;
use sea_orm::{ConnectionTrait, DatabaseConnection};
use serde_json::json;
use std::{sync::Arc, time::Duration};
use tower_http::cors::{AllowOrigin, CorsLayer};

#[derive(Clone)]
struct AppState {
    db: DatabaseConnection,
    cache: ConnectionManager,
    config: Arc<env::Env>,
    /// Partagé : le jeton d'autorisation APNs vit dans le client, et en forger
    /// un par notification coûterait une signature ES256 à chaque fois.
    apns: Arc<apns::ClientApns>,
    /// La racine à laquelle doit mener la chaîne d'une transaction StoreKit.
    ///
    /// Toujours celle d'Apple en production — `main` ne pose rien d'autre, et
    /// aucune variable d'environnement ne la change : une racine configurable
    /// depuis l'extérieur serait une porte ouverte sur ce qui protège les
    /// achats. Seuls les tests en posent une autre, pour pouvoir éprouver le
    /// chemin complet avec de vraies signatures.
    racine_storekit: Arc<Vec<u8>>,
}

/// Choisit le fournisseur cryptographique de rustls, avant tout usage de TLS.
///
/// Trois dépendances tirent rustls — `redis` pour Heroku Redis, `sqlx` pour
/// PostgreSQL, `reqwest` pour APNs — et elles n'activent pas le même
/// fournisseur : `aws-lc-rs` d'un côté, `ring` de l'autre. rustls voit les deux
/// features actives, refuse de trancher à notre place, et **panique** au
/// premier handshake :
///
/// ```text
/// Could not automatically determine the process-level CryptoProvider
/// from Rustls crate features.
/// ```
///
/// Rien ne le révèle en développement : en local, Redis et PostgreSQL se
/// joignent en clair, aucun handshake n'a lieu et le code ne s'exécute jamais.
/// Sur Heroku, Redis est en `rediss://` — le dyno s'arrêtait donc au démarrage,
/// sur un code 101 que rien ne relie à la configuration.
///
/// L'appel doit précéder la première connexion chiffrée, d'où sa place en tête
/// de `main`. Il échoue seulement si un fournisseur est déjà installé, ce qui
/// n'a rien d'un problème : l'objectif est atteint.
fn installer_fournisseur_tls() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    installer_fournisseur_tls();
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt().with_target(false).json().init();

    let config = match env::charger() {
        Ok(c) => c,
        Err(rapport) => {
            // La configuration dit elle-même ce qui manque, et le remède de
            // chaque ligne. Rien à ajouter ici.
            eprint!("{rapport}");
            std::process::exit(1);
        }
    };

    let db = db::connecter(&config.db).await?;

    // `weave-api migrate` : la phase de publication d'Heroku l'appelle avant
    // que la nouvelle version ne reçoive du trafic. Si elle échoue, la version
    // précédente reste en ligne — d'où un processus séparé, qui ne démarre ni
    // le serveur ni le cache.
    if std::env::args().nth(1).as_deref() == Some("migrate") {
        let posees = migrations::appliquer(&db).await?;
        if posees.is_empty() {
            println!("  • aucune migration à appliquer");
        } else {
            for nom in posees {
                println!("  • appliquée : {nom}");
            }
        }
        return Ok(());
    }

    // `weave-api purge` : même logique que `migrate` — un processus séparé,
    // qui ne démarre ni serveur ni cache, et qu'un planificateur appelle.
    if std::env::args().nth(1).as_deref() == Some("purge") {
        let bilan = purge::executer(&db).await?;
        println!("  • messages effacés  : {}", bilan.messages_effaces);
        println!("  • activités effacées : {}", bilan.activites_effacees);
        println!("  • comptes effacés   : {}", bilan.comptes_effaces);
        println!("  • traces effacées   : {}", bilan.traces_effacees);
        println!("  • consentements     : {}", bilan.consentements_effaces);
        if bilan.comptes_differes > 0 {
            println!(
                "  • comptes différés : {} (signalement encore ouvert)",
                bilan.comptes_differes
            );
        }
        return Ok(());
    }

    // `weave-api console …` : la modération.
    //
    // Elle a besoin du cache, contrairement à `migrate` et `purge` : le résumé
    // d'identité d'un compte y vit un quart d'heure, et une suspension qui
    // n'invalide pas ce résumé ne prendrait effet qu'à son expiration.
    if std::env::args().nth(1).as_deref() == Some("console") {
        let cache = cache::connecter(&config.cache).await?;
        let arguments: Vec<String> = std::env::args().skip(2).collect();
        return console_cli::executer(&db, &cache, &arguments).await;
    }

    let cache = cache::connecter(&config.cache).await?;

    let origine = if config.is_production() {
        AllowOrigin::exact(config.web_origin.parse()?)
    } else {
        AllowOrigin::any()
    };

    // L'adresse publique, posée sur les pages si la construction ne l'avait
    // pas. Heroku ne transmet pas les variables de configuration aux
    // constructions de conteneur : le site sortait donc sans `og:url` ni
    // `og:image`, et tout lien partagé sans vignette ni titre — alors que
    // l'adresse est connue du déploiement depuis le premier jour.
    partage::poser_l_origine(config.web_dist.as_deref(), &config.web_origin);

    let port = config.port;
    let driver = config.db.driver.as_str();

    // Dire pourquoi les achats seraient refusés, plutôt que de laisser chercher.
    //
    // La vérification est écrite, et ce message ne part donc plus. Il reste
    // pour le jour où quelqu'un retirerait le vérificateur : poser les
    // identifiants App Store est le geste par lequel on croit activer les
    // achats, et sans cette ligne rien ne relierait leur refus à sa cause.
    if config.is_production()
        && config.app_store.configure
        && !routes::billing::VERIFICATION_JWS_IMPLEMENTEE
    {
        tracing::warn!(
            "App Store configuré, mais la vérification des transactions n'est pas \
             en service : les achats sont refusés."
        );
    }

    let state = AppState {
        db,
        cache,
        config: Arc::new(config),
        apns: Arc::new(apns::ClientApns::new()),
        // La racine d'Apple, et rien d'autre.
        racine_storekit: Arc::new(storekit::RACINE_APPLE.to_vec()),
    };

    // La purge tourne depuis le service, faute de planificateur externe. Un
    // verrou dans le cache garantit un seul passage par jour, quel que soit le
    // nombre de dynos.
    purge::planifier(state.clone());
    rappels::planifier(state.clone());

    let app = construire_routeur(state).layer(CorsLayer::new().allow_origin(origine));

    let ecoute = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!(port, database = driver, "API Weave démarrée");

    // `into_make_service_with_connect_info` est requis par les routes qui
    // limitent le débit par adresse : sans lui, l'extracteur `ConnectInfo`
    // échoue à l'exécution, pas à la compilation.
    axum::serve(
        ecoute,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(arret_demande())
    .await?;

    Ok(())
}

/// Assemble toutes les routes du service.
///
/// Extrait de `main` pour que les tests puissent monter le service entier —
/// et non chaque gestionnaire isolément. Un gestionnaire juste derrière un
/// routage faux ne rend toujours pas le bon service.
fn construire_routeur(state: AppState) -> Router {
    let vitrine = state.config.web_dist.clone();
    let api = Router::new()
        .route("/health", get(health))
        .merge(routes::auth::routes())
        .merge(routes::me::routes())
        .merge(routes::plans::routes())
        .merge(routes::requests::routes())
        .merge(routes::conversations::routes())
        .merge(routes::consentements::routes())
        .merge(routes::moderation::routes())
        .merge(routes::devices::routes())
        .merge(routes::export::routes())
        .merge(routes::media::routes())
        .merge(routes::fil::routes())
        .merge(routes::billing::routes())
        .merge(routes::bilan::routes())
        .merge(routes::verification::routes())
        .with_state(state)
        // L'API compose vraiment ses messages dans la langue demandée : elle
        // est donc la seule à pouvoir l'annoncer. La couche est posée ICI, et
        // non autour de tout, parce que le recours vers le site est ajouté
        // ensuite — une page se rend dans SA langue, pas dans celle qu'on
        // demande, et l'étiqueter d'après la requête la décrivait à l'envers.
        .layer(axum::middleware::from_fn(langue::annoncer_la_langue));

    // La langue de la requête, elle, se pose autour de tout : le site n'en a
    // pas besoin, mais l'y faire entrer ne coûte rien et évite d'avoir deux
    // endroits où la lire.
    monter_vitrine(api, vitrine.as_deref())
        .layer(axum::middleware::from_fn(langue::poser_la_langue))
}

/// Monte le site vitrine sous les routes de l'API.
///
/// Le site est statique et peu visité : lui dédier un second dyno doublerait
/// la facture sans rien apporter. Il passe en recours, jamais devant l'API —
/// un fichier nommé `health` dans `dist` ne doit pas éteindre la sonde.
///
/// En développement il n'y a pas de `dist` : `WEB_DIST_PATH` n'est pas défini
/// et aucune route n'est ajoutée. Un site absent là où on l'attendait est
/// signalé, mais n'empêche pas l'API de démarrer : l'application iOS en
/// dépend, et elle n'a que faire de la vitrine.
fn monter_vitrine(routeur: Router, chemin: Option<&str>) -> Router {
    let Some(chemin) = chemin else {
        return routeur;
    };

    if !std::path::Path::new(chemin).join("index.html").is_file() {
        tracing::warn!(chemin, "site vitrine introuvable, l'API démarre sans lui");
        return routeur;
    }

    let fichiers = tower_http::services::ServeDir::new(chemin)
        // Sans cela, `/` — un répertoire — rend une 404 : c'est l'adresse du
        // service elle-même qui serait vide.
        .append_index_html_on_directories(true);

    let vitrine = Router::new()
        .fallback_service(fichiers)
        .layer(axum::middleware::from_fn(servir_vitrine));

    routeur.fallback_service(vitrine)
}

/// Ce que le recours ajoute autour des fichiers : la méthode, puis le cache.
///
/// La construction appose une empreinte au nom des scripts, des styles et des
/// images : leur contenu ne change jamais sous un même nom, ils se gardent
/// indéfiniment. `index.html`, lui, garde les noms des autres : le mettre en
/// cache une heure servirait l'ancien site une heure après chaque publication.
async fn servir_vitrine(mut requete: Request, suite: Next) -> Response {
    // Un POST sur une adresse d'API mal orthographiée arrive ici, et le
    // service de fichiers répondrait « méthode interdite » : un contresens,
    // qui laisse croire que la ressource existe. Elle n'existe pas.
    if !matches!(
        *requete.method(),
        axum::http::Method::GET | axum::http::Method::HEAD
    ) {
        return StatusCode::NOT_FOUND.into_response();
    }

    ajouter_barre_oblique(&mut requete);

    let chemin = requete.uri().path().to_string();
    let mut reponse = suite.run(requete).await;
    if reponse.status().is_success() {
        // La langue de la page vient de son ADRESSE, pas de la requête.
        // « /en/ » est anglaise pour tout le monde, y compris pour qui
        // préfère le français ; les pages juridiques n'existent qu'en
        // français et le disent, y compris à un anglophone.
        langue::poser_l_entete(&mut reponse, langue::langue_du_chemin(&chemin));

        // Le cache se décide sur le type de contenu, pas sur la forme de
        // l'adresse. Une page rendue à « /cgv » n'a ni barre oblique finale
        // ni extension : la déduire du chemin la faisait garder un an comme
        // une image, et un texte juridique corrigé serait resté invisible
        // tout ce temps.
        let html = reponse
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|valeur| valeur.to_str().ok())
            .is_some_and(|valeur| valeur.starts_with("text/html"));

        // « Un an, immuable » ne vaut que pour un fichier dont le NOM change
        // avec le contenu. Quatre fichiers vivent à une adresse fixe, parce
        // que c'est leur raison d'être : `partage.png` est récupéré par les
        // réseaux sociaux, `apple-touch-icon.png` par iOS, `favicon.svg` par
        // les navigateurs, `site.webmanifest` par l'écran d'accueil. Tous sont
        // demandés à une adresse écrite ailleurs que dans nos pages, et aucun
        // ne peut donc porter d'empreinte.
        //
        // Ils prenaient l'en-tête d'un an quand même. Changer l'image de
        // partage ou l'icône n'aurait rien changé pour personne pendant un an,
        // et il n'existe aucun moyen de le forcer : l'adresse ne peut pas
        // bouger, c'est tout l'intérêt.
        let cache = if html {
            "no-cache"
        } else if porte_une_empreinte(&chemin) {
            "public, max-age=31536000, immutable"
        } else {
            // Un jour : assez pour que l'image de partage ne soit pas
            // rechargée à chaque aperçu, assez peu pour qu'une correction
            // finisse par arriver.
            "public, max-age=86400"
        };

        reponse
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
    }
    reponse
}

/// Le nom de ce fichier change-t-il avec son contenu ?
///
/// La construction appose une empreinte avant l'extension — `chunk-qw9gfct6.js`,
/// `favicon-g0tg8e02.svg`. Un tel fichier ne change jamais sous le même nom :
/// il se garde indéfiniment.
///
/// ## Ce qui compte comme empreinte
///
/// Un segment final en base 36, d'au moins six caractères, **et contenant au
/// moins un chiffre**. La longueur n'est pas figée à celle qu'écrit Bun
/// aujourd'hui : la pinner ferait retomber tous les fichiers en cache court le
/// jour où le bundler en changerait, silencieusement.
///
/// Le chiffre exigé est ce qui sépare une empreinte d'un mot. `photo-couverture`
/// finirait sinon gardé un an sur une adresse que personne ne peut changer ;
/// `chunk-qw9gfct6` et `favicon-g0tg8e02` en portent tous deux.
///
/// ## Le doute profite à l'adresse fixe
///
/// Tout le reste est traité comme fixe, y compris un fichier qu'on ajouterait
/// demain sans y penser. C'est le sens sûr : au pire un fichier immuable est
/// redemandé une fois par jour ; au mieux on évite de figer un an quelque chose
/// qu'on voudra corriger — et qu'on ne POURRA pas corriger, l'adresse étant
/// écrite chez les réseaux sociaux et sur des écrans d'accueil.
fn porte_une_empreinte(chemin: &str) -> bool {
    let nom = chemin.rsplit('/').next().unwrap_or_default();
    let Some((tige, _)) = nom.rsplit_once('.') else {
        return false;
    };
    let Some((_, empreinte)) = tige.rsplit_once('-') else {
        return false;
    };
    empreinte.len() >= 6
        && empreinte
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && empreinte.chars().any(|c| c.is_ascii_digit())
}

/// Rend « /cgv » à la place de « /cgv/ », sans détour par une redirection.
///
/// Chaque page juridique est construite en `cgv/index.html`. Le service de
/// fichiers y voit un répertoire et répond une 307 vers « /cgv/ » — or c'est
/// « /cgv » que le pied de page met en lien, que le plan du site déclare, et
/// que l'URL canonique désigne. Chaque visite payait donc un aller-retour, et
/// l'adresse annoncée aux moteurs n'était pas celle qui répondait.
///
/// La barre oblique est ajoutée avant le service de fichiers plutôt que
/// renvoyée au navigateur. Une adresse qui porte une extension est un fichier
/// et n'est pas touchée ; une adresse inexistante reste une 404, le répertoire
/// n'existant pas davantage que le fichier.
fn ajouter_barre_oblique(requete: &mut Request) {
    let chemin = requete.uri().path();
    if chemin.ends_with('/') || chemin.rsplit('/').next().is_none_or(|f| f.contains('.')) {
        return;
    }

    let mut parties = requete.uri().clone().into_parts();
    let requete_et_suite = match parties.path_and_query.as_ref().and_then(|p| p.query()) {
        Some(requete) => format!("{chemin}/?{requete}"),
        None => format!("{chemin}/"),
    };
    let Ok(chemin_et_requete) = requete_et_suite.parse() else {
        return;
    };
    parties.path_and_query = Some(chemin_et_requete);
    if let Ok(uri) = axum::http::Uri::from_parts(parties) {
        *requete.uri_mut() = uri;
    }
}

/// Sonde de santé.
///
/// Les deux sondes sont bornées : une dépendance qui ne répond pas doit être
/// rapportée comme telle, jamais faire attendre la réponse.
async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let sonde_db = async {
        matches!(
            tokio::time::timeout(
                cache::PROBE_TIMEOUT,
                state.db.execute_unprepared("SELECT 1"),
            )
            .await,
            Ok(Ok(_))
        )
    };

    let (db_ok, cache_ok) = tokio::join!(sonde_db, cache::ping(&state.cache));
    let sain = db_ok && cache_ok;

    let corps = json!({
        "status": if sain { "ok" } else { "degraded" },
        "database": { "driver": state.config.db.driver.as_str(), "ok": db_ok },
        // Le cache n'est pas un confort dans Weave : le quota de demandes n'y
        // vit qu'ici, et sans lui l'invariant central ne tient plus.
        "cache": { "ok": cache_ok, "required": true },
        "version": "0.1.0",
    });

    let code = if sain {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (code, Json(corps))
}

async fn arret_demande() {
    let ctrl_c = async { tokio::signal::ctrl_c().await.ok() };
    #[cfg(unix)]
    let sigterm = async {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
                Some(())
            }
            Err(_) => None,
        }
    };
    #[cfg(not(unix))]
    let sigterm = std::future::pending::<Option<()>>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = sigterm => {},
    }
    tracing::info!("Arrêt demandé");
    // Laisse aux requêtes en vol le temps de se terminer.
    tokio::time::sleep(Duration::from_millis(50)).await;
}
