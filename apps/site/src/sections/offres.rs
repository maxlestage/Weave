//! Les offres : quatre abonnements, le socle gratuit, et tout à l'unité.
//!
//! Aucun chiffre n'est écrit ici. Les prix, les droits et les textes de palier
//! viennent de `contrat`, lui-même engendré depuis `packages/contracts` : le
//! site ne peut donc pas annoncer un tarif que l'App Store ne facture pas,
//! du moins pas sans que le contrat ait menti d'abord.

use crate::composants::{Carte, Etiquette, Fil, Section};
use crate::contrat::{self, Criteres, REQUESTS_PER_DAY_FLOOR};
use crate::langues::{Langue, Traduit};
use crate::racine::ProprietesLangue;
use yew::prelude::*;

const TITRE: Traduit<&str> = Traduit([
    "Quatre abonnements, et tout à l'unité",
    "Four subscriptions, and everything sold singly",
    "Cuatro suscripciones, y todo por unidades",
]);

const LE_PLUS: Traduit<&str> = Traduit(["Le plus choisi", "Most chosen", "La más elegida"]);
const GRATUIT: Traduit<&str> = Traduit(["Gratuit", "Free", "Gratis"]);
const PAR_MOIS: Traduit<&str> = Traduit([" / mois", " / month", " / mes"]);
const DEMANDES_PAR_JOUR: Traduit<&str> =
    Traduit(["Demandes par jour", "Requests a day", "Peticiones al día"]);
const PUBLIER: Traduit<&str> = Traduit(["Publier", "Post", "Publicar"]);
const CRITERES: Traduit<&str> = Traduit(["Critères", "Filters", "Criterios"]);
const PLANS_DE_GROUPE: Traduit<&str> =
    Traduit(["Plans de groupe", "Group plans", "Planes de grupo"]);
const OUI: Traduit<&str> = Traduit(["Oui", "Yes", "Sí"]);
const A_L_UNITE: Traduit<&str> = Traduit(["À l'unité", "Sold singly", "Por unidades"]);

const TITRE_UNITES: Traduit<&str> = Traduit([
    "Sans abonnement, à l'unité",
    "Without a subscription, one at a time",
    "Sin suscripción, por unidades",
]);

const CHAPEAU_UNITES: Traduit<&str> = Traduit([
    "Chaque avantage d'un abonnement s'achète aussi séparément — parce qu'à vingt ans, on ne s'abonne pas à tout. On peut utiliser Weave des mois durant sans jamais s'abonner.",
    "Every benefit of a subscription can also be bought on its own — because at twenty you don't subscribe to everything. You can use Weave for months without ever subscribing.",
    "Cada ventaja de una suscripción se puede comprar también por separado — porque a los veinte años uno no se suscribe a todo. Se puede usar Weave durante meses sin suscribirse nunca.",
]);

const FIN: Traduit<&str> = Traduit([
    "Les abonnements se souscrivent et se résilient depuis les réglages de votre compte Apple. Aucun moyen de paiement ne transite par nos serveurs.",
    "Subscriptions are taken out and cancelled from your Apple account settings. No payment details pass through our servers.",
    "Las suscripciones se contratan y se cancelan desde los ajustes de tu cuenta de Apple. Ningún medio de pago pasa por nuestros servidores.",
]);

/// Le chapeau de la section cite le plancher de demandes du contrat.
fn chapeau(langue: Langue) -> String {
    let plancher = REQUESTS_PER_DAY_FLOOR;
    match langue {
        Langue::Fr => format!(
            "Aucune offre n'achète de visibilité : payer ne fait jamais remonter un plan. Ce qui se paie, c'est l'horizon de publication, la finesse des critères et les plans de groupe. Le nombre de demandes reste borné partout — au minimum {plancher} par jour."
        ),
        Langue::En => format!(
            "No plan buys visibility: paying never pushes a plan up the feed. What you pay for is how far ahead you can post, how precise the filters are, and group plans. The number of requests stays capped everywhere — at least {plancher} a day."
        ),
        Langue::Es => format!(
            "Ninguna suscripción compra visibilidad: pagar nunca sube un plan en el muro. Lo que se paga es con cuánta antelación puedes publicar, lo finos que son los criterios y los planes de grupo. El número de peticiones sigue limitado en todas — al menos {plancher} al día."
        ),
    }
}

