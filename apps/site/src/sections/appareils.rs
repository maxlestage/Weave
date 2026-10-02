//! « Sur l'iPhone, et au poignet » — la Live Activity et la montre.

use crate::composants::{Carte, Fil, Section};
use crate::langues::Traduit;
use crate::racine::ProprietesLangue;
use yew::prelude::*;

const TITRE: Traduit<&str> = Traduit([
    "Sur l'iPhone, et au poignet",
    "On the iPhone, and on your wrist",
    "En el iPhone, y en la muñeca",
]);

const CHAPEAU: Traduit<&str> = Traduit([
    "L'application est écrite en Swift, nativement. La Live Activity et l'application Apple Watch ne sont pas des extras : elles servent précisément à ne pas ouvrir l'application.",
    "The app is written in Swift, natively. The Live Activity and the Apple Watch app aren't extras: their whole point is to save you from opening the app.",
    "La aplicación está escrita en Swift, de forma nativa. La Live Activity y la aplicación para Apple Watch no son extras: sirven precisamente para no abrir la aplicación.",
]);

const ACTIVITE_TEXTE: Traduit<&str> = Traduit([
    "Votre prochain plan sur l'écran verrouillé, avec le compte à rebours. Quand quelqu'un demande à venir, la bannière apparaît d'elle-même — c'est le seul moment où Weave se manifeste sans qu'on l'ait ouvert. Elle n'affiche ni nom, ni photo, ni message : ce qui est visible sur un écran verrouillé doit pouvoir être lu par quelqu'un d'autre.",
    "Your next plan on the lock screen, with the countdown. When someone asks to come, the banner appears on its own — it's the only moment Weave speaks up without being opened. It shows no name, no photo, no message: whatever is visible on a lock screen has to be safe for someone else to read.",
    "Tu próximo plan en la pantalla bloqueada, con la cuenta atrás. Cuando alguien pide venir, el aviso aparece solo — es el único momento en que Weave se manifiesta sin haberla abierto. No muestra ni nombre, ni foto, ni mensaje: lo que se ve en una pantalla bloqueada tiene que poder leerlo otra persona.",
]);

/// « Apple Watch » ne se traduit pas : c'est le nom du produit.
const MONTRE: Traduit<&str> = Traduit(["Apple Watch", "Apple Watch", "Apple Watch"]);

const MONTRE_TEXTE: Traduit<&str> = Traduit([
    "Une complication sur le cadran : le prochain plan et ce qui attend une réponse. De quoi accepter une demande à la volée, ou dicter deux phrases avant de repartir.",
    "A complication on the watch face: your next plan and whatever is waiting for an answer. Enough to accept a request in passing, or dictate two sentences before moving on.",
    "Una complicación en la esfera: el próximo plan y lo que espera respuesta. Lo justo para aceptar una petición al vuelo, o dictar dos frases antes de seguir.",
]);

const MONTRE_POINTS: Traduit<&[&str]> = Traduit([
    &[
        "• Complication : prochain plan et demandes à traiter",
        "• Réponse par dictée ou saisie manuscrite",
        "• Synchronisation par WatchConnectivity, et repli sur le réseau",
    ] as &[&str],
    &[
        "• Complication: next plan and requests to handle",
        "• Reply by dictation or handwriting",
        "• Sync over WatchConnectivity, falling back to the network",
    ],
    &[
        "• Complicación: próximo plan y peticiones pendientes",
        "• Respuesta por dictado o escritura a mano",
        "• Sincronización por WatchConnectivity, con respaldo por red",
    ],
]);

const FIN: Traduit<&str> = Traduit([
    "Ce site n'est qu'une présentation : Weave se vit dans l'application.",
    "This site is only an introduction: Weave happens in the app.",
    "Este sitio es solo una presentación: Weave se vive en la aplicación.",
]);

const APERCU: Traduit<&str> = Traduit([
    "Aperçu de la Live Activity : prochain plan samedi 10 h, 2 personnes veulent venir",
    "Preview of the Live Activity: next plan Saturday 10am, 2 people want to come",
    "Vista previa de la Live Activity: próximo plan el sábado a las 10 h, 2 personas quieren venir",
]);

