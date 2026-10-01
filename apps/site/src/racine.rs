//! L'arbre de l'accueil, partagé par le rendu serveur et l'hydratation.

use crate::langues::{LANGUES, Langue};
use crate::sections::deroule::Deroule;
use crate::sections::offres::Offres;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ProprietesLangue {
    pub langue: Langue,
}

/// Le choix de langue : de vrais liens vers de vraies adresses, et non un
/// réglage conservé dans le navigateur. Une page traduite doit pouvoir se
/// partager, se mettre en signet, et s'indexer — un moteur n'appuie sur aucun
/// bouton.
#[function_component]
pub fn ChoixDeLangue(p: &ProprietesLangue) -> Html {
    html! {
        <nav aria-label="Langue" class="flex gap-3 text-sm">
            { for LANGUES.iter().map(|&l| {
                let courante = l == p.langue;
                html! {
                    <a
                        href={l.chemin("")}
                        hreflang={l.code()}
                        lang={l.code()}
                        aria-current={courante.then_some("page")}
                        class={classes!("underline-offset-4", courante.then_some("font-semibold"))}
                    >
                        { l.nom() }
                    </a>
                }
            }) }
        </nav>
    }
}

/// Le menu de téléphone — la SEULE chose interactive du site.
///
/// Il porte un état, et c'est pour lui que l'hydratation existe. Le replier
/// dès qu'on navigue demande d'écouter `hashchange`, ce qu'un `<details>` ne
/// sait pas faire.
#[function_component]
pub fn MenuTelephone() -> Html {
    let ouvert = use_state(|| false);

    // Le menu se referme dès qu'on navigue : sur téléphone, il occupe l'écran.
    {
        let ouvert = ouvert.clone();
        use_effect_with(*ouvert, move |est_ouvert| {
            let nettoyer: Box<dyn FnOnce()> = if *est_ouvert {
                #[cfg(feature = "hydration")]
                {
                    use wasm_bindgen::prelude::*;
                    let ouvert = ouvert.clone();
                    let fermer = Closure::<dyn Fn()>::new(move || ouvert.set(false));
                    if let Some(fenetre) = web_sys::window() {
                        let _ = fenetre.add_event_listener_with_callback(
                            "hashchange",
                            fermer.as_ref().unchecked_ref(),
                        );
                    }
                    Box::new(move || {
                        if let Some(fenetre) = web_sys::window() {
                            let _ = fenetre.remove_event_listener_with_callback(
                                "hashchange",
                                fermer.as_ref().unchecked_ref(),
                            );
                        }
                        drop(fermer);
                    })
                }
                #[cfg(not(feature = "hydration"))]
                Box::new(|| {})
            } else {
                Box::new(|| {})
            };
            nettoyer
        });
    }

    let basculer = {
        let ouvert = ouvert.clone();
        Callback::from(move |_| ouvert.set(!*ouvert))
    };

    html! {
        <>
            <button
                type="button"
                class="-mr-2 p-2 sm:hidden"
                aria-expanded={if *ouvert { "true" } else { "false" }}
                aria-controls="menu-telephone"
                onclick={basculer}
            >
                <span class="sr-only">
                    { if *ouvert { "Fermer le menu" } else { "Ouvrir le menu" } }
                </span>
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                    if *ouvert {
                        <path
                            d="M6 6l12 12M18 6L6 18"
                            stroke="currentColor"
                            stroke-width="1.8"
                            stroke-linecap="round"
                        />
                    } else {
                        <path
                            d="M4 7h16M4 12h16M4 17h16"
                            stroke="currentColor"
                            stroke-width="1.8"
                            stroke-linecap="round"
                        />
                    }
                </svg>
            </button>
            if *ouvert {
                <div id="menu-telephone" class="sm:hidden">
                    <ul class="px-5 py-2">
                        <li><a href="#principe" class="block py-3 text-base">{ "Le principe" }</a></li>
                        <li><a href="#deroule" class="block py-3 text-base">{ "Comment ça se passe" }</a></li>
                        <li><a href="#offres" class="block py-3 text-base">{ "Les offres" }</a></li>
                    </ul>
                </div>
            }
        </>
    }
}

#[function_component]
pub fn Accueil(p: &ProprietesLangue) -> Html {
    html! {
        <>
            <header class="sticky top-0 z-50 flex items-center justify-between px-5 py-3">
                <span class="font-semibold">{ "Weave \u{2023}" }</span>
                <MenuTelephone />
            </header>
            <main>
                <Deroule langue={p.langue} />
                <Offres langue={p.langue} />
            </main>
            <footer class="px-5 py-10 sm:px-8">
                <ChoixDeLangue langue={p.langue} />
            </footer>
        </>
    }
}
