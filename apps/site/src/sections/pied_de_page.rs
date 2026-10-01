//! Le pied de page : la marque, les deux navigations, et les mentions.

use crate::contrat::{self, MIN_AGE};
use crate::langues::{LANGUE_PAR_DEFAUT, Langue, Traduit};
use crate::racine::ProprietesLangue;
use yew::prelude::*;

const ACCROCHE: Traduit<&str> = Traduit([
    "Des plans, pas des profils. On publie ce qu'on compte faire, les autres demandent à venir — en écrivant pourquoi.",
    "Plans, not profiles. You post what you're going to do, other people ask to come — and write why.",
    "Planes, no perfiles. Publicas lo que piensas hacer, los demás piden venir — y escriben por qué.",
]);

const SITE: Traduit<&str> = Traduit(["Le site", "The site", "El sitio"]);
const LEGAL: Traduit<&str> = Traduit([
    "Informations légales",
    "Legal information",
    "Información legal",
]);

/// Prévient que les documents juridiques ne sont publiés qu'en français.
///
/// `None` en français : il n'y a rien à prévenir à qui lit déjà la langue du
/// texte. Un avertissement affiché là serait du bruit, et c'est pour cela que
/// c'est une option et non une chaîne vide — une chaîne vide se serait affichée
/// dans un paragraphe vide.
const LEGAL_EN_FRANCAIS: Traduit<Option<&str>> = Traduit([
    None,
    Some(
        "Our legal documents are published in French only. The French text is the one that binds us.",
    ),
    Some(
        "Nuestros documentos legales se publican solo en francés. El texto francés es el que nos obliga.",
    ),
]);

struct Lien {
    ancre: &'static str,
    texte: &'static str,
}

const LIENS: Traduit<&[Lien]> = Traduit([
    &[
        Lien {
            ancre: "principe",
            texte: "Le principe",
        },
        Lien {
            ancre: "deroule",
            texte: "Comment ça se passe",
        },
        Lien {
            ancre: "offres",
            texte: "Offres",
        },
        Lien {
            ancre: "questions",
            texte: "Questions",
        },
    ] as &[Lien],
    &[
        Lien {
            ancre: "principe",
            texte: "The idea",
        },
        Lien {
            ancre: "deroule",
            texte: "How it goes",
        },
        Lien {
            ancre: "offres",
            texte: "Plans",
        },
        Lien {
            ancre: "questions",
            texte: "Questions",
        },
    ],
    &[
        Lien {
            ancre: "principe",
            texte: "La idea",
        },
        Lien {
            ancre: "deroule",
            texte: "Cómo funciona",
        },
        Lien {
            ancre: "offres",
            texte: "Suscripciones",
        },
        Lien {
            ancre: "questions",
            texte: "Preguntas",
        },
    ],
]);

fn age(langue: Langue, age: u32) -> String {
    match langue {
        Langue::Fr => format!("Weave est réservé aux personnes de {age} ans et plus."),
        Langue::En => format!("Weave is for people aged {age} and over."),
        Langue::Es => format!("Weave es para mayores de {age} años."),
    }
}

fn marques(langue: Langue, annee: &str) -> String {
    match langue {
        Langue::Fr => format!(
            "© {annee} Weave. Apple, iPhone et Apple Watch sont des marques déposées d'Apple Inc."
        ),
        Langue::En => format!(
            "© {annee} Weave. Apple, iPhone and Apple Watch are registered trademarks of Apple Inc."
        ),
        Langue::Es => format!(
            "© {annee} Weave. Apple, iPhone y Apple Watch son marcas registradas de Apple Inc."
        ),
    }
}

/// L'année du droit d'auteur, figée par `build.rs`. Voir ce fichier pour la
/// raison : elle n'est PAS calculée au rendu.
const ANNEE: &str = env!("WEAVE_ANNEE");

#[function_component]
pub fn PiedDePage(p: &ProprietesLangue) -> Html {
    let langue = p.langue;
    let en_francais = *LEGAL_EN_FRANCAIS.choisir(langue);

    html! {
        <footer class="px-5 py-12 sm:px-8" style="background: var(--fond-alterne)">
            <div class="mx-auto w-full max-w-5xl">
                <div class="tissage mb-8 h-1.5 rounded-full" aria-hidden="true" />

                <p class="text-2xl font-bold" style="font-family: var(--font-titre)">
                    { "Weave" }
                </p>
                <p class="mt-2 max-w-md leading-relaxed" style="color: var(--texte-doux)">
                    { *ACCROCHE.choisir(langue) }
                </p>

                // Les ancres portent le préfixe de langue : ce pied de page
                // s'affiche aussi sur les pages juridiques, où « #offres » seul
                // ne désignerait rien, et sur « /en/ », où il faut revenir à la
                // bonne page d'accueil.
                <nav aria-label={*SITE.choisir(langue)} class="mt-8">
                    <h2 class="text-sm font-semibold tracking-wide uppercase">
                        { *SITE.choisir(langue) }
                    </h2>
                    <ul class="mt-3 flex flex-wrap gap-x-8 gap-y-3 text-sm">
                        { for LIENS.choisir(langue).iter().map(|lien| html! {
                            <li>
                                <a
                                    href={format!("{}#{}", langue.chemin(""), lien.ancre)}
                                    class="hover:underline underline-offset-4"
                                >
                                    { lien.texte }
                                </a>
                            </li>
                        }) }
                    </ul>
                </nav>

                // Les documents juridiques ne sont publiés qu'en français, et
                // leurs liens ne portent donc pas de préfixe de langue.
                // Traduire des conditions générales n'est pas un travail de
                // langue : une traduction non relue engagerait sur un texte que
                // personne n'a validé. Le visiteur anglophone ou hispanophone
                // est prévenu plutôt que mené vers une page dont il ne saurait
                // pas qu'elle est dans une autre langue.
                <nav aria-label={*LEGAL.choisir(langue)} class="mt-7">
                    <h2 class="text-sm font-semibold tracking-wide uppercase">
                        { *LEGAL.choisir(langue) }
                    </h2>
                    <ul class="mt-3 flex flex-wrap gap-x-8 gap-y-3 text-sm">
                        { for contrat::PAGES_JURIDIQUES.iter().map(|page| html! {
                            <li>
                                <a
                                    href={format!("/{}", page.adresse)}
                                    hreflang={LANGUE_PAR_DEFAUT.code()}
                                    lang={LANGUE_PAR_DEFAUT.code()}
                                    class="hover:underline underline-offset-4"
                                >
                                    { page.lien }
                                    if en_francais.is_some() {
                                        <span class="sr-only">
                                            { format!(" ({})", LANGUE_PAR_DEFAUT.nom()) }
                                        </span>
                                    }
                                </a>
                            </li>
                        }) }
                    </ul>
                    if let Some(avis) = en_francais {
                        <p class="mt-3 max-w-md text-sm" style="color: var(--texte-doux)">
                            { avis }
                        </p>
                    }
                </nav>

                <p class="mt-10 text-sm" style="color: var(--texte-doux)">
                    { age(langue, MIN_AGE) }
                </p>
                <p class="mt-2 text-sm" style="color: var(--texte-doux)">
                    { marques(langue, ANNEE) }
                </p>
            </div>
        </footer>
    }
}
