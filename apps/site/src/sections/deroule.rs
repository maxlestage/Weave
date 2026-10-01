//! « Comment ça se passe » — les six étapes, dans l'ordre.

use crate::composants::{Fil, Pastille, Section};
use crate::contrat::{MAX_OPEN_PLANS, PLAN_MIN_LEAD_MINUTES, REQUESTS_PER_DAY_FLOOR};
use crate::langues::{Langue, Traduit};
use crate::racine::ProprietesLangue;
use yew::prelude::*;

const TITRE: Traduit<&str> = Traduit(["Comment ça se passe", "How it goes", "Cómo funciona"]);

const CHAPEAU: Traduit<&str> = Traduit([
    "Six étapes, et vous pouvez en rester à la troisième aussi longtemps que vous voulez.",
    "Six steps, and you can stop at the third one for as long as you like.",
    "Seis pasos, y puedes quedarte en el tercero todo el tiempo que quieras.",
]);

/// Une étape : un titre, et ce qu'elle veut dire.
struct Etape {
    titre: &'static str,
    /// Une `String` et non un `&'static str` : deux étapes citent des nombres
    /// du contrat partagé. Les écrire en dur ferait du site une source de
    /// vérité de plus pour le délai minimum de publication et le nombre de
    /// plans ouverts — et c'est précisément ce que la génération du contrat
    /// sert à éviter.
    texte: String,
}

/// Les six étapes, dans la langue demandée.
///
/// Un `match` exhaustif plutôt qu'une table : une quatrième langue ajoutée à
/// `Langue` fait échouer la compilation ICI, au lieu de livrer une page dont
/// une section serait restée en français.
fn etapes(langue: Langue) -> [Etape; 6] {
    match langue {
        Langue::Fr => [
            Etape {
                titre: "Vous dites où vous êtes, et c'est tout",
                texte: "Une ville, un genre, une phrase si vous voulez. Pas de questionnaire, pas de description de vous-même à rédiger : ce n'est pas ce qu'on va lire.".to_string(),
            },
            Etape {
                titre: "Vous publiez un plan",
                texte: format!("Ce que vous comptez faire, quand, et combien de personnes peuvent venir. Au moins {PLAN_MIN_LEAD_MINUTES} minutes à l'avance, et au plus {MAX_OPEN_PLANS} plans ouverts en même temps."),
            },
            Etape {
                titre: "Vous lisez le fil des autres",
                texte: "Les plans autour de vous, du plus imminent au plus lointain. Pas de pile à balayer : une liste qui se vide toute seule quand les dates passent.".to_string(),
            },
            Etape {
                titre: "Vous demandez à venir, en écrivant",
                texte: format!("Quelques lignes qui disent pourquoi ce plan-là. Vous en avez un nombre limité par jour — au minimum {REQUESTS_PER_DAY_FLOOR}, même sans payer — et elles reviennent à minuit."),
            },
            Etape {
                titre: "La personne accepte, ou non",
                texte: "Un refus ne se commente pas et ne notifie rien d'accusateur. Une demande retirée avant d'avoir été lue vous est rendue.".to_string(),
            },
            Etape {
                titre: "La conversation s'ouvre",
                texte: "Seulement après un oui, et seulement à deux — même sur un plan de groupe. Elle se ferme quand vous voulez, et les messages sont purgés ensuite.".to_string(),
            },
        ],
        Langue::En => [
            Etape {
                titre: "You say where you are, and that's it",
                texte: "A town, a gender, a sentence if you feel like it. No questionnaire, no self-description to write: that isn't what anyone is going to read.".to_string(),
            },
            Etape {
                titre: "You post a plan",
                texte: format!("What you're doing, when, and how many people can come. At least {PLAN_MIN_LEAD_MINUTES} minutes ahead, and at most {MAX_OPEN_PLANS} plans open at once."),
            },
            Etape {
                titre: "You read other people's feed",
                texte: "The plans around you, soonest first. No pile to swipe through: a list that empties itself as the dates pass.".to_string(),
            },
            Etape {
                titre: "You ask to come, in writing",
                texte: format!("A few lines saying why that plan. You get a limited number each day — at least {REQUESTS_PER_DAY_FLOOR}, even without paying — and they come back at midnight."),
            },
            Etape {
                titre: "They say yes, or they don't",
                texte: "A refusal carries no comment and sends nothing accusing. A request withdrawn before it was read is given back to you.".to_string(),
            },
            Etape {
                titre: "The conversation opens",
                texte: "Only after a yes, and only between two people — even on a group plan. You close it whenever you like, and the messages are purged afterwards.".to_string(),
            },
        ],
        Langue::Es => [
            Etape {
                titre: "Dices dónde estás, y ya está",
                texte: "Una ciudad, un género, una frase si te apetece. Sin cuestionario, sin descripción de ti mismo que redactar: no es eso lo que se va a leer.".to_string(),
            },
            Etape {
                titre: "Publicas un plan",
                texte: format!("Lo que piensas hacer, cuándo, y cuántas personas pueden venir. Con al menos {PLAN_MIN_LEAD_MINUTES} minutos de antelación, y como máximo {MAX_OPEN_PLANS} planes abiertos a la vez."),
            },
            Etape {
                titre: "Lees el muro de los demás",
                texte: "Los planes a tu alrededor, del más próximo al más lejano. Sin pila que deslizar: una lista que se vacía sola cuando pasan las fechas.".to_string(),
            },
            Etape {
                titre: "Pides venir, escribiendo",
                texte: format!("Unas líneas que digan por qué ese plan. Tienes un número limitado al día — al menos {REQUESTS_PER_DAY_FLOOR}, incluso sin pagar — y vuelven a medianoche."),
            },
            Etape {
                titre: "La persona acepta, o no",
                texte: "Un rechazo no se comenta y no notifica nada acusador. Una petición retirada antes de ser leída se te devuelve.".to_string(),
            },
            Etape {
                titre: "Se abre la conversación",
                texte: "Solo tras un sí, y solo entre dos — incluso en un plan de grupo. Se cierra cuando quieras, y los mensajes se purgan después.".to_string(),
            },
        ],
    }
}

/// Le fil de la pastille d'une étape : les six fils de la palette, à la suite.
const FILS: [Fil; 6] = [
    Fil::Un,
    Fil::Deux,
    Fil::Trois,
    Fil::Quatre,
    Fil::Cinq,
    Fil::Six,
];

#[function_component]
pub fn Deroule(p: &ProprietesLangue) -> Html {
    let langue = p.langue;
    html! {
        <Section
            id="deroule"
            fil={Fil::Deux}
            titre={*TITRE.choisir(langue)}
            chapeau={*CHAPEAU.choisir(langue)}
        >
            <ol class="space-y-6">
                { for etapes(langue).into_iter().enumerate().map(|(rang, etape)| html! {
                    <li class="flex gap-5">
                        <Pastille fil={FILS[rang % FILS.len()]}>{ rang + 1 }</Pastille>
                        <div class="pt-1">
                            <h3 class="text-lg font-bold">{ etape.titre }</h3>
                            <p class="mt-2 leading-relaxed" style="color: var(--texte-doux)">
                                { etape.texte }
                            </p>
                        </div>
                    </li>
                }) }
            </ol>
        </Section>
    }
}
