//! Rend le site en HTML, à la construction, et mesure ce qu'il coûte.
//!
//! ## Le HTML est hydratable, et cela se paie
//!
//! `ServerRenderer` sème des marqueurs de position que le wasm relit pour
//! reprendre l'arbre là où le serveur l'a laissé. Sans eux, l'hydratation
//! échoue et Yew reconstruit tout — la page clignote au premier affichage.
//!
//! Ils sont donc nécessaires, et ils alourdissent chaque page. La mesure plus
//! bas les compte.

use std::io::Write;
use weave_site::chaine;
use weave_site::contrat;
use weave_site::langues::{LANGUES, Langue};
use weave_site::metadonnees::metadonnees;
use weave_site::racine::{Accueil, ProprietesLangue};
use yew::ServerRenderer;

/// Le dossier de sortie, à côté de celui de `apps/web` tant que les deux
/// chaînes coexistent : on compare les deux rendus plutôt que d'en croire un.
///
/// Il est résolu depuis le dossier de la crate et NON depuis le dossier
/// courant. Lancé par `cargo run --manifest-path apps/site/Cargo.toml` depuis
/// la racine du dépôt — ce que fait `verifier.sh` — un chemin relatif au
/// dossier courant aurait semé le rendu à la racine, et la comparaison aurait
/// annoncé que Yew ne rend rien. Le rendu doit arriver au même endroit quelle
/// que soit la façon de le lancer.
fn sortie(sous: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(sous)
}

/// Le fichier qui démarre l'hydratation.
///
/// Un fichier à part, et non un script en ligne : `<script type="module" src>`
/// est exactement ce que fait la chaîne React, et une balise vide se compare —
/// un script en ligne, non, son corps étant du JavaScript que rien n'a à
/// relire. Surtout, un fichier qu'on écrit est un fichier dont on peut
/// vérifier la PRÉSENCE, et une balise qui pointe vers rien donne une page qui
/// ne s'hydratera jamais.
const DEMARREUR: &str = "demarrer.js";

/// `weave_site.js` est produit par `wasm-bindgen`, et exporte sa fonction
/// d'initialisation par défaut. L'appeler lance la fonction marquée
/// `#[wasm_bindgen(start)]`, donc l'hydratation.
const DEMARREUR_CONTENU: &str = "import demarrer from \"/weave_site.js\";\ndemarrer();\n";

