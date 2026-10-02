//! La chaîne de construction : l'origine du site, et les gardes qui refusent
//! une mise en ligne mal formée.
//!
//! Ce module ne vit que dans la compilation NATIVE (`ssr`). Il lit des
//! variables d'environnement et écrit des fichiers ; rien de tout cela n'a de
//! sens dans un wasm servi au navigateur, et l'y faire entrer alourdirait le
//! paquet d'un code qui ne pourrait pas s'exécuter.

use crate::identite::{VALEURS_A_RENSEIGNER, est_a_completer};
use crate::langues::{LANGUE_PAR_DEFAUT, LANGUES, Langue};

/// L'adresse publique du site, ou `None`.
///
/// Elle valait « https://weave.app » en dur du côté React — un domaine qui ne
/// répond pas. Toutes les pages juridiques se déclaraient donc canoniques à une
/// adresse morte, et le plan du site n'énumérait que des URL injoignables : un
/// moteur qui suit ces indications RETIRE les pages de son index plutôt que de
/// les y mettre. Mieux vaut ne rien déclarer que de désigner le vide.
///
/// La barre oblique finale est retirée : toutes les adresses se composent par
/// concaténation, et « https://x.fr//cgu » n'est pas « https://x.fr/cgu ».
pub fn origine() -> Option<String> {
    let brute = std::env::var("SITE_ORIGINE").ok()?;
    let brute = brute.trim();
    if brute.is_empty() || est_a_completer(brute) {
        return None;
    }
    Some(brute.trim_end_matches('/').to_string())
}

/// Pourquoi une origine posée est inhabitable, s'elle l'est.
///
/// Ne rien poser est permis : les adresses absolues sont alors omises, les
/// liens partagés sortent nus, et un avertissement le dit. Poser une adresse
/// FAUSSE est autre chose — les pages se déclarent canoniques à un endroit qui
/// ne répond pas. C'est pire que de n'avoir rien dit.
///
/// `weave.app` est le domaine de remplacement de ce dépôt, et il ne répond pas.
/// Si ce domaine devient un jour le vôtre, c'est cette fonction qu'il faut
/// changer — ainsi que la garde correspondante dans `ios-testflight.yml`.
pub fn pourquoi_inhabitable(origine: &str) -> Option<&'static str> {
    if !origine.starts_with("https://") {
        return Some("Ce n'est pas une adresse https.");
    }
    // L'hôte, et non la chaîne entière : « https://exemple.fr/?x=weave.app »
    // n'est pas le domaine de remplacement, et « https://a.weave.app » l'est.
    let hote = origine
        .trim_start_matches("https://")
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("");
    if hote == "weave.app" || hote.ends_with(".weave.app") {
        return Some("C'est le domaine de remplacement du dépôt, et il ne répond pas.");
    }
    None
}

/// Les champs d'`identite.ts` qui restent à renseigner.
pub fn valeurs_manquantes() -> Vec<&'static str> {
    VALEURS_A_RENSEIGNER
        .iter()
        .filter(|(_, valeur)| est_a_completer(valeur))
        .map(|(chemin, _)| *chemin)
        .collect()
}

/// Les adresses du site, dans l'ordre du plan : les trois accueils, puis les
/// pages juridiques.
pub fn adresses() -> Vec<String> {
    LANGUES
        .iter()
        .map(|langue| langue.chemin("").trim_start_matches('/').to_string())
        .chain(
            crate::contrat::PAGES_JURIDIQUES
                .iter()
                .map(|page| page.adresse.to_string()),
        )
        .collect()
}

/// `robots.txt`, toujours écrit.
///
/// Il autorise l'exploration, et c'est le premier fichier qu'un moteur
/// demande. Mais la ligne `Sitemap:` exige une adresse absolue — sans origine,
/// elle est omise plutôt que de renvoyer le moteur vers un hôte qui ne répond
/// pas. Le plan reste trouvable à la racine, où les moteurs le cherchent
/// d'eux-mêmes.
pub fn robots(origine: Option<&str>) -> String {
    let mut lignes = vec!["User-agent: *".to_string(), "Allow: /".to_string()];
    if let Some(origine) = origine {
        lignes.push(String::new());
        lignes.push(format!("Sitemap: {origine}/sitemap.xml"));
    }
    lignes.push(String::new());
    lignes.join("\n")
}

