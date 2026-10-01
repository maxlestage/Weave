//! Mentions légales.
//!
//! Obligation : article 6-III-1 de la loi pour la confiance dans l'économie
//! numérique. Un éditeur professionnel doit publier son identité, ses
//! coordonnées, son immatriculation et celles de son hébergeur, de manière
//! directement accessible. L'absence de ces mentions est pénalement
//! sanctionnée — d'où le surlignage voyant de tout ce qui manque.

use crate::identite::{CONTACT, EDITEUR};
use crate::pages::page::{AC, Article, Page, mise_a_jour};
use yew::prelude::*;

/// Une ligne « Libellé : valeur à compléter ».
fn ligne(libelle: &str, valeur: &'static str) -> Html {
    html! {
        <li>{ format!("{libelle} : ") }<AC>{ valeur }</AC></li>
    }
}

/// Une adresse de courriel, cliquable et surlignée tant qu'elle manque.
fn courriel(adresse: &'static str) -> Html {
    html! {
        <a href={format!("mailto:{adresse}")}><AC>{ adresse }</AC></a>
    }
}

fn articles() -> Vec<Article> {
    vec![
        Article {
            id: "editeur",
            titre: "Éditeur du site et de l'application",
            contenu: html! {
                <>
                    <ul>
                        { ligne("Dénomination", EDITEUR.raison_sociale) }
                        { ligne("Forme juridique et capital", EDITEUR.forme_juridique) }
                        { ligne("Siège social", EDITEUR.adresse) }
                        { ligne("Immatriculation", EDITEUR.immatriculation) }
                        { ligne("TVA intracommunautaire", EDITEUR.tva) }
                        { ligne("Téléphone", EDITEUR.telephone) }
                        <li>{ "Courriel : " }{ courriel(CONTACT.support) }</li>
                    </ul>
                </>
            },
        },
        Article {
            id: "publication",
            titre: "Directeur de la publication",
            contenu: html! {
                <p><AC>{ EDITEUR.directeur_publication }</AC></p>
            },
        },
        Article {
            id: "hebergeur",
            titre: "Hébergeur",
            contenu: html! {
                <ul>
                    { ligne("Nom", EDITEUR.hebergeur.nom) }
                    { ligne("Adresse", EDITEUR.hebergeur.adresse) }
                    { ligne("Téléphone", EDITEUR.hebergeur.telephone) }
                </ul>
            },
        },
        Article {
            id: "signalements",
            titre: "Point de contact et signalements",
            contenu: html! {
                <>
                    <p>
                        { "Le règlement européen sur les services numériques impose un point de contact unique pour les signalements de contenus illicites. Vous pouvez nous écrire à " }
                        { courriel(CONTACT.signalements) }
                        { "." }
                    </p>
                    <p>
                        { "Dans l'application, chaque profil, plan et conversation dispose d'une fonction de signalement, qui entraîne toujours un blocage immédiat de la personne signalée. Les décisions de modération peuvent être contestées à la même adresse." }
                    </p>
                </>
            },
        },
        Article {
            id: "donnees",
            titre: "Données personnelles",
            contenu: html! {
                <p>
                    { "Le traitement de vos données est décrit dans la " }
                    <a href="/confidentialite">{ "politique de confidentialité" }</a>
                    { ", qui précise les finalités, les bases légales, les durées de conservation et la manière d'exercer vos droits." }
                </p>
            },
        },
        Article {
            id: "propriete",
            titre: "Propriété intellectuelle",
            contenu: html! {
                <>
                    <p>
                        { "La marque Weave, le nom de domaine, le logo, l'identité visuelle, les textes et le code de l'application sont la propriété de " }
                        <AC>{ EDITEUR.raison_sociale }</AC>
                        { ", sauf mention contraire." }
                    </p>
                    <p>
                        { "Les contenus que vous publiez — plans, messages, photos — restent les vôtres. Vous nous concédez uniquement le droit de les afficher aux personnes concernées dans le cadre du fonctionnement du service, conformément aux " }
                        <a href="/cgu">{ "conditions générales d'utilisation" }</a>
                        { "." }
                    </p>
                    <p>{ "Apple, iPhone et Apple Watch sont des marques déposées d'Apple Inc." }</p>
                </>
            },
        },
        Article {
            id: "accessibilite",
            titre: "Accessibilité",
            contenu: html! {
                <p>
                    { "Ce site est conçu pour rester utilisable au clavier, avec un lecteur d'écran, et à fort grossissement. Si vous rencontrez un obstacle, écrivez-nous à " }
                    { courriel(CONTACT.support) }
                    { " : nous le corrigerons." }
                </p>
            },
        },
    ]
}

#[function_component]
pub fn MentionsLegales() -> Html {
    html! {
        <Page
            titre="Mentions légales"
            chapeau="Qui édite Weave, qui l'héberge, et à qui écrire."
            mise_a_jour={mise_a_jour("mentions-legales")}
            articles={articles()}
        />
    }
}