/// La coquille HTML.
///
/// Elle reprend celle qu'écrit `apps/web/build.ts`, balise pour balise : titre,
/// description, Open Graph et données structurées viennent de
/// `metadonnees.rs`, lui-même engendré depuis la source React. Les deux
/// chaînes doivent dire la même chose aux moteurs — un titre français sur
/// `/en/` ferait indexer la page anglaise comme française.
///
/// Le script est en `module`, donc différé : il ne bloque pas l'affichage. La
/// page est lisible avant qu'un octet de wasm n'arrive — c'est tout l'intérêt
/// du rendu serveur, et l'hydratation ne vient qu'après.
fn coquille(
    langue: Langue,
    corps: &str,
    feuille: &str,
    icone: &str,
    origine: Option<&str>,
) -> String {
    let m = metadonnees(langue);

    // Les autres langues, dans l'ordre où `LANGUES` les déclare : c'est celui
    // que la chaîne React emploie, et l'ordre des balises se compare.
    let alternatives: String = LANGUES
        .iter()
        .filter(|&&autre| autre != langue)
        .map(|autre| {
            format!(
                "    <meta property=\"og:locale:alternate\" content=\"{}\" />\n",
                metadonnees(*autre).og_locale,
            )
        })
        .collect();

    format!(
        // `r##` et non `r#` : le HTML contient `content="#16121f"`, et la
        // séquence `"#` refermerait une chaîne brute à un seul dièse.
        r##"<!doctype html>
<html lang="{code}">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover" />
    <meta name="theme-color" content="#16121f" media="(prefers-color-scheme: dark)" />
    <meta name="theme-color" content="#fffdf9" media="(prefers-color-scheme: light)" />
    <title>{titre}</title>
    <meta name="description" content="{description}" />
{canonique}{alternatives_de_langue}    <link rel="apple-touch-icon" href="/apple-touch-icon.png" />
    <link rel="manifest" href="/site.webmanifest" />
    <meta property="og:site_name" content="Weave" />
    <meta property="og:title" content="{partage_titre}" />
    <meta property="og:description" content="{partage_description}" />
    <meta property="og:type" content="website" />
    <meta property="og:locale" content="{og_locale}" />
{alternatives}    <meta property="og:image:width" content="1200" />
    <meta property="og:image:height" content="630" />
    <meta property="og:image:alt" content="{partage_image_alt}" />{absolues}
    <link rel="icon" href="/{icone}" type="image/svg+xml" />
    <link rel="stylesheet" href="/{feuille}" />
    <script type="application/ld+json">
      {{
        "@context": "https://schema.org",
        "@type": "MobileApplication",
        "name": "Weave",
        "applicationCategory": "SocialNetworkingApplication",
        "operatingSystem": "iOS, watchOS",
        "inLanguage": "{bcp47}",
        "description": "{application_description}",
        "offers": {{
          "@type": "Offer",
          "price": "0",
          "priceCurrency": "EUR",
          "description": "{offre_description}"
        }}
      }}
    </script>
  </head>
  <body>
    <div id="racine">{corps}</div>
    <script type="module" src="/{DEMARREUR}"></script>
  </body>
</html>
"##,
        code = langue.code(),
        titre = echapper(m.titre),
        description = echapper(m.description),
        partage_titre = echapper(m.partage_titre),
        partage_description = echapper(m.partage_description),
        partage_image_alt = echapper(m.partage_image_alt),
        og_locale = m.og_locale,
        bcp47 = m.bcp47,
        canonique = chaine::canonique_accueil(origine, langue),
        alternatives_de_langue = chaine::alternatives(origine),
        absolues = chaine::balises_absolues(origine, langue.chemin("").trim_start_matches('/')),
        application_description = echapper(m.application_description),
        offre_description = echapper(m.offre_description),
    )
}

/// La coquille d'une page juridique.
///
/// Elle diffère de celle de l'accueil, et pas par accident :
///
/// * `og:type` vaut `article` et non `website` ;
/// * une adresse canonique est déclarée — ces pages existent à une seule
///   adresse, sans variante de langue ;
/// * aucune donnée structurée, aucun `apple-touch-icon`, aucun manifeste : ce
///   n'est pas l'application qu'on présente, c'est un texte qu'on lit ;
/// * AUCUN SCRIPT. Ces pages n'ont pas d'état, donc rien à hydrater. Leur en
///   envoyer un ferait télécharger le wasm à qui vient lire des CGU.
fn coquille_juridique(
    page: &contrat::PageJuridique,
    corps: &str,
    feuille: &str,
    icone: &str,
    origine: Option<&str>,
) -> String {
    // La canonique d'une page juridique est déclarée MÊME sans origine : le
    // format relatif y est valide et se résout contre l'adresse de la page.
    // Celle de l'accueil, elle, est omise — trois adresses y désignent la même
    // page, et une relative n'y dirait rien de plus que la page elle-même.
    let canonique = match origine {
        Some(origine) => format!("{origine}/{}", page.adresse),
        None => format!("/{}", page.adresse),
    };
    format!(
        r##"<!doctype html>
<html lang="fr">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover" />
    <meta name="theme-color" content="#16121f" media="(prefers-color-scheme: dark)" />
    <meta name="theme-color" content="#fffdf9" media="(prefers-color-scheme: light)" />
    <title>{titre}</title>
    <meta name="description" content="{description}" />
    <link rel="canonical" href="{canonique}" />
    <meta property="og:site_name" content="Weave" />
    <meta property="og:title" content="{titre}" />
    <meta property="og:description" content="{description}" />
    <meta property="og:type" content="article" />
    <meta property="og:locale" content="fr_FR" />{absolues}
    <link rel="icon" href="/{icone}" type="image/svg+xml" />
    <link rel="stylesheet" href="/{feuille}" />
  </head>
  <body>
    <div id="racine">{corps}</div>
  </body>
</html>
"##,
        // « Mentions légales — Weave », et non « Mentions légales » seul.
        //
        // Le nom du site suit le titre du document : c'est ce qui s'affiche dans
        // un onglet et dans un résultat de recherche, où « Mentions légales »
        // seul ne dirait pas de quel site. `documents.ts` ne porte que le titre
        // du document, et la chaîne de construction y ajoute le suffixe — on
        // fait pareil, au même endroit du titre.
        titre = echapper(&format!("{} — Weave", page.titre)),
        description = echapper(page.description),
        absolues = chaine::balises_absolues(origine, page.adresse),
    )
}

