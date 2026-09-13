//! L'adresse publique, posée sur les pages au démarrage.
//!
//! ## Pourquoi le problème existe
//!
//! `canonical`, `og:url` et `og:image` exigent des adresses absolues : le
//! protocole Open Graph n'en résout aucune de relative. La construction du
//! site les écrit donc à partir de `SITE.origine`, et les omet quand elle est
//! absente — un partage sans vignette valant mieux qu'un partage attribué à un
//! domaine qui ne répond pas.
//!
//! Or **Heroku ne transmet pas les variables de configuration de
//! l'application aux constructions de conteneur**. L'adresse du site est
//! pourtant connue du déploiement, et depuis le premier jour :
//! `PUBLIC_WEB_ORIGIN`, que l'installation pose à `https://<app>.herokuapp.com`
//! et que l'API relit déjà pour autoriser les appels du navigateur.
//!
//! La construction ne peut pas la connaître ; le processus qui sert les pages,
//! si. L'origine est un fait de déploiement, pas un fait de construction —
//! c'est donc ici qu'elle se pose.
//!
//! ## Ce que cela ne remplace pas
//!
//! `SITE.origine`, renseignée dans `identite.ts`, reste la source : quand elle
//! est là, la construction a déjà tout écrit et ce module ne touche à rien. Il
//! ne sert qu'au cas où elle manque — celui de tout déploiement Heroku qui n'a
//! pas encore été complété à la main.

use std::path::Path;

/// La marque après laquelle les balises s'insèrent. Elle est écrite par la
/// construction sur toutes les pages, accueil comme pages juridiques.
const ANCRE: &str = r#"<meta property="og:site_name" content="Weave" />"#;

/// La preuve que la construction a déjà posé l'adresse : on ne repasse pas
/// derrière elle. `SITE.origine` l'emporte toujours.
const DEJA_POSEE: &str = r#"property="og:url""#;