/// Le plan du site, ou `None` sans origine connue.
///
/// La balise `<loc>` n'accepte que des URL absolues. Un plan qui n'énumère que
/// des adresses mortes ne fait pas indexer les pages, il les fait retirer.
pub fn sitemap(origine: Option<&str>, jour: &str) -> Option<String> {
    let origine = origine?;
    let mut lignes = vec![
        r#"<?xml version="1.0" encoding="UTF-8"?>"#.to_string(),
        r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#.to_string(),
    ];
    for chemin in adresses() {
        lignes.push("  <url>".to_string());
        lignes.push(format!("    <loc>{origine}/{chemin}</loc>"));
        lignes.push(format!("    <lastmod>{jour}</lastmod>"));
        lignes.push("  </url>".to_string());
    }
    lignes.push("</urlset>".to_string());
    lignes.push(String::new());
    Some(lignes.join("\n"))
}

/// Les balises qui n'ont de sens qu'absolues : `og:url` et `og:image`.
///
/// Le protocole Open Graph impose des URL absolues — une adresse relative n'y
/// est pas seulement mal vue, elle n'est pas résolue. Sans origine connue, ces
/// balises sont donc omises : un partage sans vignette vaut mieux qu'un partage
/// attribué à un domaine qui ne répond pas.
pub fn balises_absolues(origine: Option<&str>, adresse: &str) -> String {
    let Some(origine) = origine else {
        return String::new();
    };
    format!(
        "\n    <meta property=\"og:url\" content=\"{origine}/{adresse}\" />\n    \
         <meta property=\"og:image\" content=\"{origine}/partage.png\" />\n    \
         <meta name=\"twitter:card\" content=\"summary_large_image\" />"
    )
}

/// L'adresse canonique d'une page, et rien sans origine.
///
/// Sans origine, pas de canonique sur l'accueil : une adresse relative ne
/// dirait rien de plus que la page elle-même, et une adresse inventée dirait
/// quelque chose de faux. Les pages juridiques, elles, en portent une relative
/// — le format y est valide et se résout contre l'adresse de la page.
pub fn canonique_accueil(origine: Option<&str>, langue: Langue) -> String {
    match origine {
        Some(origine) => format!(
            "    <link rel=\"canonical\" href=\"{origine}{}\" />\n",
            langue.chemin("")
        ),
        None => String::new(),
    }
}

