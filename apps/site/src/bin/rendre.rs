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

/// La coquille HTML.
///
/// Le script est en `module` et en `defer` implicite : il ne bloque pas
/// l'affichage. La page est lisible avant qu'un octet de wasm n'arrive — c'est
/// tout l'intérêt du rendu serveur, et l'hydratation ne vient qu'après.
fn coquille(langue: Langue, corps: &str) -> String {
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
    <title>Weave ‣</title>
    <meta property="og:locale" content="{locale}" />
    <link rel="stylesheet" href="/style.css" />
  </head>
  <body>
    <div id="racine">{corps}</div>
    <script type="module">
      import demarrer from "/weave_site.js";
      demarrer();
    </script>
  </body>
</html>
"##,
        code = langue.code(),
        locale = langue.locale(),
    )
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    let racine = sortie("dist-rs");
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

        let page = coquille(langue, &corps);

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