/// Le libellé d'une finesse de critères.
///
/// Un `match` sur l'énumération du contrat, et non sur une chaîne : la version
/// TypeScript indexait un `Record<FilterDepth, string>`, ce qui tenait la même
/// garantie. Ici, une finesse ajoutée au contrat ne compile pas tant que les
/// trois langues ne l'ont pas nommée.
fn libelle_criteres(langue: Langue, criteres: Criteres) -> &'static str {
    match (langue, criteres) {
        (Langue::Fr, Criteres::Base) => "De base",
        (Langue::Fr, Criteres::Etendus) => "Étendus",
        (Langue::Fr, Criteres::Precis) => "Précis",
        (Langue::En, Criteres::Base) => "Basic",
        (Langue::En, Criteres::Etendus) => "Extended",
        (Langue::En, Criteres::Precis) => "Precise",
        (Langue::Es, Criteres::Base) => "Básicos",
        (Langue::Es, Criteres::Etendus) => "Ampliados",
        (Langue::Es, Criteres::Precis) => "Precisos",
    }
}

/// L'horizon de publication est un nombre de jours ; on le dit en semaines ou
/// en mois quand c'en est un compte rond, parce que « 60 jours à l'avance » ne
/// se lit pas comme on le pense.
enum Horizon {
    Jours(u32),
    Semaines(u32),
    Mois(u32),
}

fn decouper_horizon(jours: u32) -> Horizon {
    // La division arrondie, comme `Math.round` du côté TypeScript : 10 jours
    // font « une semaine » et non « une semaine et des miettes », et c'est
    // volontaire — l'horizon annoncé est un ordre de grandeur, le droit exact
    // reste celui du contrat.
    if jours >= 30 {
        Horizon::Mois((jours + 15) / 30)
    } else if jours >= 7 {
        Horizon::Semaines((jours + 3) / 7)
    } else {
        Horizon::Jours(jours)
    }
}

fn dire_horizon(langue: Langue, horizon: Horizon) -> String {
    match (langue, horizon) {
        (Langue::Fr, Horizon::Mois(1)) => "un mois à l'avance".to_string(),
        (Langue::Fr, Horizon::Mois(n)) => format!("{n} mois à l'avance"),
        (Langue::Fr, Horizon::Semaines(1)) => "une semaine à l'avance".to_string(),
        (Langue::Fr, Horizon::Semaines(n)) => format!("{n} semaines à l'avance"),
        (Langue::Fr, Horizon::Jours(1)) => "un jour à l'avance".to_string(),
        (Langue::Fr, Horizon::Jours(n)) => format!("{n} jours à l'avance"),
        (Langue::En, Horizon::Mois(1)) => "a month ahead".to_string(),
        (Langue::En, Horizon::Mois(n)) => format!("{n} months ahead"),
        (Langue::En, Horizon::Semaines(1)) => "a week ahead".to_string(),
        (Langue::En, Horizon::Semaines(n)) => format!("{n} weeks ahead"),
        (Langue::En, Horizon::Jours(1)) => "a day ahead".to_string(),
        (Langue::En, Horizon::Jours(n)) => format!("{n} days ahead"),
        (Langue::Es, Horizon::Mois(1)) => "un mes de antelación".to_string(),
        (Langue::Es, Horizon::Mois(n)) => format!("{n} meses de antelación"),
        (Langue::Es, Horizon::Semaines(1)) => "una semana de antelación".to_string(),
        (Langue::Es, Horizon::Semaines(n)) => format!("{n} semanas de antelación"),
        (Langue::Es, Horizon::Jours(1)) => "un día de antelación".to_string(),
        (Langue::Es, Horizon::Jours(n)) => format!("{n} días de antelación"),
    }
}

/// Chaque palier porte sa couleur.
///
/// La clé vient du contrat, où c'est une chaîne ; le cas par défaut reprend
/// donc celui de la version TypeScript (`?? 6`). Un palier ajouté au contrat
/// s'affiche en sixième fil jusqu'à ce qu'on lui en choisisse un — ce qui est
/// moins grave que de ne pas s'afficher.
fn couleur(cle: &str) -> Fil {
    match cle {
        "depart" => Fil::Quatre,
        "viree" => Fil::Trois,
        "escapade" => Fil::Six,
        "expedition" => Fil::Cinq,
        "grandtour" => Fil::Un,
        _ => Fil::Six,
    }
}

/// Les six fils, à la suite, pour les produits à l'unité.
const FILS: [Fil; 6] = [
    Fil::Un,
    Fil::Deux,
    Fil::Trois,
    Fil::Quatre,
    Fil::Cinq,
    Fil::Six,
];

/// Une ligne du tableau des droits.
fn ligne(terme: &str, valeur: Html, classe_valeur: &'static str) -> Html {
    html! {
        <div class="flex justify-between gap-3">
            <dt style="color: var(--texte-doux)">{ terme.to_string() }</dt>
            <dd class={classe_valeur}>{ valeur }</dd>
        </div>
    }
}

