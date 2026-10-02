//! Charpente des pages juridiques.
//!
//! Ces pages se lisent, elles ne se parcourent pas : une colonne étroite, des
//! titres numérotés, un sommaire en tête. Elles réutilisent le pied du site
//! pour qu'on sache toujours où l'on est, mais elles n'empruntent pas ses
//! cartes colorées — un texte contractuel n'est pas une section marketing.

use crate::langues::Langue;
use crate::sections::pied_de_page::PiedDePage;
use yew::prelude::*;

/// La date de la version en vigueur d'un document, par son adresse.
///
/// Une date par document, et non une pour tous : les textes ne changent pas
/// ensemble, et celle de la politique de confidentialité EST la version que
/// portent les consentements enregistrés — la changer périme ceux donnés sur
/// la précédente.
pub fn mise_a_jour(adresse: &str) -> &'static str {
    crate::contrat::PAGES_JURIDIQUES
        .iter()
        .find(|page| page.adresse == adresse)
        .map(|page| page.mise_a_jour)
        // Un `expect` et non une valeur de repli : une page juridique sans date
        // de version est un document qu'on ne peut pas dater, donc qu'on ne
        // doit pas publier. Mieux vaut que la construction s'arrête.
        .expect("aucune page juridique à cette adresse")
}

/// Un article du document : son ancre, son titre, son contenu.
#[derive(Clone, PartialEq)]
pub struct Article {
    pub id: &'static str,
    pub titre: &'static str,
    pub contenu: Html,
}

struct Lien {
    href: &'static str,
    texte: &'static str,
}

const LIENS: [Lien; 3] = [
    Lien {
        href: "/#principe",
        texte: "Le principe",
    },
    Lien {
        href: "/#deroule",
        texte: "Comment ça se passe",
    },
    Lien {
        href: "/#offres",
        texte: "Offres",
    },
];

/// Les six fils tissés, en marque. Le même dessin que celui de l'accueil.
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

/// En-tête des pages juridiques : sans état, donc sans JavaScript.
///
/// Celui de l'accueil ouvre un menu au téléphone, ce qui suppose du code côté
/// client. Ces pages-ci sont pré-rendues et n'embarquent AUCUN script : les
/// trois liens tiennent sur une ligne qui passe à la ligne, et le menu
/// escamotable n'a plus de raison d'être.
#[function_component]
fn EnteteStatique() -> Html {
    html! {
        <header>
            <div class="tissage h-1.5" aria-hidden="true" />
            <div class="mx-auto flex w-full max-w-5xl flex-wrap items-center justify-between gap-x-6 gap-y-2 px-5 py-3 sm:px-8">
                <a href="/" class="flex items-center gap-2.5 font-semibold">
                    <Logo />
                    <span
                        class="text-xl font-bold tracking-tight"
                        style="font-family: var(--font-titre)"
                    >
                        { "Weave" }
                    </span>
                </a>
                <nav
                    aria-label="Navigation principale"
                    class="flex flex-wrap gap-x-6 gap-y-1 text-sm"
                >
                    { for LIENS.iter().map(|lien| html! {
                        <a href={lien.href} class="hover:underline underline-offset-4">
                            { lien.texte }
                        </a>
                    }) }
                </nav>
            </div>
        </header>
    }
}

/// « À compléter » : ce que seul l'éditeur peut renseigner — raison sociale,
/// adresse, numéro d'immatriculation, hébergeur.
///
/// Le marqueur est volontairement voyant. Une mention légale incomplète expose
/// son éditeur, et un gabarit qui se fond dans le texte finit par être publié
/// tel quel — c'est le mode de défaillance qu'on cherche à rendre impossible.
#[derive(Properties, PartialEq)]
pub struct ProprietesAC {
    #[prop_or_default]
    pub children: Html,
}

#[function_component]
#[allow(non_snake_case)]
pub fn AC(p: &ProprietesAC) -> Html {
    html! {
        <mark
            class="rounded px-1.5 py-0.5 text-[0.9em] font-semibold"
            style="background: color-mix(in oklab, var(--fil-3) 22%, transparent); color: inherit"
            title="À compléter avant publication"
        >
            { p.children.clone() }
        </mark>
    }
}

#[derive(Properties, PartialEq)]
pub struct ProprietesPage {
    pub titre: AttrValue,
    pub chapeau: AttrValue,
    pub mise_a_jour: AttrValue,
    pub articles: Vec<Article>,
    /// Ce qui vient après les articles, s'il y a quelque chose.
    #[prop_or_default]
    pub apres: Option<Html>,
}

