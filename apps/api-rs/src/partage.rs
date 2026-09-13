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

/// Les langues du site, dans l'ordre, la première étant celle de la racine.
///
/// Elles doublent `apps/web/src/langues.ts`, et un test de contrat vérifie que
/// les deux listes disent la même chose : une langue ajoutée d'un seul côté
/// sortirait une page d'accueil sans `hreflang`, ou un `hreflang` désignant
/// une page qui n'existe pas.
const LANGUES: [&str; 3] = ["fr", "en", "es"];

/// Le chemin de l'accueil d'une langue : « / » pour la première, « /en/ » pour
/// les autres. La barre oblique finale compte — sans elle la canonique
/// désignerait une adresse qui redirige, et un moteur suit la redirection pour
/// ne garder que la destination.
fn chemin_de_langue(langue: &str) -> String {
    if langue == LANGUES[0] {
        "/".to_string()
    } else {
        format!("/{langue}/")
    }
}

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

    // Les accueils d'abord — « / », puis « /en/ » et « /es/ ». Ce sont des
    // traductions l'une de l'autre : chacune porte les `hreflang` des trois.
    let mut pages: Vec<(std::path::PathBuf, Page)> = LANGUES
        .iter()
        .map(|langue| {
            let dossier = if *langue == LANGUES[0] {
                racine.to_path_buf()
            } else {
                racine.join(langue)
            };
            (dossier.join("index.html"), Page::Accueil(langue))
        })
        .collect();

    // Puis chaque page juridique — un répertoire, son index dedans. Les
    // répertoires de langue sont déjà traités : les reprendre ici les
    // désignerait comme « /en », sans barre oblique finale.
    if let Ok(entrees) = std::fs::read_dir(racine) {
        for entree in entrees.flatten() {
            if !entree.path().is_dir() {
                continue;
            }
            let Some(slug) = entree.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if LANGUES.contains(&slug.as_str()) {
                continue;
            }
            pages.push((entree.path().join("index.html"), Page::Juridique(slug)));
        }
    }

    for (chemin, page) in pages {
        match poser_sur(&chemin, origine, &page) {
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

/// Ce qu'une page est, et donc l'adresse qu'elle porte.
enum Page {
    /// Un des trois accueils. Ils sont traductions l'un de l'autre, et le
    /// disent par des `hreflang`.
    Accueil(&'static str),
    /// Une page juridique — CGU, CGV, confidentialité. Elles n'existent qu'en
    /// français, et ne portent donc aucune alternative.
    Juridique(String),
}

/// Rend `true` si la page a été réécrite.
fn poser_sur(chemin: &Path, origine: &str, page: &Page) -> std::io::Result<bool> {
    let html = std::fs::read_to_string(chemin)?;
    if html.contains(DEJA_POSEE) || !html.contains(ANCRE) {
        return Ok(false);
    }

    let adresse = match page {
        Page::Accueil(langue) => format!("{origine}{}", chemin_de_langue(langue)),
        Page::Juridique(slug) => format!("{origine}/{slug}"),
    };

    let canonique = format!(r#"<link rel="canonical" href="{adresse}" />"#);

    /*
     * `hreflang` : ce qui dit à un moteur que les trois accueils sont la même
     * page dans trois langues.
     *
     * La construction les écrit quand `SITE.origine` est connue, et les omet
     * sinon — ces balises n'acceptent que des adresses absolues. Sur un
     * déploiement Heroku, `SITE.origine` manque toujours : sans ce qui suit,
     * les trois pages partiraient en concurrence l'une de l'autre, et un
     * moteur n'en garderait souvent qu'une. Elles sont pourtant ce pour quoi
     * les traductions existent.
     *
     * `x-default` désigne la version servie à qui ne demande aucune de ces
     * langues — la française, celle de la racine.
     */
    let alternatives = match page {
        Page::Juridique(_) => String::new(),
        Page::Accueil(_) => {
            let mut lignes = String::new();
            for langue in LANGUES {
                lignes.push_str(&format!(
                    "<link rel=\"alternate\" hreflang=\"{langue}\" href=\"{origine}{}\" />\n    ",
                    chemin_de_langue(langue)
                ));
            }
            lignes.push_str(&format!(
                "<link rel=\"alternate\" hreflang=\"x-default\" href=\"{origine}{}\" />\n    ",
                chemin_de_langue(LANGUES[0])
            ));
            lignes
        }
    };

    // Les pages juridiques portent déjà une canonique relative : on la
    // remplace. L'accueil n'en a aucune : la sienne s'ajoute avec le reste.
    let (html, canonique_a_poser) = match canonique_relative(&html) {
        Some(ancienne) => (html.replacen(&ancienne, &canonique, 1), String::new()),
        None => (html, format!("{canonique}\n    ")),
    };

    let balises = format!(
        concat!(
            "{canonique_a_poser}",
            "{alternatives}",
            "<meta property=\"og:url\" content=\"{adresse}\" />\n",
            "    <meta property=\"og:image\" content=\"{origine}/partage.png\" />\n",
            "    <meta name=\"twitter:card\" content=\"summary_large_image\" />\n",
            "    {ancre}"
        ),
        canonique_a_poser = canonique_a_poser,
        alternatives = alternatives,
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
        let racine =
            std::env::temp_dir().join(format!("weave-partage-{}-{nom}", std::process::id()));
        let _ = std::fs::remove_dir_all(&racine);
        std::fs::create_dir_all(racine.join("cgu")).expect("répertoire");
        std::fs::write(racine.join("index.html"), page_sans_adresse()).expect("accueil");
        std::fs::write(
            racine.join("cgu/index.html"),
            page_juridique_sans_adresse("cgu"),
        )
        .expect("cgu");

        // Les accueils traduits, tels que la construction les rend sans
        // origine : un répertoire par langue, un index dedans.
        for langue in &LANGUES[1..] {
            std::fs::create_dir_all(racine.join(langue)).expect("répertoire de langue");
            std::fs::write(racine.join(langue).join("index.html"), page_sans_adresse())
                .expect("accueil traduit");
        }
        racine
    }

    #[test]
    fn l_adresse_du_deploiement_comble_celle_qui_manque_a_la_construction() {
        let dist = dist_de_test("comble");
        poser_l_origine(Some(&dist.display().to_string()), "https://weave.example");

        let accueil = std::fs::read_to_string(dist.join("index.html")).expect("lecture");
        assert!(accueil.contains(r#"<link rel="canonical" href="https://weave.example/" />"#));
        assert!(accueil.contains(r#"<meta property="og:url" content="https://weave.example/" />"#));
        assert!(accueil.contains(
            r#"<meta property="og:image" content="https://weave.example/partage.png" />"#
        ));
        assert!(accueil.contains(r#"<meta name="twitter:card" content="summary_large_image" />"#));

        // Chaque page juridique porte SA propre adresse : un `og:url` commun
        // ferait que tout partage d'une page renverrait à l'accueil.
        let cgu = std::fs::read_to_string(dist.join("cgu/index.html")).expect("lecture");
        assert!(cgu.contains(r#"<meta property="og:url" content="https://weave.example/cgu" />"#));

        // Une seule canonique, et c'est l'absolue : la relative qu'écrivait la
        // construction est REMPLACÉE. Deux canoniques sur une même page et les
        // moteurs n'en retiennent aucune — la page finirait moins bien
        // référencée qu'avant qu'on y touche.
        assert_eq!(
            cgu.matches(r#"rel="canonical""#).count(),
            1,
            "deux canoniques : {cgu}"
        );
        assert!(cgu.contains(r#"<link rel="canonical" href="https://weave.example/cgu" />"#));
        assert!(
            !cgu.contains(r#"href="/cgu""#),
            "la canonique relative a survécu : {cgu}"
        );
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

    /// Chaque accueil traduit porte SON adresse, barre oblique finale comprise.
    ///
    /// Sans elle — « /en » au lieu de « /en/ » — la canonique désignerait une
    /// adresse qui redirige : le service ajoute la barre oblique manquante, et
    /// un moteur ne retient que la destination d'une redirection. La page
    /// aurait déclaré comme adresse véritable celle qu'elle n'est pas.
    #[test]
    fn chaque_accueil_traduit_porte_sa_propre_adresse() {
        let dist = dist_de_test("traduits");
        poser_l_origine(Some(&dist.display().to_string()), "https://weave.example");

        for langue in &LANGUES[1..] {
            let page = std::fs::read_to_string(dist.join(langue).join("index.html"))
                .expect("lecture de l'accueil traduit");
            let attendue = format!("https://weave.example/{langue}/");
            assert!(
                page.contains(&format!(r#"<link rel="canonical" href="{attendue}" />"#)),
                "« {langue} » : canonique attendue {attendue}, page : {page}"
            );
            assert!(
                page.contains(&format!(
                    r#"<meta property="og:url" content="{attendue}" />"#
                )),
                "« {langue} » : og:url attendu {attendue}, page : {page}"
            );
        }
    }

    /// Les trois accueils se déclarent traductions l'un de l'autre.
    ///
    /// Sans `hreflang`, un moteur prend trois pages au contenu voisin pour
    /// trois pages concurrentes et n'en garde souvent qu'une — ce qui annule
    /// l'intérêt même d'avoir traduit. La construction écrit ces balises quand
    /// `SITE.origine` est connue ; sur Heroku elle ne l'est jamais, et c'est
    /// donc ici qu'elles se posent.
    #[test]
    fn les_accueils_se_declarent_traductions_les_uns_des_autres() {
        let dist = dist_de_test("hreflang");
        poser_l_origine(Some(&dist.display().to_string()), "https://weave.example");

        for langue in LANGUES {
            let fichier = if langue == LANGUES[0] {
                dist.join("index.html")
            } else {
                dist.join(langue).join("index.html")
            };
            let page = std::fs::read_to_string(&fichier).expect("lecture");

            for autre in LANGUES {
                let chemin = chemin_de_langue(autre);
                let attendue = format!(
                    r#"<link rel="alternate" hreflang="{autre}" href="https://weave.example{chemin}" />"#
                );
                assert!(
                    page.contains(&attendue),
                    "« {langue} » ne désigne pas « {autre} » : {page}"
                );
            }

            assert!(
                page.contains(
                    r#"<link rel="alternate" hreflang="x-default" href="https://weave.example/" />"#
                ),
                "« {langue} » n'a pas de version par défaut : {page}"
            );
        }
    }

    /// Une page juridique n'a pas de traduction, et ne doit pas en annoncer.
    ///
    /// Les CGU, les CGV et la politique de confidentialité ne sont publiées
    /// qu'en français. Un `hreflang="en"` sur ces pages désignerait une adresse
    /// qui n'existe pas — et une alternative qui répond 404 fait ignorer tout
    /// le groupe par les moteurs, les accueils compris.
    #[test]
    fn une_page_juridique_n_annonce_aucune_traduction() {
        let dist = dist_de_test("juridique-sans-traduction");
        poser_l_origine(Some(&dist.display().to_string()), "https://weave.example");

        let cgu = std::fs::read_to_string(dist.join("cgu/index.html")).expect("lecture");
        assert!(
            !cgu.contains("hreflang"),
            "les CGU annoncent une traduction : {cgu}"
        );
    }

    /// Les langues connues ici sont celles du site.
    ///
    /// Cette liste double `apps/web/src/langues.ts`. Une langue ajoutée d'un
    /// seul côté ne casse rien à la construction : elle sort une page
    /// d'accueil sans `hreflang`, ou un `hreflang` désignant une page qui
    /// n'existe pas — invisible jusqu'à ce qu'un moteur cesse d'indexer.
    #[test]
    fn les_langues_sont_celles_du_site() {
        let chemin =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/web/src/langues.ts");
        let source = std::fs::read_to_string(&chemin)
            .unwrap_or_else(|e| panic!("langues.ts illisible en {} : {e}", chemin.display()));

        let debut = source
            .find("export const LANGUES = [")
            .expect("LANGUES a disparu de langues.ts");
        let fin = source[debut..].find(']').expect("LANGUES sans fin") + debut;

        // Les décalages rendus par `match_indices` valent DANS LA TRANCHE, et
        // non dans le fichier : les rapporter à `source` lirait ailleurs.
        let declaration = &source[debut..fin];
        let declarees: Vec<String> = declaration
            .match_indices('"')
            .collect::<Vec<_>>()
            .chunks(2)
            .filter_map(|paire| match paire {
                [(ouvre, _), (ferme, _)] => Some(declaration[ouvre + 1..*ferme].to_string()),
                _ => None,
            })
            .collect();

        assert_eq!(declarees, LANGUES, "les deux listes de langues ont divergé");

        // La première est celle de la racine, des deux côtés.
        assert!(
            source.contains(&format!("LANGUE_PAR_DEFAUT: Langue = \"{}\"", LANGUES[0])),
            "la langue par défaut du site n'est pas « {} »",
            LANGUES[0]
        );
    }

    /// Un site absent n'empêche pas le service de démarrer.
    #[test]
    fn un_site_absent_ne_fait_rien_echouer() {
        poser_l_origine(None, "https://weave.example");
        poser_l_origine(Some("/ce/chemin/n/existe/pas"), "https://weave.example");
    }
}
