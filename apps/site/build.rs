//! L'année du droit d'auteur, figée à la construction.
//!
//! Le pied de page porte « © 2026 Weave ». La version React la calculait au
//! rendu, ce qui donnait l'année de la construction au serveur et celle de la
//! consultation au client — deux valeurs différentes au passage d'une année,
//! donc un écart d'hydratation, qu'il fallait taire avec
//! `suppressHydrationWarning`.
//!
//! Ici elle est figée à la compilation, et les deux rendus en portent la même.
//! C'est aussi la bonne valeur sur le fond : l'année qui figure dans une
//! mention de droit d'auteur est celle de la publication, pas celle où le
//! visiteur passe. Et `SystemTime::now()` n'existe de toute façon pas sur
//! `wasm32-unknown-unknown` : le composant partagé ne POURRAIT pas la lire.

fn main() {
    let annee = chrono::Utc::now().format("%Y").to_string();
    println!("cargo::rustc-env=WEAVE_ANNEE={annee}");
    // Sans cela, l'année resterait celle de la première compilation tant que
    // rien d'autre ne change dans la crate.
    println!("cargo::rerun-if-changed=build.rs");
}
