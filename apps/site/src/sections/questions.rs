//! Les questions fréquentes.
//!
//! Chaque question est un `<details>` : ouvrir et refermer est du ressort du
//! navigateur, pas du nôtre. Aucun état, donc, et rien à hydrater ici — ce qui
//! vaut mieux qu'un accordéon écrit à la main, qui marcherait moins bien et
//! seulement après l'arrivée du wasm.

use crate::composants::{Fil, Section};
use crate::contrat::{MAX_OPEN_PLANS, MIN_AGE, REQUESTS_PER_DAY_FLOOR};
use crate::langues::{Langue, Traduit};
use crate::racine::ProprietesLangue;
use yew::prelude::*;

const TITRE: Traduit<&str> = Traduit([
    "Questions fréquentes",
    "Frequently asked questions",
    "Preguntas frecuentes",
]);

/// Le fil de la section : il colore aussi le signe « plus » de chaque question.
const FIL: Fil = Fil::Trois;

struct Question {
    question: &'static str,
    reponse: String,
}

/// Les huit questions, dans la langue demandée.
fn questions(langue: Langue) -> [Question; 8] {
    match langue {
        Langue::Fr => [
            Question {
                question: "À qui s'adresse Weave ?",
                reponse: format!("Aux jeunes adultes, d'abord : le vocabulaire, les prix et le rythme sont écrits pour ce moment de la vie où l'on change de ville, de travail et de cercle d'amis, et où l'on se retrouve à ne connaître personne un vendredi soir. L'accès est réservé aux {MIN_AGE} ans et plus, sans exception."),
            },
            Question {
                question: "Ce n'est pas une application de rencontre, alors ?",
                reponse: "Si. Simplement, on ne commence pas par se décrire : on commence par proposer quelque chose à faire. Ce qui se passe ensuite ne regarde que les deux personnes concernées — amitié, relation, ou rien du tout.".to_string(),
            },
            Question {
                question: "Pourquoi un nombre limité de demandes par jour ?",
                reponse: format!("Parce que sans limite, demander ne veut plus rien dire : on envoie le même message à trente personnes et on trie les réponses. Avec un quota — au minimum {REQUESTS_PER_DAY_FLOOR} par jour, même sans payer — on choisit les plans auxquels on veut vraiment aller, et on écrit quelque chose qui tient debout."),
            },
            Question {
                question: "Je peux payer pour que mon plan passe devant ?",
                reponse: "Non, et c'est la règle qui ne bougera pas. Le fil est trié par ce qui arrive le plus tôt, puis par ce qui est le plus près. Aucun abonnement, aucun achat n'intervient dans cet ordre. Ce qui se paie, c'est l'horizon de publication, la finesse des critères et les plans de groupe.".to_string(),
            },
            Question {
                question: "Que se passe-t-il si ma demande est refusée ?",
                reponse: "Elle se ferme, sans motif et sans notification accusatrice. Si vous la retirez avant qu'elle ait été lue, elle vous est rendue — se raviser vite ne doit pas coûter la journée.".to_string(),
            },
            Question {
                question: "Combien de plans puis-je publier ?",
                reponse: format!("{MAX_OPEN_PLANS} ouverts à la fois. Ce n'est pas une brimade : au-delà, ce ne sont plus des plans, c'est une annonce permanente. Quand une date passe, la place se libère."),
            },
            Question {
                question: "Et si personne ne demande à venir ?",
                reponse: "Le plan passe et disparaît. Vous faites ce que vous aviez prévu — c'était l'idée de départ. Rien ne vous reproche un plan sans réponse, et aucun compteur ne vous le rappelle.".to_string(),
            },
            Question {
                question: "Y a-t-il une version Android ou web ?",
                reponse: "Pas au lancement. Weave commence sur iPhone et Apple Watch, en Swift natif ; ce site présente l'application, il ne la remplace pas.".to_string(),
            },
        ],
        Langue::En => [
            Question {
                question: "Who is Weave for?",
                reponse: format!("Young adults, first of all: the wording, the prices and the pace are written for that moment in life when you change town, job and circle of friends, and find yourself knowing nobody on a Friday night. Access is restricted to people aged {MIN_AGE} and over, without exception."),
            },
            Question {
                question: "So it isn't a dating app?",
                reponse: "It is. It's just that you don't start by describing yourself: you start by suggesting something to do. What happens next is between the two people concerned — friendship, a relationship, or nothing at all.".to_string(),
            },
            Question {
                question: "Why a limited number of requests a day?",
                reponse: format!("Because with no limit, asking stops meaning anything: you send the same message to thirty people and sort through the replies. With a quota — at least {REQUESTS_PER_DAY_FLOOR} a day, even without paying — you choose the plans you actually want to go to, and you write something that stands up."),
            },
            Question {
                question: "Can I pay to get my plan shown first?",
                reponse: "No, and that is the rule that will not move. The feed is sorted by what happens soonest, then by what is nearest. No subscription, no purchase touches that order. What you pay for is how far ahead you can post, how precise the filters are, and group plans.".to_string(),
            },
            Question {
                question: "What happens if my request is turned down?",
                reponse: "It closes, with no reason given and no accusing notification. If you withdraw it before it has been read, it is given back to you — changing your mind quickly shouldn't cost you the day.".to_string(),
            },
            Question {
                question: "How many plans can I post?",
                reponse: format!("{MAX_OPEN_PLANS} open at a time. It isn't a punishment: beyond that they stop being plans and become a standing advert. When a date passes, the slot frees up."),
            },
            Question {
                question: "And if nobody asks to come?",
                reponse: "The plan passes and disappears. You do what you had planned — that was the idea to begin with. Nothing holds an unanswered plan against you, and no counter reminds you of it.".to_string(),
            },
            Question {
                question: "Is there an Android or web version?",
                reponse: "Not at launch. Weave starts on iPhone and Apple Watch, in native Swift; this site introduces the app, it doesn't replace it.".to_string(),
            },
        ],
        Langue::Es => [
            Question {
                question: "¿A quién va dirigida Weave?",
                reponse: format!("A los adultos jóvenes, ante todo: el vocabulario, los precios y el ritmo están escritos para ese momento de la vida en que cambias de ciudad, de trabajo y de círculo de amigos, y te encuentras sin conocer a nadie un viernes por la noche. El acceso está reservado a mayores de {MIN_AGE} años, sin excepción."),
            },
            Question {
                question: "Entonces, ¿no es una aplicación de citas?",
                reponse: "Sí lo es. Solo que no se empieza describiéndose: se empieza proponiendo algo que hacer. Lo que pase después solo concierne a las dos personas — amistad, relación, o nada en absoluto.".to_string(),
            },
            Question {
                question: "¿Por qué un número limitado de peticiones al día?",
                reponse: format!("Porque sin límite, pedir deja de significar nada: mandas el mismo mensaje a treinta personas y luego cribas las respuestas. Con una cuota — al menos {REQUESTS_PER_DAY_FLOOR} al día, incluso sin pagar — eliges los planes a los que de verdad quieres ir, y escribes algo que se sostiene."),
            },
            Question {
                question: "¿Puedo pagar para que mi plan salga el primero?",
                reponse: "No, y esa es la regla que no se va a mover. El muro se ordena por lo que ocurre antes, y luego por lo que está más cerca. Ninguna suscripción, ninguna compra interviene en ese orden. Lo que se paga es la antelación para publicar, lo finos que son los criterios y los planes de grupo.".to_string(),
            },
            Question {
                question: "¿Qué pasa si rechazan mi petición?",
                reponse: "Se cierra, sin motivo y sin ninguna notificación acusadora. Si la retiras antes de que la hayan leído, se te devuelve — cambiar de idea rápido no debe costarte el día.".to_string(),
            },
            Question {
                question: "¿Cuántos planes puedo publicar?",
                reponse: format!("{MAX_OPEN_PLANS} abiertos a la vez. No es un castigo: más allá de eso dejan de ser planes y se convierten en un anuncio permanente. Cuando pasa una fecha, la plaza se libera."),
            },
            Question {
                question: "¿Y si nadie pide venir?",
                reponse: "El plan pasa y desaparece. Haces lo que tenías previsto — esa era la idea de partida. Nada te reprocha un plan sin respuesta, y ningún contador te lo recuerda.".to_string(),
            },
            Question {
                question: "¿Hay versión para Android o web?",
                reponse: "No en el lanzamiento. Weave empieza en iPhone y Apple Watch, en Swift nativo; este sitio presenta la aplicación, no la sustituye.".to_string(),
            },
        ],
    }
}

#[function_component]
pub fn Questions(p: &ProprietesLangue) -> Html {
    let langue = p.langue;
    html! {
        <Section id="questions" fil={FIL} titre={*TITRE.choisir(langue)}>
            <div class="divide-y" style="border-color: var(--bordure)">
                { for questions(langue).into_iter().map(|item| html! {
                    <details class="group py-4" style="border-color: var(--bordure)">
                        <summary class="flex cursor-pointer list-none items-center justify-between gap-4 text-lg font-medium">
                            { item.question }
                            <svg
                                width="20"
                                height="20"
                                viewBox="0 0 20 20"
                                class="shrink-0 transition-transform group-open:rotate-45"
                                aria-hidden="true"
                            >
                                <path
                                    d="M10 4v12M4 10h12"
                                    stroke={FIL.var()}
                                    stroke-width="1.8"
                                    stroke-linecap="round"
                                />
                            </svg>
                        </summary>
                        <p class="mt-3 max-w-2xl leading-relaxed" style="color: var(--texte-doux)">
                            { item.reponse }
                        </p>
                    </details>
                }) }
            </div>
        </Section>
    }
}