#[function_component]
pub fn Page(p: &ProprietesPage) -> Html {
    html! {
        <>
            <a
                href="#contenu"
                class="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-[100] focus:rounded-full focus:px-4 focus:py-2"
                style="background: var(--accent); color: var(--sur-accent)"
            >
                { "Aller au contenu" }
            </a>

            <EnteteStatique />

            <main id="contenu" class="px-5 pt-12 pb-20 sm:px-8">
                <div class="mx-auto w-full max-w-2xl">
                    <nav aria-label="Fil d'Ariane" class="text-sm" style="color: var(--texte-doux)">
                        <a href="/" class="hover:underline underline-offset-4">{ "Accueil" }</a>
                        <span aria-hidden="true">{ " › " }</span>
                        <span>{ p.titre.clone() }</span>
                    </nav>

                    <h1
                        class="mt-5 text-[2rem] leading-tight font-bold tracking-tight sm:text-5xl"
                        style="font-family: var(--font-titre)"
                    >
                        { p.titre.clone() }
                    </h1>

                    <p class="mt-5 text-lg leading-relaxed">{ p.chapeau.clone() }</p>

                    <p class="mt-4 text-sm" style="color: var(--texte-doux)">
                        { "Dernière mise à jour : " }{ p.mise_a_jour.clone() }
                    </p>

                    <div class="tissage mt-8 h-1.5 rounded-full" aria-hidden="true" />

                    <nav aria-labelledby="sommaire" class="mt-8">
                        <h2 id="sommaire" class="text-sm font-semibold tracking-wide uppercase">
                            { "Sommaire" }
                        </h2>
                        <ol class="mt-3 space-y-1.5 text-sm">
                            { for p.articles.iter().enumerate().map(|(rang, article)| html! {
                                <li>
                                    <a
                                        href={format!("#{}", article.id)}
                                        class="hover:underline underline-offset-4"
                                    >
                                        <span class="tabular-nums" style="color: var(--texte-doux)">
                                            { format!("{}.", rang + 1) }
                                        </span>
                                        { " " }
                                        { article.titre }
                                    </a>
                                </li>
                            }) }
                        </ol>
                    </nav>

                    <div class="mt-12 space-y-11">
                        { for p.articles.iter().enumerate().map(|(rang, article)| html! {
                            <section id={article.id} class="scroll-mt-24">
                                <h2 class="text-xl font-bold sm:text-2xl" style="color: var(--fil-4)">
                                    <span class="tabular-nums" style="color: var(--texte-doux)">
                                        { format!("{}.", rang + 1) }
                                    </span>
                                    { " " }
                                    { article.titre }
                                </h2>
                                <div class="prose-weave mt-3.5 leading-relaxed">
                                    { article.contenu.clone() }
                                </div>
                            </section>
                        }) }
                    </div>

                    if let Some(apres) = &p.apres {
                        { apres.clone() }
                    }
                </div>
            </main>

            // Le pied de page est celui du site, en français : ces pages ne
            // sont publiées que dans cette langue.
            <PiedDePage langue={Langue::Fr} />
        </>
    }
}

#[derive(Properties, PartialEq)]
pub struct ProprietesTableau {
    pub entetes: Vec<&'static str>,
    pub lignes: Vec<Vec<Html>>,
    /// Ce que le tableau montre, pour qui l'atteint au clavier.
    #[prop_or_default]
    pub titre: Option<AttrValue>,
}

/// Un tableau lisible au téléphone : il défile seul plutôt que de déborder.
///
/// Le conteneur qui défile est FOCALISABLE, et porte un nom. Sans cela, ce qui
/// dépasse à droite n'était atteignable qu'à la souris ou au doigt : personne
/// ne peut faire défiler au clavier une zone qui ne prend pas le focus, et les
/// colonnes cachées d'un tableau de tarifs devenaient illisibles pour qui
/// navigue au clavier. Le nom vient du titre : une zone focalisable et muette
/// ne dit pas ce qu'on vient d'atteindre.
#[function_component]
pub fn Tableau(p: &ProprietesTableau) -> Html {
    html! {
        <div
            class="-mx-5 mt-4 overflow-x-auto px-5 sm:mx-0 sm:px-0"
            tabindex="0"
            role="group"
            aria-label={p.titre.clone().unwrap_or(AttrValue::Static("Tableau"))}
        >
            <table class="w-full min-w-[34rem] border-collapse text-sm">
                <thead>
                    <tr>
                        { for p.entetes.iter().map(|entete| html! {
                            <th
                                scope="col"
                                class="py-2 pr-4 text-left align-bottom font-semibold"
                                style="border-bottom: 2px solid var(--bordure)"
                            >
                                { *entete }
                            </th>
                        }) }
                    </tr>
                </thead>
                <tbody>
                    { for p.lignes.iter().map(|ligne| html! {
                        <tr>
                            { for ligne.iter().map(|cellule| html! {
                                <td
                                    class="py-2.5 pr-4 align-top"
                                    style="border-bottom: 1px solid var(--bordure)"
                                >
                                    { cellule.clone() }
                                </td>
                            }) }
                        </tr>
                    }) }
                </tbody>
            </table>
        </div>
    }
}
