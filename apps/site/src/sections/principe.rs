//! « Le principe » — les quatre piliers, ce que Weave ne fait pas, et ce qu'on
//! y publie.

use crate::composants::{Carte, Fil, Section};
use crate::contrat::{self, MAX_OPEN_PLANS, REQUEST_MIN_CHARS, REQUESTS_PER_DAY_FLOOR};
use crate::langues::{Langue, Traduit};
use crate::racine::ProprietesLangue;
use yew::prelude::*;

const TITRE: Traduit<&str> = Traduit(["Le principe", "The idea", "La idea"]);

const CHAPEAU: Traduit<&str> = Traduit([
    "Une fiche dit qui on prétend être. Un plan dit ce qu'on fait jeudi. Weave ne garde que le second.",
    "A profile says who you claim to be. A plan says what you're doing on Thursday. Weave keeps only the second.",
    "Un perfil dice quién dices ser. Un plan dice qué haces el jueves. Weave solo se queda con lo segundo.",
]);

const ABSENTS_TITRE: Traduit<&str> = Traduit([
    "Ce que Weave ne fait pas",
    "What Weave does not do",
    "Lo que Weave no hace",
]);

const PUBLIE: Traduit<&str> =
    Traduit(["Ce qu'on y publie", "What people post", "Lo que se publica"]);

const PUBLIE_TEXTE: Traduit<&str> = Traduit([
    "Rien de spectaculaire, et c'est voulu : ce qu'on allait faire de toute façon. Un plan réussi, c'est un samedi qu'on n'a pas passé seul.",
    "Nothing spectacular, and that's the point: the thing you were going to do anyway. A plan that worked is a Saturday you didn't spend alone.",
    "Nada espectacular, y es a propósito: lo que ibas a hacer de todos modos. Un plan que sale bien es un sábado que no pasaste solo.",
]);

/// Les guillemets ne sont pas les mêmes d'une langue à l'autre : l'anglais
/// prend des guillemets anglais courbes, le français et l'espagnol des
/// chevrons. Ce sont des caractères du texte, et non de la mise en forme.
const ABSENTS: Traduit<&[&str]> = Traduit([
    &[
        "Pile de cartes à balayer",
        "Liste des personnes qui vous ont remarqué",
        "Mise en avant payante dans le fil",
        "Compteur de « vues » de votre profil",
        "Publicité et revente de données",
        "Notification inventée pour vous faire revenir",
    ] as &[&str],
    &[
        "A pile of cards to swipe",
        "A list of people who noticed you",
        "Paid placement in the feed",
        "A counter of profile \u{201c}views\u{201d}",
        "Advertising and data resale",
        "A notification invented to bring you back",
    ],
    &[
        "Una pila de tarjetas que deslizar",
        "Una lista de quién se ha fijado en ti",
        "Destacados de pago en el muro",
        "Un contador de « visitas » a tu perfil",
        "Publicidad y reventa de datos",
        "Una notificación inventada para hacerte volver",
    ],
]);

struct Pilier {
    titre: &'static str,
    texte: String,
}

fn piliers(langue: Langue) -> [Pilier; 4] {
    match langue {
        Langue::Fr => [
            Pilier {
                titre: "On ne peut pas arroser",
                texte: format!("Le nombre de demandes qu'on peut envoyer dans une journée est borné — au minimum {REQUESTS_PER_DAY_FLOOR}, à toutes les offres, socle gratuit compris. Quand une demande coûte quelque chose, on la choisit."),
            },
            Pilier {
                titre: "On ne peut pas acheter de visibilité",
                texte: "Le fil est trié par imminence puis par proximité. Aucun abonnement, aucun achat ne place un plan devant celui de quelqu'un d'autre. C'est la règle que Weave ne changera pas.".to_string(),
            },
            Pilier {
                titre: "On demande en écrivant",
                texte: format!("Il n'existe aucun geste pour dire « je viens ». On écrit au moins {REQUEST_MIN_CHARS} caractères qui disent pourquoi. C'est ce qui distingue une demande d'un réflexe."),
            },
            Pilier {
                titre: "Un plan a une date, donc une fin",
                texte: format!("Passé le rendez-vous, le plan disparaît du fil. Rien ne s'accumule, rien ne traîne. Et vous n'en gardez que {MAX_OPEN_PLANS} ouverts à la fois : ce que vous comptez vraiment faire."),
            },
        ],
        Langue::En => [
            Pilier {
                titre: "You can't spray and pray",
                texte: format!("The number of requests you can send in a day is capped — at least {REQUESTS_PER_DAY_FLOOR}, on every plan, the free tier included. When a request costs you something, you choose it."),
            },
            Pilier {
                titre: "You can't buy visibility",
                texte: "The feed is sorted by how soon, then by how near. No subscription, no purchase puts one plan ahead of someone else's. That is the rule Weave will not change.".to_string(),
            },
            Pilier {
                titre: "You ask by writing",
                texte: format!("There is no gesture that means \u{201c}I'm coming\u{201d}. You write at least {REQUEST_MIN_CHARS} characters saying why. That is what separates a request from a reflex."),
            },
            Pilier {
                titre: "A plan has a date, so it has an end",
                texte: format!("Once the time has passed, the plan leaves the feed. Nothing piles up, nothing lingers. And you keep only {MAX_OPEN_PLANS} open at a time: the things you actually mean to do."),
            },
        ],
        Langue::Es => [
            Pilier {
                titre: "No se puede lanzar la caña a todo el mundo",
                texte: format!("El número de peticiones que puedes enviar al día está limitado — al menos {REQUESTS_PER_DAY_FLOOR}, en todas las suscripciones, incluida la gratuita. Cuando una petición cuesta algo, la eliges."),
            },
            Pilier {
                titre: "No se puede comprar visibilidad",
                texte: "El muro se ordena por lo que ocurre antes y luego por cercanía. Ninguna suscripción, ninguna compra coloca un plan por delante del de otra persona. Es la regla que Weave no va a cambiar.".to_string(),
            },
            Pilier {
                titre: "Se pide escribiendo",
                texte: format!("No existe ningún gesto para decir « voy ». Escribes al menos {REQUEST_MIN_CHARS} caracteres que digan por qué. Eso es lo que distingue una petición de un reflejo."),
            },
            Pilier {
                titre: "Un plan tiene fecha, así que tiene final",
                texte: format!("Pasada la cita, el plan desaparece del muro. Nada se acumula, nada se queda. Y solo mantienes {MAX_OPEN_PLANS} abiertos a la vez: lo que de verdad piensas hacer."),
            },
        ],
    }
}

