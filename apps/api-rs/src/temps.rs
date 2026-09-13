//! Calculs d'horaires.
//!
//! L'heure locale n'est pas un détail d'affichage : le quota de demandes
//! expire à minuit dans le fuseau de l'utilisateur, pas dans celui du serveur.
//! Un fuseau mal résolu rendrait ses demandes au mauvais moment.

use chrono::{DateTime, Datelike, Timelike, Utc};
use chrono_tz::Tz;

/// Résout un fuseau nommé. Un fuseau inconnu retombe sur UTC plutôt que
/// d'échouer : mieux vaut un quota qui se réinitialise à la mauvaise heure
/// qu'un compte qui ne peut plus rien demander.
fn fuseau(nom: &str) -> Tz {
    nom.parse().unwrap_or_else(|_| {
        tracing::warn!(fuseau = nom, "fuseau inconnu, UTC retenu");
        chrono_tz::UTC
    })
}

/// Jour local au format AAAA-MM-JJ, pour les clés de cache journalières.
pub fn jour_local(nom_fuseau: &str, a: DateTime<Utc>) -> String {
    a.with_timezone(&fuseau(nom_fuseau))
        .format("%Y-%m-%d")
        .to_string()
}

/// Secondes restantes avant minuit, dans un fuseau donné.
pub fn secondes_avant_minuit(nom_fuseau: &str, depuis: DateTime<Utc>) -> i64 {
    let local = depuis.with_timezone(&fuseau(nom_fuseau));
    (24 - local.hour() as i64) * 3600 - local.minute() as i64 * 60
}

/// Âge en années révolues.
pub fn age_depuis(naissance: DateTime<Utc>, a: DateTime<Utc>) -> i32 {
    let mut age = a.year() - naissance.year();
    let mois = a.month() as i32 - naissance.month() as i32;
    if mois < 0 || (mois == 0 && a.day() < naissance.day()) {
        age -= 1;
    }
    age
}

/// Boîte englobante autour d'un point, pour préfiltrer en SQL sans extension
/// géospatiale : le schéma doit rester identique sur PostgreSQL et SQLite. La
/// distance exacte est recalculée ensuite en mémoire.
pub struct Boite {
    pub lat_min: f64,
    pub lat_max: f64,
    pub lon_min: f64,
    pub lon_max: f64,
}

pub fn boite_englobante(lat: f64, lon: f64, rayon_km: f64) -> Boite {
    let delta_lat = rayon_km / 111.0;
    let delta_lon = rayon_km / (111.0 * lat.to_radians().cos()).max(1.0);
    Boite {
        lat_min: lat - delta_lat,
        lat_max: lat + delta_lat,
        lon_min: lon - delta_lon,
        lon_max: lon + delta_lon,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn instant(iso: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(iso)
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn le_jour_local_suit_le_fuseau_et_non_le_serveur() {
        // 23h30 UTC le 11 septembre, c'est déjà le 12 à Paris (UTC+2 en été).
        let t = instant("2026-09-11T23:30:00Z");
        assert_eq!(jour_local("Europe/Paris", t), "2026-09-12");
        assert_eq!(jour_local("UTC", t), "2026-09-11");
        // Et la veille à New York.
        assert_eq!(jour_local("America/New_York", t), "2026-09-11");
    }

    #[test]
    fn un_fuseau_inconnu_ne_bloque_pas_le_compte() {
        let t = instant("2026-09-11T23:30:00Z");
        assert_eq!(jour_local("Mars/Olympus", t), jour_local("UTC", t));
    }

    #[test]
    fn l_age_ne_compte_pas_l_anniversaire_a_venir() {
        let naissance = Utc.with_ymd_and_hms(2000, 9, 20, 0, 0, 0).unwrap();
        // La veille de ses 26 ans, on en a encore 25.
        assert_eq!(age_depuis(naissance, instant("2026-09-19T12:00:00Z")), 25);
        assert_eq!(age_depuis(naissance, instant("2026-09-20T12:00:00Z")), 26);
    }

    #[test]
    fn le_quota_expire_bien_a_minuit_local() {
        // 22h00 à Paris : il reste deux heures.
        let t = instant("2026-09-11T20:00:00Z");
        assert_eq!(secondes_avant_minuit("Europe/Paris", t), 2 * 3600);
    }
}

/// Rend une date au format que `Date.prototype.toISOString()` produit —
/// millisecondes et « Z », jamais « +00:00 ». Le site et l'application iOS
/// lisent ces champs tels quels : un décalage de format les casserait sans
/// que rien ne le signale côté serveur.
pub fn iso8601(date: DateTime<Utc>) -> String {
    date.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests_iso {
    use super::*;

    #[test]
    fn le_format_est_celui_de_toisostring() {
        let t = DateTime::parse_from_rfc3339("2026-09-12T11:25:31.517Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(iso8601(t), "2026-09-12T11:25:31.517Z");
        assert!(!iso8601(t).contains("+00:00"));
    }
}

/// Distance à vol d'oiseau, en kilomètres arrondis.
///
/// Les coordonnées sont déjà arrondies au kilomètre en base : Weave n'expose
/// jamais de position précise, et cette distance n'est donc qu'un ordre de
/// grandeur — c'est voulu.
pub fn distance_km(a_lat: f64, a_lon: f64, b_lat: f64, b_lon: f64) -> f64 {
    const RAYON_TERRE_KM: f64 = 6371.0;
    let d_lat = (b_lat - a_lat).to_radians();
    let d_lon = (b_lon - a_lon).to_radians();
    let h = (d_lat / 2.0).sin().powi(2)
        + a_lat.to_radians().cos() * b_lat.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
    (2.0 * RAYON_TERRE_KM * h.sqrt().min(1.0).asin()).round()
}

#[cfg(test)]
mod tests_distance {
    use super::*;

    #[test]
    fn la_distance_correspond_aux_reperes_connus() {
        // Lyon → Paris : environ 390 km à vol d'oiseau.
        let d = distance_km(45.75, 4.85, 48.85, 2.35);
        assert!((385.0..=395.0).contains(&d), "Lyon-Paris valait {d} km");
        // Le même point est à zéro, et la distance est symétrique.
        assert_eq!(distance_km(45.75, 4.85, 45.75, 4.85), 0.0);
        assert_eq!(
            distance_km(45.75, 4.85, 48.85, 2.35),
            distance_km(48.85, 2.35, 45.75, 4.85)
        );
    }
}
