//! Le site vitrine, en Rust.
//!
//! Les composants vivent ici, et servent aux DEUX rendus : le binaire natif
//! `rendre` en fait du HTML à la construction, et le même code compilé en wasm
//! vient l'hydrater dans le navigateur.
//!
//! C'est le point de l'affaire : un seul arbre, décrit une fois. Un rendu
//! serveur qui diverge de son hydratation produit une page qui se réécrit sous
//! les yeux du visiteur au premier affichage — et comme les deux rendus
//! viennent d'ici, cela ne peut pas arriver.

pub mod composants;
pub mod contrat;
pub mod langues;
pub mod racine;
pub mod sections;

#[cfg(feature = "hydration")]
mod hydratation {
    //! Le point d'entrée du navigateur.
    //!
    //! `hydrate` et non `render` : le HTML est déjà là, rendu à la
    //! construction. `render` le jetterait pour le rebâtir à l'identique — la
    //! page clignoterait, et le travail de la construction serait perdu.
    //!
    //! C'est la même leçon que `main.tsx` avait apprise du temps de React, et
    //! elle vaut mot pour mot ici.

    use crate::langues::Langue;
    use crate::racine::{Accueil, ProprietesLangue};
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(start)]
    pub fn demarrer() {
        console_error_panic_hook::set_once();

        let racine = web_sys::window()
            .and_then(|f| f.document())
            .and_then(|d| d.get_element_by_id("racine"));
        let Some(racine) = racine else {
            // Pas de racine : il n'y a rien à hydrater, et le dire vaut mieux
            // que de paniquer sur une page qui s'affiche très bien sans nous.
            web_sys::console::warn_1(&"Weave : élément racine introuvable.".into());
            return;
        };

        // La langue vient de l'ADRESSE, et de rien d'autre.
        //
        // `navigator.language` paraîtrait plus attentionné, mais il donnerait
        // un arbre différent de celui que la construction a rendu à cette
        // adresse : l'hydratation échouerait et Yew reconstruirait tout. Un
        // visiteur qui veut une autre langue suit le lien du choix de langue
        // — et l'adresse suit.
        let chemin = web_sys::window()
            .and_then(|f| f.location().pathname().ok())
            .unwrap_or_else(|| "/".to_string());
        let langue = Langue::du_chemin(&chemin);

        yew::Renderer::<Accueil>::with_root_and_props(racine, ProprietesLangue { langue })
            .hydrate();
    }
}