/// Échappe un texte destiné à un attribut HTML.
///
/// Les descriptions contiennent des apostrophes et des deux-points ; une seule
/// a besoin d'être échappée pour tenir dans un attribut entre guillemets
/// doubles, mais les quatre autres caractères le sont aussi — un texte de
/// référencement se modifie, et il ne doit pas pouvoir casser la page le jour
/// où quelqu'un y met un chevron.
fn echapper(texte: &str) -> String {
    texte
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Les noms des fichiers d'habillage, tels que la chaîne React les a empreints.
///
/// La feuille de style et l'icône portent une empreinte de leur contenu
/// (`chunk-5jqgjvep.css`), produite par l'empaqueteur de Bun. La chaîne Rust ne
/// sait pas la recalculer : il faudrait refaire le même empaquetage, au même
/// octet.
///
/// Elle les LIT donc dans `apps/web/dist`, le temps que le portage dure. C'est
/// un étai, et il est visible : il disparaît le jour où la chaîne Rust produira
/// elle-même l'habillage. Sans cet étai, les deux coquilles différeraient sur
/// deux lignes qui ne disent rien du portage.
fn habillage() -> std::io::Result<(String, String)> {
    let dossier = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("web")
        .join("dist");

    let trouver = |prefixe: &str, extension: &str| -> std::io::Result<String> {
        let mut noms: Vec<String> = std::fs::read_dir(&dossier)?
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|nom| nom.starts_with(prefixe) && nom.ends_with(extension))
            .collect();
        // Trié : `read_dir` ne promet aucun ordre, et une sortie qui change
        // d'une exécution à l'autre ferait clignoter la comparaison.
        noms.sort();
        match noms.len() {
            1 => Ok(noms.remove(0)),
            0 => Err(std::io::Error::other(format!(
                "aucun fichier « {prefixe}*{extension} » dans apps/web/dist — \
                 construire le site React d'abord : bun run --filter @weave/web build"
            ))),
            n => Err(std::io::Error::other(format!(
                "{n} fichiers « {prefixe}*{extension} » dans apps/web/dist : {noms:?}"
            ))),
        }
    };

    Ok((trouver("chunk-", ".css")?, trouver("favicon-", ".svg")?))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    let racine = sortie("dist-rs");

    // Le dossier de sortie est VIDÉ d'abord, comme le fait `build.ts`.
    //
    // Sans cela, un fichier qui cesse d'être produit survit et continue d'être
    // servi. La comparaison l'a trouvé : une construction lancée avec une
    // origine avait écrit `sitemap.xml`, puis refusé pour mentions légales
    // incomplètes — et le plan du site restait là, annonçant une adresse qui
    // n'était plus la bonne, à travers toutes les constructions suivantes.
    if racine.exists() {
        std::fs::remove_dir_all(&racine)?;
    }
    std::fs::create_dir_all(&racine)?;

    // L'origine d'abord, et son refus avant tout rendu.
    //
    // Écrire huit pages canoniques à une adresse morte, puis s'arrêter, laisse
    // derrière soi un `dist-rs` qu'on pourrait servir. Mieux vaut ne rien
    // produire.
    let origine = chaine::origine();
    if let Some(origine) = &origine
        && let Some(raison) = chaine::pourquoi_inhabitable(origine)
    {
        refuser_l_origine(origine, raison);
    }
    let origine = origine.as_deref();

    let (feuille, icone) = habillage()?;
    let mut pages = Vec::new();

    for langue in LANGUES {
        let corps = ServerRenderer::<Accueil>::with_props(move || ProprietesLangue { langue })
            .render()
            .await;
        // Le MÊME arbre rendu sans marqueurs, pour connaître leur coût réel.
        // Deux rendus plutôt qu'une estimation : la différence est le prix de
        // l'hydratation, et il n'y a pas de raison de le deviner.
        let nu = ServerRenderer::<Accueil>::with_props(move || ProprietesLangue { langue })
            .hydratable(false)
            .render()
            .await;

        let page = coquille(langue, &corps, &feuille, &icone, origine);

        let dossier = racine.join(langue.prefixe().trim_start_matches('/'));
        std::fs::create_dir_all(&dossier)?;
        let chemin = dossier.join("index.html");
        std::fs::write(&chemin, page.as_bytes())?;
        pages.push((
            langue,
            page.len(),
            corps.len() - nu.len(),
            gzip(page.as_bytes()),
        ));
    }

    std::fs::create_dir_all(&racine)?;
    std::fs::write(racine.join(DEMARREUR), DEMARREUR_CONTENU)?;

    // Les pages juridiques, en français seulement. Rendues SANS marqueurs
    // d'hydratation : rien ne les hydratera, et les marqueurs ne seraient que
    // du poids sur des pages déjà denses en texte.
    let mut juridiques = Vec::new();
    for page in contrat::PAGES_JURIDIQUES.iter() {
        let Some(corps) = rendre_page_juridique(page.adresse).await else {
            // Une page encore à porter n'arrête pas la construction : la
            // comparaison la listera comme absente, et c'est le bon message.
            continue;
        };
        let entier = coquille_juridique(page, &corps, &feuille, &icone, origine);
        let dossier = racine.join(page.adresse);
        std::fs::create_dir_all(&dossier)?;
        std::fs::write(dossier.join("index.html"), entier.as_bytes())?;
        juridiques.push((page.adresse, entier.len(), gzip(entier.as_bytes())));
    }
    rapporter_juridiques(&juridiques);

    copier_les_ressources_publiques(&racine)?;
    ecrire_les_fichiers_de_referencement(&racine, origine)?;
    verifier_qu_aucune_adresse_n_est_ecrite_en_dur(&racine, origine)?;
    avertir_des_mentions_incompletes(origine);

    rapporter(&pages);
    Ok(())
}

