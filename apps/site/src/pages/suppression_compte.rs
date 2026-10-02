//! Suppression de compte.
//!
//! Apple exige, pour toute application permettant de créer un compte, un moyen
//! de le supprimer depuis l'application. Google Play exige en plus une page web
//! publique, accessible sans installer l'application ni se connecter, décrivant
//! la marche à suivre et ce qui est effacé. C'est cette page.
//!
//! Elle sert aussi le droit à l'effacement de l'article 17 du RGPD, et vaut
//! mieux qu'un paragraphe perdu au milieu de la politique de confidentialité :
//! quelqu'un qui veut partir ne doit pas avoir à chercher.

use crate::contrat::ACCOUNT_PURGE_DAYS;
use crate::identite::CONTACT;
use crate::pages::page::{AC, Article, Page, mise_a_jour};
use yew::prelude::*;

fn articles() -> Vec<Article> {
    vec![
        Article {
            id: "application",
            titre: "Depuis l'application",
            contenu: html! {
                <>
                    <p>{ "C'est la voie la plus rapide, et elle ne demande aucune démarche auprès de nous." }</p>
                    <ol>
                        <li>{ "Ouvrez Weave et allez dans « Réglages »." }</li>
                        <li>{ "Descendez jusqu'à « Supprimer mon compte »." }</li>
                        <li>{ "Confirmez : la demande est prise en compte immédiatement." }</li>
                    </ol>
                    <p>
                        <strong>{ "Vos plans ouverts disparaissent du fil sur-le-champ." }</strong>
                        { " Personne ne peut plus vous envoyer de demande, ni vous trouver." }
                    </p>
                </>
            },
        },
        Article {
            id: "courriel",
            titre: "Par courriel, si vous n'avez plus l'application",
            contenu: html! {
                <>
                    <p>
                        { "Écrivez à " }
                        <a href={format!("mailto:{}", CONTACT.confidentialite)}>
                            <AC>{ CONTACT.confidentialite }</AC>
                        </a>
                        { " depuis l'adresse associée à votre compte. Nous vérifions qu'il s'agit bien de vous, puis procédons à la suppression." }
                    </p>
                    <p>
                        { "Nous répondons dans un délai d'un mois au plus, et en pratique bien plus vite. Si vous écrivez depuis une autre adresse, nous vous demanderons un élément permettant de confirmer que le compte est le vôtre — c'est une protection contre la suppression du compte d'autrui." }
                    </p>
                </>
            },
        },
        Article {
            id: "efface",
            titre: "Ce qui est effacé",
            contenu: html! {
                <>
                    <p>
                        { "Sous " }
                        <strong>{ format!("{ACCOUNT_PURGE_DAYS} jours") }</strong>
                        { ", tout ce qui vous concerne est supprimé de nos bases :" }
                    </p>
                    <ul>
                        <li>{ "votre compte, votre adresse e-mail et vos identifiants de connexion ;" }</li>
                        <li>{ "votre profil : ville, position arrondie, genre, présentation, photo ;" }</li>
                        <li>{ "vos critères de recherche et vos consentements en cours ;" }</li>
                        <li>{ "vos plans, publiés comme passés ;" }</li>
                        <li>{ "les demandes que vous avez envoyées et reçues ;" }</li>
                        <li>
                            { "vos conversations et vos messages — y compris ceux que vous aviez envoyés, qui disparaissent aussi de l'écran de vos correspondants ;" }
                        </li>
                        <li>{ "vos appareils liés et leurs jetons de notification." }</li>
                    </ul>
                    <p>
                        { format!("Le délai de {ACCOUNT_PURGE_DAYS} jours n'est pas une période de rétention commerciale : c'est la marge qui permet de traiter une suppression demandée par erreur, et d'achever les opérations comptables en cours.") }
                    </p>
                </>
            },
        },
        Article {
            id: "conserve",
            titre: "Ce qui est conservé, et pourquoi",
            contenu: html! {
                <>
                    <p>
                        { "Trois choses survivent à la suppression. Nous préférons le dire clairement plutôt que de laisser croire à un effacement total qui n'en serait pas un." }
                    </p>
                    <ul>
                        <li>
                            <strong>{ "Les signalements vous concernant" }</strong>
                            { ", le temps d'instruire le dossier. Sans cela, supprimer son compte suffirait à effacer les preuves d'un comportement qu'on vient de signaler. Tant qu'un signalement reste ouvert, la purge de votre compte est donc reportée — il demeure invisible et inutilisable dans l'intervalle, puis il est effacé dès le dossier clos. " }
                            <strong>{ "Ce report est borné à quatre-vingt-dix jours :" }</strong>
                            { " une instruction qui n'a pas eu lieu en trois mois ne justifie plus de conserver vos données, et votre compte est effacé même si le dossier est resté ouvert." }
                        </li>
                        <li>
                            <strong>{ "Les traces techniques" }</strong>
                            { " de vos actions sensibles, dans notre journal d'audit — mais " }
                            <em>{ "détachées de vous" }</em>
                            { " : l'action reste consignée, son auteur devient anonyme. C'est ce qui permet d'établir qu'un incident a eu lieu sans continuer à vous désigner." }
                        </li>
                        <li>
                            <strong>{ "Vos factures, chez Apple." }</strong>
                            { " C'est Apple qui encaisse et qui édite les justificatifs comptables ; ils suivent ses propres durées de conservation, que nous ne maîtrisons pas. Notre copie des transactions, elle, part avec votre compte." }
                        </li>
                    </ul>
                </>
            },
        },
        Article {
            id: "pause",
            titre: "Une pause plutôt qu'un départ",
            contenu: html! {
                <>
                    <p>
                        { "Si vous voulez seulement souffler, « Mettre en pause » dans les réglages retire vos plans du fil et empêche qu'on vous écrive, " }
                        <strong>{ "sans rien supprimer" }</strong>
                        { ". Vos conversations vous attendent, et vos plans reviennent tels quels à la reprise. Vous reprenez quand vous voulez." }
                    </p>
                    <p>
                        // L'espace insécable de « App Store » : le nom ne doit
                        // pas se couper en fin de ligne.
                        { "Pensez aussi à résilier votre abonnement séparément, dans les réglages de votre compte Apple : supprimer le compte Weave ne résilie pas un abonnement souscrit via l'App\u{a0}Store. Les " }
                        <a href="/cgv">{ "conditions générales de vente" }</a>
                        { " détaillent la marche à suivre." }
                    </p>
                </>
            },
        },
    ]
}

#[function_component]
pub fn SuppressionCompte() -> Html {
    html! {
        <Page
            titre="Supprimer votre compte"
            chapeau="Partir doit être aussi simple qu'arriver. Voici comment faire, ce qui est effacé, et les trois choses que nous sommes tenus de conserver."
            mise_a_jour={mise_a_jour("suppression-compte")}
            articles={articles()}
        />
    }
}
