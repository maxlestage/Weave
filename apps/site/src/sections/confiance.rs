//! « Ce à quoi nous nous engageons » — et les trois durées qui le chiffrent.
//!
//! Les trois durées viennent du contrat : l'âge minimum, la conservation des
//! messages et le délai de purge d'un compte supprimé. Elles sont aussi dans
//! la politique de confidentialité et dans le code de l'API. Les écrire en dur
//! ici en ferait une promesse que rien ne tient.

use crate::composants::{Carte, Fil, Section};
use crate::contrat::{ACCOUNT_PURGE_DAYS, MESSAGE_RETENTION_DAYS, MIN_AGE};
use crate::langues::{Langue, Traduit};
use crate::racine::ProprietesLangue;
use yew::prelude::*;

const TITRE: Traduit<&str> = Traduit([
    "Ce à quoi nous nous engageons",
    "What we commit to",
    "A qué nos comprometemos",
]);

const CHAPEAU: Traduit<&str> = Traduit([
    "Une application où l'on donne son heure et son lieu manipule ce qu'il y a de plus sensible. Voici ce que nous nous interdisons.",
    "An app where you give away your time and your place handles the most sensitive thing there is. Here is what we rule out for ourselves.",
    "Una aplicación en la que das tu hora y tu sitio maneja lo más sensible que hay. Esto es lo que nos prohibimos.",
]);

const AGE: Traduit<&str> = Traduit(["Âge minimum", "Minimum age", "Edad mínima"]);
const MESSAGES: Traduit<&str> = Traduit([
    "Messages conservés",
    "Messages kept",
    "Mensajes conservados",
]);
const APRES_CLOTURE: Traduit<&str> = Traduit([
    "après clôture d'une conversation",
    "after a conversation is closed",
    "tras cerrar una conversación",
]);
const COMPTE: Traduit<&str> = Traduit(["Compte supprimé", "Deleted account", "Cuenta eliminada"]);
const AVANT_PURGE: Traduit<&str> = Traduit([
    "avant purge définitive",
    "before permanent purge",
    "antes del borrado definitivo",
]);

/// Un âge, dit dans la langue. L'anglais ne compte pas les années : « 18 and
/// over » et non « 18 years », parce que c'est un seuil et non une durée.
fn ans(langue: Langue, nombre: u32) -> String {
    match langue {
        Langue::Fr => format!("{nombre} ans"),
        Langue::En => format!("{nombre} and over"),
        Langue::Es => format!("{nombre} años"),
    }
}

fn jours(langue: Langue, nombre: u32) -> String {
    match langue {
        Langue::Fr => format!("{nombre} jours"),
        Langue::En => format!("{nombre} days"),
        Langue::Es => format!("{nombre} días"),
    }
}

struct Engagement {
    titre: &'static str,
    texte: &'static str,
}

