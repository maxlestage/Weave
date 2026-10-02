//! L'ouverture : le titre, les deux boutons, et l'illustration du fil.
//!
//! C'est la seule section qui n'emploie pas le composant `Section` : elle n'a
//! ni trait de couleur ni titre de niveau deux, et elle porte sa propre trame
//! de fond. Elle garde en revanche le même `id` que du côté React — `#haut` —
//! parce que c'est l'ancre du lien « aller en haut » et du menu.

use crate::composants::{Etiquette, Fil};
use crate::contrat::{Categorie, MAX_OPEN_PLANS, MIN_AGE};
use crate::langues::{Langue, Traduit};
use crate::racine::ProprietesLangue;
use yew::prelude::*;

const BIENTOT: Traduit<&str> = Traduit([
    "Bientôt sur iPhone et Apple Watch",
    "Coming soon to iPhone and Apple Watch",
    "Pronto en iPhone y Apple Watch",
]);

/// Le titre, découpé en mots — un fil de couleur par mot.
///
/// Le découpage n'est pas le même d'une langue à l'autre : le français en a
/// cinq, l'anglais et l'espagnol trois. Ce n'est pas un détail de mise en
/// forme : la couleur suit les mots, donc elle suit la traduction.
const TITRE: Traduit<&[&str]> = Traduit([
    &["Des", "plans,", "pas", "des", "profils."] as &[&str],
    &["Plans,", "not", "profiles."],
    &["Planes,", "no", "perfiles."],
]);

const ACCROCHE: Traduit<&str> = Traduit([
    "Sur Weave, on ne se décrit pas : on écrit ce qu'on compte faire jeudi soir. Les autres demandent à venir — en disant pourquoi. Pas de cartes à balayer, pas de « il/elle vous a remarqué », pas de file d'attente.",
    "On Weave you don't describe yourself: you write down what you're doing on Thursday evening. Other people ask to come — and say why. No cards to swipe, no \u{201c}someone noticed you\u{201d}, no queue.",
    "En Weave no te describes: escribes lo que piensas hacer el jueves por la noche. Los demás piden venir — y dicen por qué. Sin tarjetas que deslizar, sin « alguien se ha fijado en ti », sin cola de espera.",
]);

const OFFRES: Traduit<&str> =
    Traduit(["Voir les offres", "See the plans", "Ver las suscripciones"]);

const PRINCIPE: Traduit<&str> =
    Traduit(["Comprendre le principe", "How it works", "Cómo funciona"]);

const ILLUSTRATION: Traduit<&str> = Traduit([
    "Illustration du fil : trois plans à venir",
    "Illustration of the feed: three upcoming plans",
    "Ilustración del muro: tres planes próximos",
]);

fn mentions(langue: Langue, age: u32) -> String {
    match langue {
        Langue::Fr => {
            format!("Gratuit pour commencer · Sans publicité · Réservé aux {age} ans et plus")
        }
        Langue::En => format!("Free to start · No advertising · {age} and over only"),
        Langue::Es => format!("Gratis para empezar · Sin publicidad · Solo mayores de {age} años"),
    }
}

fn tri(langue: Langue, plans: u32) -> String {
    match langue {
        Langue::Fr => format!(
            "Le fil est trié par ce qui arrive le plus tôt, puis par ce qui est le plus près. Rien d'autre : aucun abonnement ne fait remonter un plan. Et vous publiez au plus {plans} plans à la fois."
        ),
        Langue::En => format!(
            "The feed is sorted by what happens soonest, then by what is nearest. Nothing else: no subscription pushes a plan up. And you post at most {plans} plans at a time."
        ),
        Langue::Es => format!(
            "El muro se ordena por lo que ocurre antes, y luego por lo que está más cerca. Nada más: ninguna suscripción sube un plan. Y publicas como máximo {plans} planes a la vez."
        ),
    }
}

/// Les couleurs des mots du titre, dans l'ordre.
const FILS_DU_TITRE: [Fil; 5] = [Fil::Un, Fil::Deux, Fil::Quatre, Fil::Cinq, Fil::Six];