#[function_component]
pub fn Offres(p: &ProprietesLangue) -> Html {
    let langue = p.langue;

    html! {
        <Section
            id="offres"
            fil={Fil::Six}
            titre={*TITRE.choisir(langue)}
            chapeau={chapeau(langue)}
        >
            <div class="mt-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
                { for contrat::PALIERS.iter().map(|palier| {
                    let fil = couleur(palier.cle);
                    let gratuit = palier.prix_centimes == 0;
                    let accentuee = palier.cle == "escapade";
                    html! {
                        <Carte {fil} {accentuee}>
                            <div class="flex items-baseline justify-between gap-3">
                                <h3
                                    class="text-2xl font-bold"
                                    style={format!(
                                        "font-family: var(--font-titre); color: {}",
                                        fil.var(),
                                    )}
                                >
                                    { palier.nom }
                                </h3>
                                if accentuee {
                                    <Etiquette {fil}>{ *LE_PLUS.choisir(langue) }</Etiquette>
                                }
                            </div>

                            <p class="mt-2 text-sm" style="color: var(--texte-doux)">
                                { *palier.accroche.choisir(langue) }
                            </p>

                            <p class="mt-5">
                                <span class="text-3xl font-semibold tabular-nums">
                                    { if gratuit {
                                        *GRATUIT.choisir(langue)
                                    } else {
                                        *palier.prix.choisir(langue)
                                    } }
                                </span>
                                // Vide quand c'est gratuit : « 0,00 € / mois »
                                // se lirait comme un abonnement à zéro euro,
                                // ce qui n'est pas la même chose qu'un socle
                                // sans abonnement du tout.
                                <span class="text-sm" style="color: var(--texte-doux)">
                                    { if gratuit { "" } else { *PAR_MOIS.choisir(langue) } }
                                </span>
                            </p>

                            <dl class="mt-5 space-y-1.5 text-sm">
                                { ligne(
                                    DEMANDES_PAR_JOUR.choisir(langue),
                                    html! { { palier.demandes_par_jour } },
                                    "font-semibold tabular-nums",
                                ) }
                                { ligne(
                                    PUBLIER.choisir(langue),
                                    html! { { dire_horizon(
                                        langue,
                                        decouper_horizon(palier.jours_a_l_avance),
                                    ) } },
                                    "text-right font-semibold",
                                ) }
                                { ligne(
                                    CRITERES.choisir(langue),
                                    html! { { libelle_criteres(langue, palier.criteres) } },
                                    "font-semibold",
                                ) }
                                { ligne(
                                    PLANS_DE_GROUPE.choisir(langue),
                                    html! { { if palier.plans_de_groupe {
                                        *OUI.choisir(langue)
                                    } else {
                                        *A_L_UNITE.choisir(langue)
                                    } } },
                                    "font-semibold",
                                ) }
                            </dl>

                            <ul class="mt-5 space-y-2 text-sm">
                                { for palier.atouts.choisir(langue).iter().map(|point| html! {
                                    <li class="flex gap-2.5">
                                        <span
                                            aria-hidden="true"
                                            style={format!("color: {}", fil.var())}
                                        >
                                            { "—" }
                                        </span>
                                        <span style="color: var(--texte-doux)">{ *point }</span>
                                    </li>
                                }) }
                            </ul>
                        </Carte>
                    }
                }) }
            </div>

            <h3 class="mt-14 text-2xl font-bold" style="font-family: var(--font-titre)">
                { *TITRE_UNITES.choisir(langue) }
            </h3>
            <p class="mt-3 max-w-2xl leading-relaxed" style="color: var(--texte-doux)">
                { *CHAPEAU_UNITES.choisir(langue) }
            </p>

            <ul class="mt-6 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                { for contrat::PRODUITS_UNITE.iter().enumerate().map(|(rang, produit)| {
                    let fil = FILS[rang % FILS.len()];
                    html! {
                        <li
                            class="rounded-2xl p-4"
                            style={format!(
                                "background: var(--carte); border: 2px solid {}",
                                fil.var(),
                            )}
                        >
                            <div class="flex items-baseline justify-between gap-3">
                                <span class="font-bold">{ produit.nom }</span>
                                <span
                                    class="text-sm font-bold tabular-nums"
                                    style={format!("color: {}", fil.var())}
                                >
                                    { *produit.prix.choisir(langue) }
                                </span>
                            </div>
                            <p class="mt-2 text-sm leading-relaxed" style="color: var(--texte-doux)">
                                { *produit.description.choisir(langue) }
                            </p>
                        </li>
                    }
                }) }
            </ul>

            <p class="mt-8 text-sm" style="color: var(--texte-doux)">
                { *FIN.choisir(langue) }
            </p>
        </Section>
    }
}