/// L'adresse canonique relative que la construction écrit sur les pages
/// juridiques faute d'origine — `<link rel="canonical" href="/cgu" />`.
///
/// Elle est valide et se résout contre la page ; elle doit néanmoins être
/// REMPLACÉE et non doublée. Deux `canonical` sur une même page, les moteurs
/// n'en retiennent aucune : la page se retrouve moins bien référencée
/// qu'avant qu'on y touche.
fn canonique_relative(html: &str) -> Option<String> {
    let debut = html.find(r#"<link rel="canonical" href="/"#)?;
    let fin = html[debut..].find("/>")? + debut + 2;
    Some(html[debut..fin].to_string())
}

/// Pose l'adresse publique sur les pages qui n'en ont pas.
///
/// Les échecs ne sont jamais fatals : une page sans vignette se partage encore,
/// un service qui refuse de démarrer ne sert plus rien. Ils sont journalisés.
pub fn poser_l_origine(dist: Option<&str>, origine: &str) {
    let Some(dist) = dist else { return };

    // En développement l'origine vaut `http://localhost:5173` : l'inscrire
    // dans un `og:url` produirait une adresse que personne d'autre ne peut
    // atteindre, ce qui est pire que pas d'adresse du tout.
    if origine.contains("localhost") || !origine.starts_with("https://") {
        return;
    }
    let origine = origine.trim_end_matches('/');

    let racine = Path::new(dist);
    let mut posees = 0;

    // L'accueil, puis chaque page juridique — un répertoire, son index dedans.
    let mut pages = vec![(racine.join("index.html"), String::new())];
    if let Ok(entrees) = std::fs::read_dir(racine) {
        for entree in entrees.flatten() {
            if !entree.path().is_dir() {
                continue;
            }
            let Some(slug) = entree.file_name().to_str().map(str::to_string) else {
                continue;
            };
            pages.push((entree.path().join("index.html"), slug));
        }
    }

    for (chemin, slug) in pages {
        match poser_sur(&chemin, origine, &slug) {
            Ok(true) => posees += 1,
            Ok(false) => {}
            Err(erreur) => {
                tracing::warn!(page = %chemin.display(), erreur = %erreur, "adresse non posée");
            }
        }
    }

    if posees > 0 {
        tracing::info!(
            pages = posees,
            origine,
            "adresse publique posée sur les pages (SITE.origine absente à la construction)"
        );
    }
}

/// Rend `true` si la page a été réécrite.
fn poser_sur(chemin: &Path, origine: &str, slug: &str) -> std::io::Result<bool> {
    let html = std::fs::read_to_string(chemin)?;
    if html.contains(DEJA_POSEE) || !html.contains(ANCRE) {
        return Ok(false);
    }

    let adresse = if slug.is_empty() {
        format!("{origine}/")
    } else {
        format!("{origine}/{slug}")
    };

    let canonique = format!(r#"<link rel="canonical" href="{adresse}" />"#);

    // Les pages juridiques portent déjà une canonique relative : on la
    // remplace. L'accueil n'en a aucune : la sienne s'ajoute avec le reste.
    let (html, canonique_a_poser) = match canonique_relative(&html) {
        Some(ancienne) => (html.replacen(&ancienne, &canonique, 1), String::new()),
        None => (html, format!("{canonique}\n    ")),
    };

    let balises = format!(
        concat!(
            "{canonique_a_poser}",
            "<meta property=\"og:url\" content=\"{adresse}\" />\n",
            "    <meta property=\"og:image\" content=\"{origine}/partage.png\" />\n",
            "    <meta name=\"twitter:card\" content=\"summary_large_image\" />\n",
            "    {ancre}"
        ),
        canonique_a_poser = canonique_a_poser,
        adresse = adresse,
        origine = origine,
        ancre = ANCRE
    );

    std::fs::write(chemin, html.replacen(ANCRE, &balises, 1))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une page telle que la construction la rend sans origine : la marque est
    /// là, les balises absolues ne le sont pas.
    fn page_sans_adresse() -> String {
        format!("<!doctype html>\n<html>\n  <head>\n    {ANCRE}\n  </head>\n</html>\n")
    }

    /// Une page juridique telle que la construction la rend sans origine : la
    /// canonique relative y est, contrairement à l'accueil.
    fn page_juridique_sans_adresse(slug: &str) -> String {
        format!(
            "<!doctype html>\n<html>\n  <head>\n    <link rel=\"canonical\" href=\"/{slug}\" />\n    {ANCRE}\n  </head>\n</html>\n"
        )
    }

    fn dist_de_test(nom: &str) -> std::path::PathBuf {
        let racine = std::env::temp_dir().join(format!("weave-partage-{}-{nom}", std::process::id()));
        let _ = std::fs::remove_dir_all(&racine);
        std::fs::create_dir_all(racine.join("cgu")).expect("répertoire");
        std::fs::write(racine.join("index.html"), page_sans_adresse()).expect("accueil");
        std::fs::write(racine.join("cgu/index.html"), page_juridique_sans_adresse("cgu"))
            .expect("cgu");
        racine
    }

    #[test]
    fn l_adresse_du_deploiement_comble_celle_qui_manque_a_la_construction() {
        let dist = dist_de_test("comble");
        poser_l_origine(Some(&dist.display().to_string()), "https://weave.example");

        let accueil = std::fs::read_to_string(dist.join("index.html")).expect("lecture");
        assert!(accueil.contains(r#"<link rel="canonical" href="https://weave.example/" />"#));
        assert!(accueil.contains(r#"<meta property="og:url" content="https://weave.example/" />"#));
        assert!(accueil
            .contains(r#"<meta property="og:image" content="https://weave.example/partage.png" />"#));
        assert!(accueil.contains(r#"<meta name="twitter:card" content="summary_large_image" />"#));

        // Chaque page juridique porte SA propre adresse : un `og:url` commun
        // ferait que tout partage d'une page renverrait à l'accueil.
        let cgu = std::fs::read_to_string(dist.join("cgu/index.html")).expect("lecture");
        assert!(cgu.contains(r#"<meta property="og:url" content="https://weave.example/cgu" />"#));

        // Une seule canonique, et c'est l'absolue : la relative qu'écrivait la
        // construction est REMPLACÉE. Deux canoniques sur une même page et les
        // moteurs n'en retiennent aucune — la page finirait moins bien
        // référencée qu'avant qu'on y touche.
        assert_eq!(cgu.matches(r#"rel="canonical""#).count(), 1, "deux canoniques : {cgu}");
        assert!(cgu.contains(r#"<link rel="canonical" href="https://weave.example/cgu" />"#));
        assert!(!cgu.contains(r#"href="/cgu""#), "la canonique relative a survécu : {cgu}");
    }

    /// `SITE.origine` l'emporte : on ne repasse pas derrière la construction.
    #[test]
    fn une_adresse_deja_posee_n_est_pas_touchee() {
        let dist = dist_de_test("deja");
        let deja = format!(
            "<head>\n    <meta property=\"og:url\" content=\"https://vrai-domaine.fr/\" />\n    {ANCRE}\n</head>"
        );
        std::fs::write(dist.join("index.html"), &deja).expect("écriture");

        poser_l_origine(Some(&dist.display().to_string()), "https://weave.example");

        let relu = std::fs::read_to_string(dist.join("index.html")).expect("lecture");
        assert_eq!(relu, deja, "la construction fait foi quand elle a parlé");
    }

    /// Une adresse de développement n'a rien à faire dans un `og:url` : elle
    /// désignerait une machine que personne d'autre ne peut atteindre.
    #[test]
    fn une_adresse_locale_n_est_jamais_posee() {
        for origine in ["http://localhost:5173", "http://127.0.0.1:3000", "weave.fr"] {
            let dist = dist_de_test("local");
            poser_l_origine(Some(&dist.display().to_string()), origine);
            let accueil = std::fs::read_to_string(dist.join("index.html")).expect("lecture");
            assert_eq!(accueil, page_sans_adresse(), "« {origine} » a été posée");
        }
    }

    /// Un site absent n'empêche pas le service de démarrer.
    #[test]
    fn un_site_absent_ne_fait_rien_echouer() {
        poser_l_origine(None, "https://weave.example");
        poser_l_origine(Some("/ce/chemin/n/existe/pas"), "https://weave.example");
    }
}
