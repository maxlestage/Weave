//! Conditions générales de vente.
//!
//! Les tarifs sont lus dans `contrat`, comme la section « Offres » et comme
//! l'API qui ouvre les droits. Recopier des prix ici les aurait laissés
//! dériver — et un prix affiché qui ne correspond plus à celui pratiqué est une
//! pratique commerciale trompeuse.
//!
//! Le point qui mérite l'attention est la rétractation : Weave vend du contenu
//! numérique fourni immédiatement, et les achats passent par l'App Store, où
//! Apple est le vendeur. Annoncer « quatorze jours pour changer d'avis » sans
//! ces deux précisions serait aussi faux que de ne rien dire du tout.

use crate::contrat;
use crate::identite::{CONTACT, EDITEUR};
use crate::langues::Langue;
use crate::pages::page::{AC, Article, Page, Tableau, mise_a_jour};
use yew::prelude::*;

/// Ces pages sont en français : les prix s'y mettent en forme en `fr-FR`, qui
/// est aussi la langue par défaut de `formatPrice`.
const LANGUE: Langue = Langue::Fr;

/// Le palier gratuit n'a pas de prix mensuel : il n'a rien à faire dans un
/// tableau de tarifs.
fn payants() -> Vec<&'static contrat::Palier> {
    contrat::PALIERS
        .iter()
        .filter(|palier| palier.prix_centimes > 0)
        .collect()
}

/// Le palier de départ, nommé dans le texte.
fn depart() -> &'static contrat::Palier {
    contrat::PALIERS
        .iter()
        .find(|palier| palier.cle == "depart")
        .expect("le catalogue n'a pas de palier « depart »")
}