fn refuser_l_origine(origine: &str, raison: &str) -> ! {
    eprintln!();
    eprintln!("  ✗ SITE_ORIGINE vaut « {origine} ».");
    eprintln!("    {raison}");
    eprintln!();
    eprintln!("    Les pages se déclareraient canoniques à cette adresse, et le plan");
    eprintln!("    du site n'énumérerait qu'elle. Mieux vaut ne rien poser du tout :");
    eprintln!("    les adresses absolues sont alors simplement omises.");
    std::process::exit(1);
}

/// Les fichiers demandés par une adresse FIXE, écrite ailleurs que dans nos
/// pages.
///
/// Un réseau social récupère `partage.png` depuis l'extérieur, iOS demande
/// `apple-touch-icon.png` à la racine, et un navigateur ou un robot demande
/// `favicon.svg` sans avoir lu la moindre balise. Aucune ne doit donc porter
/// d'empreinte — l'adresse partagée serait périmée à la construction suivante.
fn copier_les_ressources_publiques(racine: &std::path::Path) -> std::io::Result<()> {
    let public = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("web")
        .join("public");
    for fichier in [
        "partage.png",
        "apple-touch-icon.png",
        "favicon.svg",
        "site.webmanifest",
    ] {
        std::fs::copy(public.join(fichier), racine.join(fichier))?;
    }
    Ok(())
}