/// Les six fils, à la suite.
const FILS: [Fil; 6] = [
    Fil::Un,
    Fil::Deux,
    Fil::Trois,
    Fil::Quatre,
    Fil::Cinq,
    Fil::Six,
];

#[function_component]
pub fn Principe(p: &ProprietesLangue) -> Html {
    let langue = p.langue;
    html! {
        <Section
            id="principe"
            fil={Fil::Quatre}
            titre={*TITRE.choisir(langue)}
            chapeau={*CHAPEAU.choisir(langue)}
            alterne=true
        >
            <div class="grid gap-4 sm:grid-cols-2">
                { for piliers(langue).into_iter().enumerate().map(|(rang, pilier)| {
                    let fil = FILS[rang % FILS.len()];
                    html! {
                        <Carte {fil}>
                            <h3
                                class="text-xl font-bold"
                                style={format!(
                                    "font-family: var(--font-titre); color: {}",
                                    fil.var(),
                                )}
                            >
                                { pilier.titre }
                            </h3>
                            <p class="mt-3 leading-relaxed" style="color: var(--texte-doux)">
                                { pilier.texte }
                            </p>
                        </Carte>
                    }
                }) }
            </div>

            <div class="mt-12 grid gap-8 sm:grid-cols-2">
                <div>
                    <h3 class="text-xl font-semibold" style="font-family: var(--font-titre)">
                        { *ABSENTS_TITRE.choisir(langue) }
                    </h3>
                    <ul class="mt-4 space-y-2.5">
                        { for ABSENTS.choisir(langue).iter().map(|absent| html! {
                            <li class="flex items-start gap-3">
                                <svg
                                    width="20"
                                    height="20"
                                    viewBox="0 0 20 20"
                                    class="mt-0.5 shrink-0"
                                    aria-hidden="true"
                                >
                                    <path
                                        d="M6 6l8 8M14 6l-8 8"
                                        stroke="var(--texte-doux)"
                                        stroke-width="1.6"
                                        stroke-linecap="round"
                                    />
                                </svg>
                                <span style="color: var(--texte-doux)">{ *absent }</span>
                            </li>
                        }) }
                    </ul>
                </div>

                <div>
                    <h3 class="text-xl font-semibold" style="font-family: var(--font-titre)">
                        { *PUBLIE.choisir(langue) }
                    </h3>
                    <p class="mt-4 leading-relaxed" style="color: var(--texte-doux)">
                        { *PUBLIE_TEXTE.choisir(langue) }
                    </p>
                    <ul class="mt-5 flex flex-wrap gap-2">
                        { for contrat::CATEGORIES.iter().enumerate().map(|(rang, categorie)| {
                            let fil = FILS[rang % FILS.len()];
                            html! {
                                // Le texte est à la couleur du texte, pas à
                                // celle du fil.
                                //
                                // Il portait la couleur du fil sur un aplat du
                                // même fil à 16 % : deux tons d'une même
                                // teinte, donc un contraste de 2,7 à 3,9 selon
                                // la couleur — sous le seuil de 4,5 que réclame
                                // un texte de cette taille. Éclaircir l'aplat
                                // n'y suffisait pas : le safran lui-même ne
                                // dépasse pas 3,0 sur du blanc.
                                //
                                // La couleur reste dite par l'aplat ET par le
                                // trait, qui n'ont rien à lire. Le libellé, lui,
                                // se lit.
                                <li
                                    class="rounded-full px-4 py-2 text-sm font-bold"
                                    style={format!(
                                        "background: {}; border: 1.5px solid {}; color: var(--texte)",
                                        fil.teinte(16),
                                        fil.var(),
                                    )}
                                >
                                    { *categorie.libelle().choisir(langue) }
                                </li>
                            }
                        }) }
                    </ul>
                </div>
            </div>
        </Section>
    }
}
