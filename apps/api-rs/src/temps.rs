//! Calculs d'horaires.
//!
//! L'heure locale n'est pas un détail d'affichage : le quota de demandes
//! expire à minuit dans le fuseau de l'utilisateur, pas dans celui du serveur.
//! Un fuseau mal résolu rendrait ses demandes au mauvais moment.

use chrono::{DateTime, Datelike, TimeZone, Utc};
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
///
/// Le calcul cherche l'instant du prochain minuit local, puis le soustrait.
/// Il partait auparavant de « vingt-quatre moins l'heure locale », ce qui
/// suppose deux choses fausses : qu'un jour dure toujours vingt-quatre heures,
/// et que les secondes ne comptent pas.
///
/// La seconde erreur ne prêtait pas à conséquence — la clé survivait une
/// minute de trop à un jour qui ne la relit plus. La première, si : le jour du
/// retour à l'heure d'hiver dure vingt-cinq heures, la clé du quota expirait
/// une heure avant la fin du jour, et le compteur de demandes repartait de
/// zéro. Le plafond journalier — ce qui distingue les paliers, et donc ce qui
/// se paie — se levait une fois par an, dans chaque fuseau qui change d'heure.
pub fn secondes_avant_minuit(nom_fuseau: &str, depuis: DateTime<Utc>) -> i64 {
    let zone = fuseau(nom_fuseau);
    let local = depuis.with_timezone(&zone);
    let Some(demain) = local.date_naive().succ_opt() else {
        // Fin de l'ère représentable : mieux vaut un jour entier qu'un zéro,
        // qui ferait expirer la clé sur-le-champ et lèverait le plafond.
        return 24 * 3600;
    };

    // Minuit n'existe pas partout tous les jours : à Santiago, le passage à
    // l'heure d'été se fait à minuit et la journée commence à 01h00. Quand
    // l'heure demandée n'existe pas, on prend la première qui existe.
    let instant = (0..=3).find_map(|heure| {
        let candidat = demain.and_hms_opt(heure, 0, 0)?;
        zone.from_local_datetime(&candidat).earliest()
    });

    match instant {
        Some(minuit) => (minuit.with_timezone(&Utc) - depuis).num_seconds().max(0),
        None => 24 * 3600,
    }
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

    /// Les secondes comptent aussi.
    #[test]
    fn les_secondes_ne_sont_pas_perdues() {
        // 23h59m30 à Paris : il reste trente secondes, pas soixante.
        let t = instant("2026-09-11T21:59:30Z");
        assert_eq!(secondes_avant_minuit("Europe/Paris", t), 30);
    }

    /// Le jour du retour à l'heure d'hiver dure vingt-cinq heures.
    ///
    /// La formule partait de « vingt-quatre moins l'heure locale ». Le
    /// 25 octobre 2026, à Paris, 02h00 sonne deux fois : de minuit à minuit il
    /// s'écoule vingt-cinq heures, et la clé du quota expirait une heure avant
    /// la fin du jour. Le compteur repartait de zéro, et le plafond de demandes
    /// — ce qui distingue les paliers — se levait une fois par an.
    #[test]
    fn le_jour_du_changement_d_heure_dure_ce_qu_il_dure() {
        // 00h30 heure locale, le jour du retour à l'heure d'hiver.
        let t = instant("2026-10-24T22:30:00Z");
        assert_eq!(
            secondes_avant_minuit("Europe/Paris", t),
            24 * 3600 + 30 * 60,
            "le 25 octobre 2026 dure vingt-cinq heures à Paris"
        );

        // Et le jour du passage à l'heure d'été, vingt-trois.
        let t = instant("2026-03-28T23:30:00Z");
        assert_eq!(
            secondes_avant_minuit("Europe/Paris", t),
            22 * 3600 + 30 * 60,
            "le 29 mars 2026 dure vingt-trois heures à Paris"
        );
    }

    /// Il existe des fuseaux où minuit n'existe pas certains jours.
    ///
    /// À Santiago, le passage à l'heure d'été se fait à minuit : la journée
    /// commence à 01h00. Demander « l'instant de minuit » n'y rend rien, et une
    /// résolution naïve paniquerait ou rendrait zéro — un quota expirant
    /// sur-le-champ, donc illimité.
    #[test]
    fn un_minuit_qui_n_existe_pas_ne_fait_pas_tomber_le_quota() {
        let t = instant("2026-09-05T20:00:00Z");
        let reste = secondes_avant_minuit("America/Santiago", t);
        assert!(
            reste > 3600,
            "il reste plus d'une heure avant la fin du jour, or {reste} s"
        );
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