/// `robots.txt` et `sitemap.xml`.
///
/// Sans eux, rien n'indique aux moteurs quelles pages existent : le site n'a
/// pas de liens entrants et ses pages juridiques ne sont atteignables que
/// depuis le pied de page.
fn ecrire_les_fichiers_de_referencement(
    racine: &std::path::Path,
    origine: Option<&str>,
) -> std::io::Result<()> {
    std::fs::write(racine.join("robots.txt"), chaine::robots(origine))?;

    let jour = chrono::Utc::now().format("%Y-%m-%d").to_string();
    match chaine::sitemap(origine, &jour) {
        Some(plan) => std::fs::write(racine.join("sitemap.xml"), plan)?,
        None => {
            println!();
            println!("  ↳ pas de sitemap.xml : SITE_ORIGINE n'est pas posée, et un plan");
            println!("    du site n'accepte que des adresses absolues.");
        }
    }
    Ok(())
}

/// Refuse si une page nomme notre propre adresse en dur.
///
/// L'accueil portait « https://weave.app » dans `canonical`, `og:url` et
/// `og:image` — un domaine qui n'est pas le nôtre. Un lien partagé aurait
/// montré l'image d'un autre site, et `canonical` aurait désigné ce domaine
/// comme l'adresse véritable de nos pages : de quoi remettre à quelqu'un
/// d'autre le référencement du site.
///
/// Le contrôle ne porte QUE sur les balises dont le rôle est de nommer notre
/// adresse. Les liens sortants d'une page — la CNIL, l'assistance d'Apple —
/// sont du contenu : ils désignent autrui, et c'est bien ce qu'on leur demande.
fn verifier_qu_aucune_adresse_n_est_ecrite_en_dur(
    racine: &std::path::Path,
    origine: Option<&str>,
) -> std::io::Result<()> {
    let mut fautes = Vec::new();

    for adresse in chaine::adresses() {
        let chemin = racine.join(&adresse).join("index.html");
        let html = std::fs::read_to_string(&chemin)?;
        for valeur in adresses_declarees(&html) {
            if !valeur.starts_with("http") {
                continue;
            }
            if origine.is_some_and(|o| valeur.starts_with(o)) {
                continue;
            }
            fautes.push(format!("{adresse}index.html : {valeur}"));
        }
    }

    if !fautes.is_empty() {
        eprintln!();
        eprintln!("  ✗ Ces balises nomment notre adresse sans passer par SITE_ORIGINE :");
        for faute in &fautes {
            eprintln!("      {faute}");
        }
        eprintln!("    Une adresse partagée se déduit de l'origine, elle ne s'écrit pas.");
        std::process::exit(1);
    }
    Ok(())
}