const ENGAGEMENTS: Traduit<&[Engagement]> = Traduit([
    &[
        Engagement {
            titre: "Rien à vendre à personne",
            texte: "Pas de publicité, pas de courtier en données, pas de revente. Le produit est financé par celles et ceux qui s'abonnent ou achètent à l'unité.",
        },
        Engagement {
            titre: "Localisation au kilomètre",
            texte: "Votre position est arrondie avant d'être enregistrée, et un plan n'affiche jamais d'adresse dans le fil : une ville, une distance arrondie. L'endroit exact se dit dans la conversation, à qui vous avez accepté.",
        },
        Engagement {
            titre: "Ce que vous consultez n'est pas archivé",
            texte: "Le fil est recomposé à la demande et vit quelques minutes dans un cache. Nous ne constituons pas d'historique des plans que vous avez regardés, et personne ne peut le consulter.",
        },
        Engagement {
            titre: "Blocage immédiat",
            texte: "Bloquer ou signaler coupe tout des deux côtés — demandes en attente closes, conversation fermée — sans notification à l'autre personne. Un signalement entraîne toujours un blocage.",
        },
    ] as &[Engagement],
    &[
        Engagement {
            titre: "Nothing to sell to anyone",
            texte: "No advertising, no data brokers, no resale. The product is paid for by the people who subscribe or buy things singly.",
        },
        Engagement {
            titre: "Location to the kilometre",
            texte: "Your position is rounded before it is stored, and a plan never shows an address in the feed: a town, a rounded distance. The exact place is said in the conversation, to whoever you accepted.",
        },
        Engagement {
            titre: "What you look at is not kept",
            texte: "The feed is rebuilt on demand and lives a few minutes in a cache. We build no history of the plans you looked at, and nobody can consult one.",
        },
        Engagement {
            titre: "Blocking takes effect at once",
            texte: "Blocking or reporting cuts everything off both ways — pending requests closed, conversation shut — with no notification to the other person. A report always carries a block with it.",
        },
    ],
    &[
        Engagement {
            titre: "Nada que vender a nadie",
            texte: "Sin publicidad, sin intermediarios de datos, sin reventa. El producto lo financian quienes se suscriben o compran por unidades.",
        },
        Engagement {
            titre: "Ubicación al kilómetro",
            texte: "Tu posición se redondea antes de guardarse, y un plan nunca muestra una dirección en el muro: una ciudad, una distancia redondeada. El sitio exacto se dice en la conversación, a quien hayas aceptado.",
        },
        Engagement {
            titre: "Lo que consultas no se archiva",
            texte: "El muro se recompone bajo demanda y vive unos minutos en una caché. No creamos ningún historial de los planes que has mirado, y nadie puede consultarlo.",
        },
        Engagement {
            titre: "Bloqueo inmediato",
            texte: "Bloquear o denunciar corta todo por ambos lados — peticiones pendientes cerradas, conversación cerrada — sin notificar a la otra persona. Una denuncia conlleva siempre un bloqueo.",
        },
    ],
]);

/// Les six fils, à la suite.
const FILS: [Fil; 6] = [
    Fil::Un,
    Fil::Deux,
    Fil::Trois,
    Fil::Quatre,
    Fil::Cinq,
    Fil::Six,
];

/// Un chiffre et sa légende, dans la liste du bas.
fn chiffre(terme: &str, valeur: String, legende: Option<&str>) -> Html {
    html! {
        <div>
            <dt class="text-sm" style="color: var(--texte-doux)">{ terme.to_string() }</dt>
            <dd class="mt-1 text-2xl font-semibold tabular-nums">
                { valeur }
                if let Some(legende) = legende {
                    <span
                        class="mt-1 block text-sm font-normal"
                        style="color: var(--texte-doux)"
                    >
                        { legende.to_string() }
                    </span>
                }
            </dd>
        </div>
    }
}

#[function_component]
pub fn Confiance(p: &ProprietesLangue) -> Html {
    let langue = p.langue;
    html! {
        <Section
            id="confiance"
            fil={Fil::Un}
            titre={*TITRE.choisir(langue)}
            chapeau={*CHAPEAU.choisir(langue)}
            alterne=true
        >
            <div class="grid gap-4 sm:grid-cols-2">
                { for ENGAGEMENTS.choisir(langue).iter().enumerate().map(|(rang, engagement)| {
                    let fil = FILS[rang % FILS.len()];
                    html! {
                        <Carte {fil}>
                            <h3 class="text-lg font-bold" style={format!("color: {}", fil.var())}>
                                { engagement.titre }
                            </h3>
                            <p class="mt-2.5 leading-relaxed" style="color: var(--texte-doux)">
                                { engagement.texte }
                            </p>
                        </Carte>
                    }
                }) }
            </div>

            <dl class="mt-10 grid gap-6 sm:grid-cols-3">
                { chiffre(AGE.choisir(langue), ans(langue, MIN_AGE), None) }
                { chiffre(
                    MESSAGES.choisir(langue),
                    jours(langue, MESSAGE_RETENTION_DAYS),
                    Some(APRES_CLOTURE.choisir(langue)),
                ) }
                { chiffre(
                    COMPTE.choisir(langue),
                    jours(langue, ACCOUNT_PURGE_DAYS),
                    Some(AVANT_PURGE.choisir(langue)),
                ) }
            </dl>
        </Section>
    }
}
