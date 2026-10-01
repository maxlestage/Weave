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
fn coquille(langue: Langue, corps: &str, feuille: &str, icone: &str) -> String {
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

    <link rel="apple-touch-icon" href="/apple-touch-icon.png" />
    <link rel="manifest" href="/site.webmanifest" />
    <meta property="og:site_name" content="Weave" />
    <meta property="og:title" content="{partage_titre}" />
    <meta property="og:description" content="{partage_description}" />
    <meta property="og:type" content="website" />
    <meta property="og:locale" content="{og_locale}" />
{alternatives}    <meta property="og:image:width" content="1200" />
    <meta property="og:image:height" content="630" />
    <meta property="og:image:alt" content="{partage_image_alt}" />
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
        application_description = echapper(m.application_description),
        offre_description = echapper(m.offre_description),
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

        let page = coquille(langue, &corps, &feuille, &icone);

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

    rapporter(&pages);
    Ok(())
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
