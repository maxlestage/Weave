//! L'en-tête : la marque, la navigation, le choix de langue, le menu de
//! téléphone.
//!
//! C'est la seule partie du site qui porte un état, et c'est pour elle que
//! l'hydratation existe : le menu de téléphone s'ouvre, se referme, et se
//! referme AUSSI dès qu'on navigue — ce qu'un `<details>` ne sait pas faire.

use crate::langues::{LANGUES, Traduit};
use crate::racine::ProprietesLangue;
use yew::prelude::*;

struct Lien {
    ancre: &'static str,
    texte: &'static str,
}

const LIENS: Traduit<&[Lien]> = Traduit([
    &[
        Lien {
            ancre: "#principe",
            texte: "Le principe",
        },
        Lien {
            ancre: "#deroule",
            texte: "Comment ça se passe",
        },
        Lien {
            ancre: "#montre",
            texte: "iPhone et Watch",
        },
        Lien {
            ancre: "#offres",
            texte: "Offres",
        },
    ] as &[Lien],
    &[
        Lien {
            ancre: "#principe",
            texte: "The idea",
        },
        Lien {
            ancre: "#deroule",
            texte: "How it goes",
        },
        Lien {
            ancre: "#montre",
            texte: "iPhone and Watch",
        },
        Lien {
            ancre: "#offres",
            texte: "Plans",
        },
    ],
    &[
        Lien {
            ancre: "#principe",
            texte: "La idea",
        },
        Lien {
            ancre: "#deroule",
            texte: "Cómo funciona",
        },
        Lien {
            ancre: "#montre",
            texte: "iPhone y Watch",
        },
        Lien {
            ancre: "#offres",
            texte: "Suscripciones",
        },
    ],
]);

const NAVIGATION: Traduit<&str> = Traduit([
    "Navigation principale",
    "Main navigation",
    "Navegación principal",
]);
const OUVRIR: Traduit<&str> = Traduit(["Ouvrir le menu", "Open the menu", "Abrir el menú"]);
const FERMER: Traduit<&str> = Traduit(["Fermer le menu", "Close the menu", "Cerrar el menú"]);
const CHOIX_DE_LANGUE: Traduit<&str> =
    Traduit(["Choix de la langue", "Choose a language", "Elegir idioma"]);

/// Le choix de la langue, en toutes lettres plutôt qu'en drapeaux : un drapeau
/// désigne un pays, pas une langue, et l'espagnol se parle sur deux
/// continents.
///
/// Ce sont de vrais liens vers de vraies adresses — `/`, `/en/`, `/es/` — et
/// non un état conservé dans le navigateur : une page traduite doit pouvoir se
/// partager, se mettre en signet et s'indexer. Un moteur n'appuie sur aucun
/// bouton.
///
/// Le nom complet est lu par les synthèses vocales (`sr-only`), le code à deux
/// lettres est montré à l'œil : « EN » se reconnaît d'un coup, « English » tient
/// mal dans une barre de navigation.
#[function_component]
pub fn ChoixDeLangue(p: &ProprietesLangue) -> Html {
    html! {
        <nav aria-label={*CHOIX_DE_LANGUE.choisir(p.langue)} class="flex items-center gap-1 text-sm">
            { for LANGUES.iter().map(|&langue| {
                let courante = langue == p.langue;
                let style = if courante {
                    "background: var(--fil-4); color: var(--sur-accent); font-weight: 700"
                } else {
                    "color: var(--texte-doux)"
                };
                html! {
                    <a
                        href={langue.chemin("")}
                        hreflang={langue.code()}
                        lang={langue.code()}
                        aria-current={courante.then_some("true")}
                        class="rounded-full px-2 py-1"
                        {style}
                    >
                        <span class="sr-only">{ langue.nom() }</span>
                        <span aria-hidden="true">{ langue.code().to_uppercase() }</span>
                    </a>
                }
            }) }
        </nav>
    }
}

/// Les six fils tissés, en marque.
#[function_component]
fn Logo() -> Html {
    html! {
        <svg width="26" height="26" viewBox="0 0 32 32" aria-hidden="true">
            <g stroke-width="3" stroke-linecap="round" fill="none">
                <path d="M7 5v22" stroke="var(--fil-1)" />
                <path d="M16 5v22" stroke="var(--fil-4)" />
                <path d="M25 5v22" stroke="var(--fil-6)" />
            </g>
            <g stroke-width="2.4" stroke-linecap="round" fill="none">
                <path d="M4 12c4 3 8 3 12 0s8-3 12 0" stroke="var(--fil-2)" />
                <path d="M4 21c4 3 8 3 12 0s8-3 12 0" stroke="var(--fil-5)" />
            </g>
        </svg>
    }
}

#[function_component]
pub fn Entete(p: &ProprietesLangue) -> Html {
    let langue = p.langue;
    let ouvert = use_state(|| false);

    // Le menu se referme dès qu'on navigue : sur téléphone, il occupe l'écran.
    //
    // Le code d'écoute ne vit que dans la compilation wasm : `web_sys` n'est
    // pas lié au binaire natif, et le rendu serveur n'a ni fenêtre ni
    // événement. L'effet reste déclaré des deux côtés pour que l'arbre soit le
    // même — c'est l'arbre qui doit coïncider, pas les effets.
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
    let refermer = {
        let ouvert = ouvert.clone();
        Callback::from(move |_| ouvert.set(false))
    };

    html! {
        <header
            class="sticky top-0 z-50 backdrop-blur-md"
            style="background: color-mix(in oklab, var(--fond) 90%, transparent)"
        >
            // Les six fils, en bandeau : la marque tient en une ligne.
            <div class="tissage h-1.5" aria-hidden="true" />
            <div class="mx-auto flex w-full max-w-5xl items-center justify-between px-5 py-3 sm:px-8">
                <a href="#haut" class="flex items-center gap-2.5 font-semibold">
                    <Logo />
                    <span
                        class="text-xl font-bold tracking-tight"
                        style="font-family: var(--font-titre)"
                    >
                        { "Weave" }
                    </span>
                </a>

                <div class="flex items-center gap-5">
                    <nav
                        aria-label={*NAVIGATION.choisir(langue)}
                        class="hidden gap-7 text-sm sm:flex"
                    >
                        { for LIENS.choisir(langue).iter().map(|lien| html! {
                            <a href={lien.ancre} class="hover:underline underline-offset-4">
                                { lien.texte }
                            </a>
                        }) }
                    </nav>

                    <ChoixDeLangue {langue} />
                </div>

                <button
                    type="button"
                    class="-mr-2 p-2 sm:hidden"
                    aria-expanded={if *ouvert { "true" } else { "false" }}
                    aria-controls="menu-mobile"
                    onclick={basculer}
                >
                    <span class="sr-only">
                        { if *ouvert { *FERMER.choisir(langue) } else { *OUVRIR.choisir(langue) } }
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
            </div>

            if *ouvert {
                <nav
                    id="menu-mobile"
                    aria-label={*NAVIGATION.choisir(langue)}
                    class="sm:hidden"
                    style="border-top: 2px solid var(--bordure)"
                >
                    <ul class="px-5 py-2">
                        { for LIENS.choisir(langue).iter().map(|lien| html! {
                            <li>
                                <a
                                    href={lien.ancre}
                                    class="block py-3 text-base"
                                    onclick={refermer.clone()}
                                    style="border-bottom: 1px solid var(--bordure)"
                                >
                                    { lien.texte }
                                </a>
                            </li>
                        }) }
                    </ul>
                </nav>
            }
        </header>
    }
}