/// Un plan de l'illustration.
struct PlanIllustre {
    titre: &'static str,
    note: &'static str,
    auteur: &'static str,
    quand: &'static str,
    ou: &'static str,
    places: &'static str,
    categorie: Categorie,
    fil: Fil,
}

/// Les trois plans de l'illustration, déclinés.
///
/// Rien de réel n'y figure : ce sont des plans inventés, avec des prénoms
/// inventés. Une capture d'écran aurait montré les plans de quelqu'un.
const PLANS_ILLUSTRES: Traduit<&[PlanIllustre]> = Traduit([
    &[
        PlanIllustre {
            titre: "Bloc au mur de 19 h, niveau débutant",
            note: "Je grimpe depuis six mois, très mal.",
            auteur: "Théo, 23 ans",
            quand: "Jeudi 19 h",
            ou: "à 2 km",
            places: "1 place",
            categorie: Categorie::Sport,
            fil: Fil::Un,
        },
        PlanIllustre {
            titre: "Concert d'un groupe que personne ne connaît",
            note: "Petite salle, 8 € à l'entrée.",
            auteur: "Sofia, 21 ans",
            quand: "Vendredi 20 h 30",
            ou: "à 4 km",
            places: "2 places",
            categorie: Categorie::Musique,
            fil: Fil::Quatre,
        },
        PlanIllustre {
            titre: "Marché puis brunch, sans se presser",
            note: "Venez si vous aimez goûter dix choses avant d'acheter.",
            auteur: "Alex, 24 ans",
            quand: "Samedi 10 h",
            ou: "à 1 km",
            places: "2 places",
            categorie: Categorie::Repas,
            fil: Fil::Cinq,
        },
    ] as &[PlanIllustre],
    &[
        PlanIllustre {
            titre: "Bouldering at the 7pm wall, beginners welcome",
            note: "I've been climbing for six months, very badly.",
            auteur: "Théo, 23",
            quand: "Thursday 7pm",
            ou: "2 km away",
            places: "1 spot",
            categorie: Categorie::Sport,
            fil: Fil::Un,
        },
        PlanIllustre {
            titre: "A gig by a band nobody has heard of",
            note: "Small venue, £8 on the door.",
            auteur: "Sofia, 21",
            quand: "Friday 8.30pm",
            ou: "4 km away",
            places: "2 spots",
            categorie: Categorie::Musique,
            fil: Fil::Quatre,
        },
        PlanIllustre {
            titre: "Market, then a slow brunch",
            note: "Come along if you like tasting ten things before buying one.",
            auteur: "Alex, 24",
            quand: "Saturday 10am",
            ou: "1 km away",
            places: "2 spots",
            categorie: Categorie::Repas,
            fil: Fil::Cinq,
        },
    ],
    &[
        PlanIllustre {
            titre: "Bloque en el muro de las 19 h, nivel principiante",
            note: "Llevo seis meses escalando, fatal.",
            auteur: "Théo, 23 años",
            quand: "Jueves 19 h",
            ou: "a 2 km",
            places: "1 plaza",
            categorie: Categorie::Sport,
            fil: Fil::Un,
        },
        PlanIllustre {
            titre: "Concierto de un grupo que no conoce nadie",
            note: "Sala pequeña, 8 € en la puerta.",
            auteur: "Sofia, 21 años",
            quand: "Viernes 20.30 h",
            ou: "a 4 km",
            places: "2 plazas",
            categorie: Categorie::Musique,
            fil: Fil::Quatre,
        },
        PlanIllustre {
            titre: "Mercado y luego brunch, sin prisa",
            note: "Ven si te gusta probar diez cosas antes de comprar una.",
            auteur: "Alex, 24 años",
            quand: "Sábado 10 h",
            ou: "a 1 km",
            places: "2 plazas",
            categorie: Categorie::Repas,
            fil: Fil::Cinq,
        },
    ],
]);