/// Les valeurs des balises dont le rôle est de dire « où cette page vit-elle ».
///
/// Une petite lecture à la main plutôt qu'une expression rationnelle : les
/// attributs peuvent arriver dans n'importe quel ordre, et une expression
/// qui en suppose un finit par ne plus rien trouver le jour où on en ajoute
/// un — sans échouer, ce qui est le pire des deux.
fn adresses_declarees(html: &str) -> Vec<String> {
    const PORTEUSES: [&str; 5] = [
        "canonical",
        "alternate",
        "og:url",
        "og:image",
        "twitter:image",
    ];
    let mut trouvees = Vec::new();
    for balise in html.split('<').skip(1) {
        let Some(fin) = balise.find('>') else {
            continue;
        };
        let balise = &balise[..fin];
        if !(balise.starts_with("link ") || balise.starts_with("meta ")) {
            continue;
        }
        let role = attribut(balise, "rel")
            .or_else(|| attribut(balise, "property"))
            .or_else(|| attribut(balise, "name"));
        if !role.is_some_and(|r| PORTEUSES.contains(&r)) {
            continue;
        }
        // `og:image:alt` et `og:image:width` portent un texte et un nombre, pas
        // une adresse : leur rôle complet ne figure pas dans la liste, et
        // c'est ce qui les écarte.
        if let Some(valeur) = attribut(balise, "href").or_else(|| attribut(balise, "content")) {
            trouvees.push(valeur.to_string());
        }
    }
    trouvees
}

fn attribut<'a>(balise: &'a str, nom: &str) -> Option<&'a str> {
    let motif = format!(" {nom}=\"");
    let debut = balise.find(&motif)? + motif.len();
    let reste = &balise[debut..];
    let fin = reste.find('\"')?;
    Some(&reste[..fin])
}

/// Ce qu'il reste à renseigner dans les mentions légales.
///
/// Une construction POUR UN VRAI DOMAINE s'arrête ; une construction locale
/// avertit seulement. La distinction tient à `SITE_ORIGINE` : on ne le pose que
/// pour mettre en ligne. Bloquer toute construction obligerait à remplir un
/// SIREN pour lancer le site en local, ce qui n'a aucun sens — et finirait par
/// faire poser des valeurs bidon pour avancer, c'est-à-dire exactement ce que
/// cette garde existe pour empêcher.
///
/// Mais mettre en ligne SANS elles n'est pas une négligence rattrapable plus
/// tard : l'article 6-III de la LCEN rend ces mentions obligatoires, et les
/// pages s'afficheraient avec « [à compléter : SIREN et RCS] » surligné. Un
/// avertissement dans un journal de construction ne se voit pas ; une page
/// juridique publiée, si.
fn avertir_des_mentions_incompletes(origine: Option<&str>) {
    let manquantes = chaine::valeurs_manquantes();
    if manquantes.is_empty() {
        return;
    }

    if origine.is_some() {
        eprintln!();
        eprintln!(
            "  ✗ {} mentions légales manquent, et le site part pour un vrai domaine.",
            manquantes.len()
        );
        eprintln!("    Renseignez-les dans apps/web/src/pages/identite.ts :");
        for champ in &manquantes {
            eprintln!("      • {champ}");
        }
        eprintln!();
        eprintln!("    L'article 6-III de la LCEN les rend obligatoires. Publiées telles");
        eprintln!("    quelles, elles s'afficheraient surlignées « [à compléter : … ] ».");
        std::process::exit(1);
    }

    println!();
    println!("  ⚠ {} valeurs restent à renseigner", manquantes.len());
    println!("    dans apps/web/src/pages/identite.ts :");
    for champ in &manquantes {
        println!("      • {champ}");
    }
    println!("    Les mentions légales manquantes apparaissent surlignées");
    println!("    sur les pages publiées.");
    println!();
    println!("  ⚠ SITE_ORIGINE n'est pas posée : les liens partagés sortiront nus.");
    println!("    Pas de vignette, pas de titre, pas de description — le protocole");
    println!("    Open Graph ne résout aucune adresse relative. Ni plan du site.");
}

