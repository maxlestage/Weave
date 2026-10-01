//! L'arbre de la page d'accueil, partagé par le rendu serveur et
//! l'hydratation.
//!
//! La langue est un ACCESSOIRE et non une détection : la construction rend le
//! même arbre trois fois, une par adresse, et le client reprend celui que
//! l'adresse indique. Deviner d'après `navigator.language` produirait un
//! balisage différent de celui rendu au serveur, et l'hydratation échouerait.

use crate::langues::{Langue, Traduit};
use crate::sections::appareils::Appareils;
use crate::sections::confiance::Confiance;
use crate::sections::deroule::Deroule;
use crate::sections::entete::Entete;
use crate::sections::offres::Offres;
use crate::sections::ouverture::Ouverture;
use crate::sections::pied_de_page::PiedDePage;
use crate::sections::principe::Principe;
use crate::sections::questions::Questions;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ProprietesLangue {
    pub langue: Langue,
}

const CONTENU: Traduit<&str> = Traduit(["Aller au contenu", "Skip to content", "Ir al contenido"]);

#[function_component]
pub fn Accueil(p: &ProprietesLangue) -> Html {
    let langue = p.langue;
    html! {
        <>
            // Le lien d'évitement : invisible, sauf au clavier. Il est le
            // premier élément de la page parce qu'il doit être la première
            // chose qu'on atteigne en tabulant — sans quoi il ne sert à rien.
            <a
                href="#contenu"
                class="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-[100] focus:rounded-full focus:px-4 focus:py-2"
                style="background: var(--accent); color: var(--sur-accent)"
            >
                { *CONTENU.choisir(langue) }
            </a>

            <Entete {langue} />

            <main id="contenu">
                <Ouverture {langue} />
                <Principe {langue} />
                <Deroule {langue} />
                <Appareils {langue} />
                <Offres {langue} />
                <Confiance {langue} />
                <Questions {langue} />
            </main>

            <PiedDePage {langue} />
        </>
    }
}