/// Représentation du fil : trois plans à venir, tels qu'on les verrait.
#[function_component]
fn FilIllustre(p: &ProprietesLangue) -> Html {
    let langue = p.langue;
    html! {
        <div class="mt-14" aria-label={*ILLUSTRATION.choisir(langue)} role="img">
            <div class="grid gap-4 sm:grid-cols-3">
                { for PLANS_ILLUSTRES.choisir(langue).iter().map(|plan| html! {
                    <article
                        class="flex flex-col overflow-hidden rounded-3xl"
                        style={format!(
                            "background: var(--carte); border: 2px solid {}",
                            plan.fil.var(),
                        )}
                    >
                        <div
                            class="h-2"
                            style={format!("background: {}", plan.fil.var())}
                            aria-hidden="true"
                        />

                        <div class="flex flex-1 flex-col p-5">
                            <div class="flex items-baseline justify-between gap-3">
                                <span
                                    class="text-xs font-bold tracking-wide uppercase"
                                    style={format!("color: {}", plan.fil.var())}
                                >
                                    { *plan.categorie.libelle().choisir(langue) }
                                </span>
                                <span
                                    class="shrink-0 text-sm font-bold tabular-nums"
                                    style="color: var(--texte-doux)"
                                >
                                    { plan.ou }
                                </span>
                            </div>

                            <h3 class="mt-2 text-base leading-snug font-bold">{ plan.titre }</h3>

                            <p class="mt-2 flex-1 text-sm" style="color: var(--texte-doux)">
                                { plan.note }
                            </p>

                            <p
                                class="mt-4 text-sm font-bold"
                                style={format!("color: {}", plan.fil.var())}
                            >
                                { plan.quand }{ " · " }{ plan.places }
                            </p>
                            <p class="mt-1 text-sm" style="color: var(--texte-doux)">
                                { plan.auteur }
                            </p>
                        </div>
                    </article>
                }) }
            </div>

            <p class="mt-4 text-sm" style="color: var(--texte-doux)">
                { tri(langue, MAX_OPEN_PLANS) }
            </p>
        </div>
    }
}

#[function_component]
pub fn Ouverture(p: &ProprietesLangue) -> Html {
    let langue = p.langue;
    html! {
        <section
            id="haut"
            class="relative overflow-hidden px-5 pt-12 pb-16 sm:px-8 sm:pt-20 sm:pb-24"
        >
            <div class="trame pointer-events-none absolute inset-0 opacity-40" aria-hidden="true" />

            <div class="relative mx-auto w-full max-w-5xl">
                <Etiquette fil={Fil::Six}>{ *BIENTOT.choisir(langue) }</Etiquette>

                // Un fil par mot, plutôt qu'un dégradé : le dégradé coupait les
                // lettres au milieu et la couleur paraissait accidentelle. Là,
                // elle est voulue.
                <h1
                    class="mt-6 text-[2.3rem] leading-[1.05] font-bold tracking-tight sm:text-6xl lg:text-7xl"
                    style="font-family: var(--font-titre)"
                >
                    { for TITRE.choisir(langue).iter().enumerate().map(|(rang, mot)| html! {
                        <span
                            style={format!(
                                "color: {}",
                                FILS_DU_TITRE[rang % FILS_DU_TITRE.len()].var(),
                            )}
                        >
                            { if rang > 0 { " " } else { "" } }
                            { *mot }
                        </span>
                    }) }
                </h1>

                <p class="mt-6 max-w-xl text-lg leading-relaxed sm:text-xl">
                    { *ACCROCHE.choisir(langue) }
                </p>

                <div class="mt-9 flex flex-col gap-3 sm:flex-row sm:items-center">
                    <a
                        href="#offres"
                        class="inline-flex items-center justify-center rounded-full px-7 py-4 text-base font-bold"
                        style="background: var(--accent); color: var(--sur-accent)"
                    >
                        { *OFFRES.choisir(langue) }
                    </a>
                    <a
                        href="#principe"
                        class="inline-flex items-center justify-center rounded-full px-7 py-4 text-base font-bold"
                        style={format!(
                            "border: 2px solid {}; color: var(--texte)",
                            Fil::Quatre.var(),
                        )}
                    >
                        { *PRINCIPE.choisir(langue) }
                    </a>
                </div>

                <p class="mt-5 text-sm font-medium" style="color: var(--texte-doux)">
                    { mentions(langue, MIN_AGE) }
                </p>

                <FilIllustre {langue} />
            </div>
        </section>
    }
}
