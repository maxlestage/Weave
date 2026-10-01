//! Le site vitrine, rendu en HTML à la construction.
//!
//! ## Où en est la reprise
//!
//! Le front React est en cours de remplacement par Yew. Ce binaire est la
//! fondation : elle est éprouvée de bout en bout — Yew 0.23 rend du HTML
//! nativement, le contrat partagé est engendré sans recopie, les briques
//! d'interface compilent et rendent le même balisage qu'avant.
//!
//! Ce qui reste à porter est listé dans `apps/site/RESTE-A-FAIRE.md`, avec le
//! compte de lignes de chaque fichier. Tant que ce n'est pas fini, c'est
//! `apps/web` qui construit le site livré : rien n'est débranché.
//!
//! ## Pourquoi aucun wasm n'est produit
//!
//! Le site est du texte — quelques milliers de lignes en trois langues — et
//! son unique élément interactif est le menu de téléphone. Les questions
//! dépliantes, elles, utilisaient déjà `<details>` natif.
//!
//! Envoyer un binaire wasm pour animer un menu coûterait plus cher que tout
//! ce que la page contient. Le menu devient donc un `<details>` lui aussi, et
//! le navigateur ne reçoit qu'un fichier HTML et une feuille de style — là où
//! React envoyait 84 Kio compressés de JavaScript.
//!
//! Les traits `csr` et `hydration` de Yew restent disponibles : le jour où une
//! page a besoin d'état côté client, les composants ne changent pas, seul un
//! point d'entrée s'ajoute.

mod composants;
mod contrat;
mod langues;

use langues::{LANGUES, Langue};
use yew::ServerRenderer;
use yew::prelude::*;

/// Le pied de page, porté : il n'a besoin d'aucun état.
#[derive(Properties, PartialEq)]
struct ProprietesLangue {
    langue: Langue,
}

/// Le choix de langue, tel qu'il existait : de vrais liens vers de vraies
/// adresses, et non un réglage conservé dans le navigateur. Une page traduite
/// doit pouvoir se partager, se mettre en signet, et s'indexer — un moteur
/// n'appuie sur aucun bouton.
#[function_component]
fn ChoixDeLangue(p: &ProprietesLangue) -> Html {
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

#[function_component]
fn Accueil(p: &ProprietesLangue) -> Html {
    html! {
        <>
            <main>
                // Les neuf sections viennent ici, au fur et à mesure.
                <composants::Section
                    id="offres"
                    titre="Les offres"
                    fil={composants::Fil::Cinq}
                    alterne=true
                >
                    <ul class="grid gap-6 sm:grid-cols-2">
                        { for contrat::PALIERS.iter().map(|palier| html! {
                            <li>
                                <composants::Carte
                                    fil={composants::Fil::Cinq}
                                    accentuee={palier.prix_centimes == 0}
                                >
                                    <h3 class="text-lg font-bold">{ palier.nom }</h3>
                                    <p class="mt-1 text-2xl font-semibold tabular-nums">
                                        { *palier.prix.choisir(p.langue) }
                                    </p>
                                    <p class="mt-2" style="color: var(--texte-doux)">
                                        { palier.accroche }
                                    </p>
                                    <ul class="mt-4 space-y-1 text-sm">
                                        { for palier.atouts.iter().map(|atout| html! {
                                            <li>{ *atout }</li>
                                        }) }
                                    </ul>
                                </composants::Carte>
                            </li>
                        }) }
                    </ul>
                </composants::Section>
            </main>
            <footer class="px-5 py-10 sm:px-8">
                <ChoixDeLangue langue={p.langue} />
            </footer>
        </>
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    for langue in LANGUES {
        let corps = ServerRenderer::<Accueil>::with_props(move || ProprietesLangue { langue })
            // Aucun marqueur d'hydratation : rien ne viendra hydrater, et les
            // laisser alourdirait chaque page de commentaires inertes.
            .hydratable(false)
            .render()
            .await;
        println!("--- {} ---\n{corps}\n", langue.code());
    }
}