/// Rend le corps d'une page juridique, ou `None` si elle n'est pas portée.
///
/// L'association entre une adresse et son composant se fait ICI, à un seul
/// endroit. Les pages arrivent une à une, et une adresse sans composant doit se
/// voir — pas se deviner.
async fn rendre_page_juridique(adresse: &str) -> Option<String> {
    use weave_site::pages::cgu::Cgu;
    use weave_site::pages::cgv::Cgv;
    use weave_site::pages::confidentialite::Confidentialite;
    use weave_site::pages::mentions_legales::MentionsLegales;
    use weave_site::pages::suppression_compte::SuppressionCompte;
    match adresse {
        "mentions-legales" => Some(
            ServerRenderer::<MentionsLegales>::new()
                .hydratable(false)
                .render()
                .await,
        ),
        "confidentialite" => Some(
            ServerRenderer::<Confidentialite>::new()
                .hydratable(false)
                .render()
                .await,
        ),
        "cgu" => Some(
            ServerRenderer::<Cgu>::new()
                .hydratable(false)
                .render()
                .await,
        ),
        "cgv" => Some(
            ServerRenderer::<Cgv>::new()
                .hydratable(false)
                .render()
                .await,
        ),
        "suppression-compte" => Some(
            ServerRenderer::<SuppressionCompte>::new()
                .hydratable(false)
                .render()
                .await,
        ),
        _ => None,
    }
}

fn rapporter_juridiques(pages: &[(&str, usize, usize)]) {
    if pages.is_empty() {
        return;
    }
    println!("\n  Pages juridiques (français seulement, sans hydratation)");
    for (adresse, octets, compresse) in pages {
        println!("    {adresse:<22} {octets:>7} octets   gzip {compresse:>6}");
    }
    let manquantes = contrat::PAGES_JURIDIQUES.len() - pages.len();
    if manquantes > 0 {
        println!("    {manquantes} page(s) encore à porter.");
    }
}

/// Ce que le visiteur télécharge réellement.
///
/// La chaîne React mesurait le poids compressé à chaque construction, « sur un
/// site consulté surtout en 4G », pour rendre visible toute dérive au lieu de
/// la découvrir en production. La même mesure vaut ici, et elle vaut d'autant
/// plus que l'hydratation ajoute un binaire au téléchargement.
fn rapporter(pages: &[(Langue, usize, usize, usize)]) {
    println!("\n  HTML rendu");
    for (langue, page, marqueurs, compresse) in pages {
        println!(
            "    {:<4} {:>7} octets   gzip {:>6}   dont {:>4} de marqueurs d'hydratation",
            langue.code(),
            page,
            compresse,
            marqueurs,
        );
    }

    // Le wasm n'est pas produit par ce binaire : il vient d'une autre
    // compilation, pour une autre cible. On le mesure s'il est là, et on le
    // dit s'il manque — une mesure partielle présentée comme complète vaut
    // moins que pas de mesure.
    println!("\n  Ce que le navigateur télécharge en plus");
    let mut total = 0usize;
    let mut complet = true;
    for fichier in ["dist-wasm/weave_site_bg.wasm", "dist-wasm/weave_site.js"] {
        match std::fs::read(sortie(fichier)) {
            Ok(octets) => {
                let compresse = gzip(&octets);
                total += compresse;
                println!(
                    "    {:<28} gzip {:>6}",
                    fichier.rsplit('/').next().unwrap_or(fichier),
                    compresse
                );
            }
            Err(_) => {
                complet = false;
                println!("    {fichier} — absent, non mesuré");
            }
        }
    }
    if complet {
        println!("    {:<28} gzip {:>6}", "total", total);
        println!(
            "\n  Pour mémoire, le paquet React qu'il remplace : gzip {}.",
            85_237
        );
        println!("  Le wasm est plus lourd. Il est AUSSI chargé après l'affichage,");
        println!("  qui ne dépend que du HTML ci-dessus.");
    }
}

fn gzip(octets: &[u8]) -> usize {
    use std::process::{Command, Stdio};
    let Ok(mut enfant) = Command::new("gzip")
        .args(["-9", "-c"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
    else {
        return 0;
    };
    if let Some(entree) = enfant.stdin.as_mut() {
        let _ = entree.write_all(octets);
    }
    enfant
        .wait_with_output()
        .map(|s| s.stdout.len())
        .unwrap_or(0)
}