/// Ce qui dit à un moteur que les trois accueils sont la même page, dans trois
/// langues.
///
/// Sans ces balises, il les prend pour trois pages distinctes au contenu
/// voisin, et n'en garde souvent qu'une. Elles exigent des adresses absolues :
/// sans origine connue, elles sont omises plutôt qu'écrites contre un hôte qui
/// ne répond pas. `x-default` désigne la version servie à qui ne demande aucune
/// de ces langues.
pub fn alternatives(origine: Option<&str>) -> String {
    let Some(origine) = origine else {
        return String::new();
    };
    let mut lignes: Vec<String> = LANGUES
        .iter()
        .map(|autre| {
            format!(
                "    <link rel=\"alternate\" hreflang=\"{}\" href=\"{origine}{}\" />",
                autre.code(),
                autre.chemin("")
            )
        })
        .collect();
    lignes.push(format!(
        "    <link rel=\"alternate\" hreflang=\"x-default\" href=\"{origine}{}\" />",
        LANGUE_PAR_DEFAUT.chemin("")
    ));
    format!("{}\n", lignes.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l_origine_perd_sa_barre_oblique_finale() {
        // « https://x.fr/ » plus « /cgu » donnerait « https://x.fr//cgu », une
        // adresse différente de la page réelle — donc une canonique fausse.
        unsafe { std::env::set_var("SITE_ORIGINE", "https://exemple.fr/") };
        assert_eq!(origine().as_deref(), Some("https://exemple.fr"));
        unsafe { std::env::remove_var("SITE_ORIGINE") };
    }

    #[test]
    fn une_origine_vide_ou_a_completer_ne_compte_pas() {
        for valeur in ["", "   ", "[à compléter : adresse publique du site]"] {
            unsafe { std::env::set_var("SITE_ORIGINE", valeur) };
            assert_eq!(origine(), None, "« {valeur} » devrait valoir None");
        }
        unsafe { std::env::remove_var("SITE_ORIGINE") };
    }

    #[test]
    fn le_domaine_de_remplacement_est_refuse_sous_toutes_ses_formes() {
        assert!(pourquoi_inhabitable("https://weave.app").is_some());
        assert!(pourquoi_inhabitable("https://www.weave.app").is_some());
        assert!(pourquoi_inhabitable("https://weave.app/cgu").is_some());
    }

    #[test]
    fn une_adresse_qui_mentionne_le_domaine_ailleurs_passe() {
        // La garde porte sur l'HÔTE. Un chemin ou un paramètre qui contient
        // « weave.app » n'est pas le domaine de remplacement, et refuser une
        // adresse valable pousserait à contourner la garde.
        assert_eq!(pourquoi_inhabitable("https://exemple.fr/weave.app"), None);
        assert_eq!(pourquoi_inhabitable("https://monweave.app.fr"), None);
    }

    #[test]
    fn une_adresse_non_https_est_refusee() {
        assert!(pourquoi_inhabitable("http://exemple.fr").is_some());
        assert_eq!(pourquoi_inhabitable("https://exemple.fr"), None);
    }

    #[test]
    fn les_quinze_valeurs_manquent_encore() {
        // Le jour où l'éditeur les renseigne, ce test change — et c'est le
        // bon moment pour s'en apercevoir.
        assert_eq!(valeurs_manquantes().len(), 15);
        assert!(valeurs_manquantes().contains(&"EDITEUR.immatriculation"));
    }

    #[test]
    fn robots_n_annonce_un_plan_que_s_il_y_en_a_un() {
        assert!(!robots(None).contains("Sitemap"));
        assert!(
            robots(Some("https://exemple.fr")).contains("Sitemap: https://exemple.fr/sitemap.xml")
        );
        // L'exploration est autorisée dans les deux cas : c'est le premier
        // fichier qu'un moteur demande, et son absence se lit comme un refus.
        for texte in [robots(None), robots(Some("https://exemple.fr"))] {
            assert!(texte.contains("User-agent: *"));
            assert!(texte.contains("Allow: /"));
        }
    }

    #[test]
    fn le_plan_du_site_enumere_les_huit_pages() {
        let plan =
            sitemap(Some("https://exemple.fr"), "2026-10-01").expect("un plan avec une origine");
        assert_eq!(plan.matches("<loc>").count(), 8);
        // Les trois accueils y figurent chacun : ils ont des adresses
        // distinctes, et `hreflang` dit qu'ils sont la même page — il ne les
        // remplace pas dans le plan.
        assert!(plan.contains("<loc>https://exemple.fr/</loc>"));
        assert!(plan.contains("<loc>https://exemple.fr/en/</loc>"));
        assert!(plan.contains("<loc>https://exemple.fr/es/</loc>"));
        assert!(plan.contains("<loc>https://exemple.fr/cgu</loc>"));
    }

    #[test]
    fn sans_origine_il_n_y_a_pas_de_plan_du_site() {
        assert_eq!(sitemap(None, "2026-10-01"), None);
    }

    #[test]
    fn les_balises_absolues_et_les_alternatives_s_effacent_sans_origine() {
        assert_eq!(balises_absolues(None, "cgu"), "");
        assert_eq!(alternatives(None), "");
        assert_eq!(canonique_accueil(None, Langue::Fr), "");
    }

    #[test]
    fn les_alternatives_couvrent_les_trois_langues_et_le_defaut() {
        let html = alternatives(Some("https://exemple.fr"));
        for code in ["fr", "en", "es", "x-default"] {
            assert!(
                html.contains(&format!("hreflang=\"{code}\"")),
                "« {code} » manque"
            );
        }
        // Quatre balises : trois langues plus `x-default`, qui répète le
        // français. Une de moins voudrait dire qu'une langue ne se déclare pas.
        assert_eq!(html.matches("rel=\"alternate\"").count(), 4);
    }

    #[test]
    fn la_canonique_de_l_accueil_suit_la_langue() {
        let origine = Some("https://exemple.fr");
        assert!(canonique_accueil(origine, Langue::Fr).contains("href=\"https://exemple.fr/\""));
        assert!(canonique_accueil(origine, Langue::En).contains("href=\"https://exemple.fr/en/\""));
    }
}