fn articles() -> Vec<Article> {
    vec![
        Article {
            id: "objet",
            titre: "Objet et vendeur",
            contenu: html! {
                <>
                    <p>
                        { "Les présentes conditions régissent la vente des abonnements et des achats à l'unité proposés dans Weave, application éditée par " }
                        <AC>{ EDITEUR.raison_sociale }</AC>
                        { "." }
                    </p>
                    <p>
                        { "Ces achats s'effectuent " }
                        <strong>{ "exclusivement via l'App\u{a0}Store d'Apple" }</strong>
                        { ". Apple agit comme vendeur : c'est à elle que vous payez, c'est elle qui édite la facture, et c'est à elle que s'adressent les demandes de remboursement. Les conditions de l'App\u{a0}Store s'appliquent donc en complément des présentes." }
                    </p>
                </>
            },
        },
        Article {
            id: "offres",
            titre: "Les abonnements",
            contenu: html! {
                <>
                    <p>
                        { "Weave reste utilisable gratuitement. Le palier " }
                        <strong>{ depart().nom }</strong>
                        { " donne de quoi publier ses plans et demander à venir. Les paliers payants élargissent la finesse des critères, l'horizon de publication et les plans de groupe." }
                    </p>
                    <Tableau
                        titre="Les paliers payants, leur prix mensuel et ce qu'ils ajoutent"
                        entetes={vec!["Palier", "Par mois", "Ce qu'il ajoute"]}
                        lignes={payants().into_iter().map(|palier| vec![
                            html! { <strong>{ palier.nom }</strong> },
                            html! { { *palier.prix.choisir(LANGUE) } },
                            html! { { *palier.accroche.choisir(LANGUE) } },
                        ]).collect::<Vec<_>>()}
                    />
                    <p>
                        <strong>{ "Tous les abonnements sont mensuels." }</strong>
                        { " Il n'existe pas d'engagement à l'année : un service qu'on peut vouloir quitter du jour au lendemain ne se prépaie pas sur douze mois." }
                    </p>
                    <p>
                        <strong>{ "Aucun palier n'achète de visibilité." }</strong>
                        { " Payer n'a jamais pour effet de faire remonter un plan devant celui de quelqu'un d'autre, et le plafond de demandes quotidiennes demeure à tous les paliers. C'est une règle de conception, pas une limite temporaire." }
                    </p>
                </>
            },
        },
        Article {
            id: "unites",
            titre: "Les achats à l'unité",
            contenu: html! {
                <>
                    <p>
                        { "Chaque avantage est également achetable seul, sans abonnement — on ne s'abonne pas à tout à vingt ans." }
                    </p>
                    <Tableau
                        titre="Les achats à l'unité, leur prix et ce qu'ils donnent"
                        entetes={vec!["Achat", "Prix", "Ce qu'il donne"]}
                        lignes={contrat::PRODUITS_UNITE.iter().map(|produit| vec![
                            html! { <strong>{ produit.nom }</strong> },
                            html! { { *produit.prix.choisir(LANGUE) } },
                            html! { { *produit.description.choisir(LANGUE) } },
                        ]).collect::<Vec<_>>()}
                    />
                    <p>
                        { "Ces achats sont consommables : ils s'appliquent une fois et ne se renouvellent pas d'eux-mêmes." }
                    </p>
                </>
            },
        },
        Article {
            id: "prix",
            titre: "Prix et paiement",
            contenu: html! {
                <>
                    <p>
                        { "Les prix sont indiqués en euros, toutes taxes comprises. Le montant effectivement débité est celui affiché par l'App\u{a0}Store au moment de l'achat : Apple applique ses propres grilles par pays, et le prix peut donc différer selon votre région." }
                    </p>
                    <p>
                        { "Le paiement s'effectue avec le moyen enregistré sur votre compte Apple. " }
                        <strong>
                            { "Aucune coordonnée bancaire ne nous est transmise ni ne transite par nos serveurs." }
                        </strong>
                    </p>
                    <p>
                        { "Nous pouvons faire évoluer nos tarifs. Un changement de prix sur un abonnement en cours vous est notifié par Apple et ne prend effet qu'après votre acceptation ; à défaut, l'abonnement s'arrête au terme de la période en cours." }
                    </p>
                </>
            },
        },
        Article {
            id: "reconduction",
            titre: "Reconduction et résiliation",
            contenu: html! {
                <>
                    <p>
                        { "Les abonnements sont " }
                        <strong>{ "reconduits automatiquement" }</strong>
                        { " à échéance, sauf résiliation au moins " }
                        <strong>{ "24 heures avant la fin de la période en cours" }</strong>
                        { ". Le renouvellement est débité dans les 24 heures précédant le terme." }
                    </p>
                    <p>
                        { "La résiliation se fait dans les réglages de votre compte Apple — « Abonnements » —, jamais depuis Weave : nous n'avons pas la main sur les abonnements de l'App\u{a0}Store." }
                    </p>
                    <p>
                        <strong>{ "Supprimer votre compte Weave ne résilie pas votre abonnement." }</strong>
                        { " Ce sont deux démarches distinctes, et l'oublier conduit à continuer de payer un service qu'on n'utilise plus. Pensez à faire les deux." }
                    </p>
                    <p>
                        { "Après résiliation, vous conservez vos droits jusqu'au terme de la période déjà payée." }
                    </p>
                </>
            },
        },
        Article {
            id: "retractation",
            titre: "Droit de rétractation",
            contenu: html! {
                <>
                    <p>
                        { "Un consommateur dispose en principe de " }
                        <strong>{ "quatorze jours" }</strong>
                        { " pour se rétracter d'un achat à distance." }
                    </p>
                    <p>
                        { "Ce droit connaît une exception pour le contenu numérique fourni immédiatement : en demandant l'accès instantané à vos droits au moment de l'achat, vous demandez l'exécution immédiate du contrat et " }
                        <strong>{ "renoncez à votre droit de rétractation" }</strong>
                        { " une fois la fourniture commencée. L'App\u{a0}Store recueille cet accord lors de l'achat." }
                    </p>
                    <p>
                        { "Cela ne vous laisse pas sans recours. Apple accorde des remboursements au cas par cas : la demande se fait sur " }
                        <a href="https://reportaproblem.apple.com" rel="noreferrer">
                            { "reportaproblem.apple.com" }
                        </a>
                        { ", avec le compte Apple ayant servi à l'achat. Si un achat n'a pas ouvert les droits annoncés, écrivez-nous à " }
                        <a href={format!("mailto:{}", CONTACT.support)}>
                            <AC>{ CONTACT.support }</AC>
                        </a>
                        { " : nous appuyons votre démarche auprès d'Apple." }
                    </p>
                </>
            },
        },
        Article {
            id: "conformite",
            titre: "Garantie de conformité",
            contenu: html! {
                <>
                    <p>
                        { "Indépendamment de tout geste commercial, vous bénéficiez de la " }
                        <strong>{ "garantie légale de conformité" }</strong>
                        { " du contenu numérique et de la garantie contre les vices cachés, prévues par le code de la consommation et le code civil." }
                    </p>
                    <p>
                        { "Si un service payé ne fonctionne pas comme annoncé, vous pouvez en exiger la mise en conformité, et à défaut une réduction du prix ou la résolution du contrat. Ces garanties sont gratuites et ne sont subordonnées à aucune condition que nous poserions." }
                    </p>
                </>
            },
        },
        Article {
            id: "litiges",
            titre: "Réclamations et litiges",
            contenu: html! {
                <>
                    <p>
                        { "Écrivez-nous d'abord à " }
                        <a href={format!("mailto:{}", CONTACT.support)}>
                            <AC>{ CONTACT.support }</AC>
                        </a>
                        { ". Nous accusons réception et traitons votre réclamation dans les meilleurs délais." }
                    </p>
                    <p>
                        { "À défaut de solution, un consommateur peut recourir gratuitement à un médiateur de la consommation, ou saisir la plateforme européenne de règlement en ligne des litiges. Le droit français s'applique, sans priver le consommateur des dispositions impératives de son pays de résidence." }
                    </p>
                </>
            },
        },
    ]
}

#[function_component]
pub fn Cgv() -> Html {
    html! {
        <Page
            titre="Conditions générales de vente"
            chapeau="Ce qui se vend, à quel prix, comment se résilier et comment se faire rembourser. Les achats passent par l'App Store : cela change qui fait quoi, et nous le disons plutôt que de l'omettre."
            mise_a_jour={mise_a_jour("cgv")}
            articles={articles()}
        />
    }
}