const PLAN: Traduit<&str> = Traduit([
    "Marché puis brunch",
    "Market, then brunch",
    "Mercado y luego brunch",
]);

const DEMANDES: Traduit<&str> = Traduit([
    "2 personnes veulent venir",
    "2 people want to come",
    "2 personas quieren venir",
]);

const QUAND: Traduit<&str> = Traduit(["Sam. 10 h", "Sat 10am", "Sáb. 10 h"]);
const DANS: Traduit<&str> = Traduit(["dans 2 jours", "in 2 days", "en 2 días"]);

/// Maquette statique de la bannière : illustration, pas capture d'écran.
///
/// Elle porte `role="img"` et une description complète : ce qu'on y lit est une
/// image faite de texte, et un lecteur d'écran qui l'épellerait mot à mot
/// n'aurait aucun sens. La description dit la même chose que ce qu'on voit.
#[function_component]
fn ApercuActivite(p: &ProprietesLangue) -> Html {
    let langue = p.langue;
    html! {
        <div
            class="mt-6 rounded-2xl p-4"
            role="img"
            aria-label={*APERCU.choisir(langue)}
            style="background: var(--color-encre); color: #f4efe7"
        >
            <div class="flex items-center justify-between gap-4">
                <div class="flex items-center gap-3">
                    <svg width="24" height="24" viewBox="0 0 32 32" aria-hidden="true">
                        <g stroke-width="2.4" stroke-linecap="round" fill="none">
                            <path d="M8 6v20" stroke="var(--color-framboise-nuit)" />
                            <path d="M16 6v20" stroke="var(--color-menthe-nuit)" />
                            <path d="M24 6v20" stroke="var(--color-iris-nuit)" />
                        </g>
                    </svg>
                    <div>
                        <p class="text-sm font-semibold">{ *PLAN.choisir(langue) }</p>
                        <p class="text-xs" style="color: #b6aa9c">
                            { *DEMANDES.choisir(langue) }
                        </p>
                    </div>
                </div>
                <div class="text-right">
                    <p
                        class="text-lg font-semibold tabular-nums"
                        style="color: var(--color-safran-nuit)"
                    >
                        { *QUAND.choisir(langue) }
                    </p>
                    <p class="text-xs" style="color: #b6aa9c">{ *DANS.choisir(langue) }</p>
                </div>
            </div>
        </div>
    }
}

#[function_component]
pub fn Appareils(p: &ProprietesLangue) -> Html {
    let langue = p.langue;
    html! {
        <Section
            id="montre"
            fil={Fil::Cinq}
            titre={*TITRE.choisir(langue)}
            chapeau={*CHAPEAU.choisir(langue)}
            alterne=true
        >
            <div class="grid gap-4 lg:grid-cols-2">
                <Carte fil={Fil::Cinq}>
                    <h3
                        class="text-xl font-bold"
                        style={format!(
                            "font-family: var(--font-titre); color: {}",
                            Fil::Cinq.var(),
                        )}
                    >
                        { "Live Activity" }
                    </h3>
                    <p class="mt-3 leading-relaxed" style="color: var(--texte-doux)">
                        { *ACTIVITE_TEXTE.choisir(langue) }
                    </p>
                    <ApercuActivite {langue} />
                </Carte>

                <Carte fil={Fil::Quatre}>
                    <h3
                        class="text-xl font-bold"
                        style={format!(
                            "font-family: var(--font-titre); color: {}",
                            Fil::Quatre.var(),
                        )}
                    >
                        { *MONTRE.choisir(langue) }
                    </h3>
                    <p class="mt-3 leading-relaxed" style="color: var(--texte-doux)">
                        { *MONTRE_TEXTE.choisir(langue) }
                    </p>
                    <ul class="mt-5 space-y-2.5 text-sm" style="color: var(--texte-doux)">
                        { for MONTRE_POINTS.choisir(langue).iter().map(|point| html! {
                            <li>{ *point }</li>
                        }) }
                    </ul>
                </Carte>
            </div>

            <p class="mt-8 text-sm" style="color: var(--texte-doux)">
                { *FIN.choisir(langue) }
            </p>
        </Section>
    }
}
