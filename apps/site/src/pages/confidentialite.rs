//! Politique de confidentialité.
//!
//! Chaque ligne du tableau des données correspond à une colonne réelle du
//! schéma de l'API. Ce n'est pas une précaution de style : une politique qui
//! décrit autre chose que ce que le service fait est une déclaration inexacte,
//! et c'est elle qu'on oppose à l'éditeur en cas de contrôle.
//!
//! Les durées viennent de `contrat`. Si l'une d'elles change dans le produit,
//! ce texte change avec — il n'y a pas de second endroit à mettre à jour, et
//! donc pas de dérive silencieuse possible.

use crate::contrat::{ACCOUNT_PURGE_DAYS, MESSAGE_RETENTION_DAYS, MIN_AGE};
use crate::identite::{CONTACT, EDITEUR};
use crate::pages::page::{AC, Article, Page, Tableau, mise_a_jour};
use yew::prelude::*;

/// « Durée du compte, puis 30 jours » — la formule revient sept fois dans le
/// tableau. Une fonction plutôt que sept copies : le jour où le délai change,
/// il change partout.
fn duree_du_compte() -> String {
    format!("Durée du compte, puis {ACCOUNT_PURGE_DAYS} jours")
}

/// Une cellule de tableau faite de texte.
fn c(texte: impl Into<String>) -> Html {
    let texte: String = texte.into();
    html! { { texte } }
}

fn lignes_des_donnees() -> Vec<Vec<Html>> {
    vec![
        vec![
            c("Adresse e-mail, empreinte de l'adresse"),
            c("Créer le compte, envoyer les codes de connexion"),
            c("Contrat"),
            c(duree_du_compte()),
        ],
        vec![
            c("Code de connexion à usage unique"),
            c("Vérifier que l'adresse est bien la vôtre"),
            c("Contrat"),
            c("Quelques minutes, puis effacé"),
        ],
        vec![
            c("Pseudonyme, nom affiché, date de naissance"),
            c(format!(
                "Identité publique dans l'application et contrôle de l'âge minimum ({MIN_AGE} ans)"
            )),
            c("Contrat, obligation légale"),
            c(duree_du_compte()),
        ],
        vec![
            c("Ville, position arrondie, genre, présentation, photo"),
            c("Composer le fil autour de vous et vous présenter aux autres"),
            c("Contrat ; consentement distinct pour les données sensibles"),
            c(duree_du_compte()),
        ],
        vec![
            c("Critères de recherche (âge, distance, catégories, personnes recherchées)"),
            c("Filtrer le fil selon ce que vous cherchez"),
            c("Consentement — ces critères peuvent révéler l'orientation sexuelle"),
            c(duree_du_compte()),
        ],
        vec![
            c("Plans publiés : intitulé, note, catégorie, date, ville, position arrondie"),
            c("Le service lui-même"),
            c("Contrat"),
            c(duree_du_compte()),
        ],
        vec![
            c("Demandes envoyées et reçues, avec leur message"),
            c("Vous permettre de demander à venir, et de répondre"),
            c("Contrat"),
            c(duree_du_compte()),
        ],
        vec![
            c("Messages échangés"),
            c("La conversation qui suit une demande acceptée"),
            c("Contrat"),
            c(format!(
                "{MESSAGE_RETENTION_DAYS} jours après la clôture de la conversation"
            )),
        ],
        vec![
            c(
                "Appareil : modèle, version du système, version de l'application, jeton de notification",
            ),
            c("Envoyer les notifications et les activités en direct que vous avez autorisées"),
            c("Consentement (autorisation système)"),
            c("Tant que l'appareil reste lié au compte"),
        ],
        vec![
            c("Activités en direct : jeton de mise à jour, dernier état affiché"),
            c("Tenir à jour l'écran verrouillé et le cadran de la montre pendant un plan"),
            c("Consentement (autorisation système)"),
            c("Effacées dès la fin de l'activité"),
        ],
        vec![
            c("Blocages et signalements"),
            c("Votre sécurité et celle des autres, traitement des signalements"),
            c("Intérêt légitime, obligation légale"),
            c(
                "Effacés avec votre compte ; mais tant qu'un signalement vous vise et reste ouvert, la purge de votre compte est reportée — au plus quatre-vingt-dix jours, après quoi il est effacé même si le dossier est resté ouvert",
            ),
        ],
        vec![
            c("Abonnements et achats : identifiant de transaction, référence du produit, montant"),
            c("Ouvrir les droits que vous avez payés et traiter les remboursements"),
            c("Contrat"),
            c(format!(
                "Durée du compte, puis {ACCOUNT_PURGE_DAYS} jours. Les justificatifs comptables sont établis et conservés par Apple, qui encaisse"
            )),
        ],
        vec![
            c("Traces techniques : identifiant de compte, action, adresse IP"),
            c("Détecter les abus, prouver ce qui s'est passé en cas de litige"),
            c("Intérêt légitime"),
            c("12 mois ; la suppression du compte détache la trace de vous, elle ne l'efface pas"),
        ],
        vec![
            c("Consentements : objet, version du texte, date d'octroi et de retrait"),
            c("Prouver que le traitement était licite au moment où il a eu lieu"),
            c("Obligation légale"),
            c("5 ans après le retrait"),
        ],
    ]
}

fn lignes_des_droits() -> Vec<Vec<Html>> {
    vec![
        vec![
            c("Accès et portabilité"),
            c("Obtenir une copie de vos données dans un format lisible par machine"),
            c("Réglages › Mes données, ou par courriel"),
        ],
        vec![
            c("Rectification"),
            c("Corriger votre profil, vos critères, votre compte"),
            c("Directement dans l'application"),
        ],
        vec![
            c("Effacement"),
            c("Supprimer votre compte et tout ce qui s'y rattache"),
            html! { <a href="/suppression-compte">{ "Voir la marche à suivre" }</a> },
        ],
        vec![
            c("Opposition et limitation"),
            c(
                "Mettre le compte en pause : vos plans sortent du fil, personne ne peut vous écrire, rien n'est supprimé — tout revient à la reprise",
            ),
            c("Réglages › Mettre en pause"),
        ],
        vec![
            c("Retrait du consentement"),
            c("Retirer le consentement aux données sensibles, sans perdre le compte"),
            c("Réglages › Confidentialité"),
        ],
    ]
}

fn articles() -> Vec<Article> {
    vec![
        Article {
            id: "responsable",
            titre: "Qui traite vos données",
            contenu: html! {
                <>
                    <p>
                        { "Le responsable du traitement est " }
                        <AC>{ EDITEUR.raison_sociale }</AC>
                        { ", dont les coordonnées complètes figurent dans les " }
                        <a href="/mentions-legales">{ "mentions légales" }</a>
                        { "." }
                    </p>
                    <p>
                        { "Pour toute question relative à vos données, ou pour exercer vos droits : " }
                        <a href={format!("mailto:{}", CONTACT.confidentialite)}>
                            <AC>{ CONTACT.confidentialite }</AC>
                        </a>
                        { "." }
                    </p>
                    <p>
                        { "Un délégué à la protection des données doit être désigné avant l'ouverture du service au public : Weave traite des données sensibles à grande échelle, ce qui rend la désignation obligatoire. Ses coordonnées seront publiées ici." }
                    </p>
                </>
            },
        },
        Article {
            id: "principe",
            titre: "Le principe qui gouverne le reste",
            contenu: html! {
                <>
                    <p>
                        { "Weave publie des " }
                        <strong>{ "plans" }</strong>
                        { " : un lieu, une heure, une intention. C'est une donnée de déplacement " }
                        <em>{ "à venir" }</em>
                        { ", plus sensible qu'une position passée, parce qu'elle dit où vous serez." }
                    </p>
                    <p>
                        { "Trois règles en découlent, et elles sont appliquées par le code, pas par la promesse :" }
                    </p>
                    <ul>
                        <li>
                            <strong>{ "Votre position est arrondie avant d'être enregistrée." }</strong>
                            { " Nous ne stockons jamais une précision meilleure que l'ordre du kilomètre. Une adresse exacte ne quitte jamais votre téléphone." }
                        </li>
                        <li>
                            <strong>{ "Un plan n'affiche jamais d'adresse" }</strong>
                            { " dans le fil : une ville et une distance arrondie. Le lieu précis se dit dans la conversation, aux personnes que vous avez acceptées." }
                        </li>
                        <li>
                            <strong>{ "Ce que vous consultez n'est pas archivé." }</strong>
                            { " Le fil est recalculé à la demande et vit quelques minutes en cache. Nous ne constituons pas d'historique de ce que vous avez regardé." }
                        </li>
                    </ul>
                </>
            },
        },
        Article {
            id: "donnees",
            titre: "Ce que nous collectons, et pourquoi",
            contenu: html! {
                <>
                    <p>
                        { "Le tableau suit le service dans l'ordre où vous le rencontrez. La base légale est indiquée pour chaque finalité : le contrat quand la donnée est nécessaire pour vous fournir le service, le consentement quand elle ne l'est pas, l'obligation légale quand la loi l'impose, l'intérêt légitime pour la sécurité du service." }
                    </p>
                    <Tableau
                        titre="Les données collectées, leur finalité, leur base légale et leur durée de conservation"
                        entetes={vec!["Données", "Pourquoi", "Base légale", "Conservation"]}
                        lignes={lignes_des_donnees()}
                    />
                    <p>
                        <strong>{ "Ce que nous ne collectons pas :" }</strong>
                        { " vos contacts, votre carnet d'adresses, vos identifiants publicitaires, votre position en arrière-plan, ni aucune donnée acquise auprès d'un tiers." }
                    </p>
                </>
            },
        },
        Article {
            id: "sensibles",
            titre: "Les données sensibles, et le consentement à part",
            contenu: html! {
                <>
                    <p>
                        { "Les personnes que vous cherchez, rapprochées de votre genre, peuvent révéler votre orientation sexuelle. Le règlement européen range cette information parmi les catégories particulières de l'article\u{a0}9 : elle ne peut être traitée que sur un " }
                        <strong>{ "consentement explicite et distinct" }</strong>
                        { "." }
                    </p>
                    <p>
                        { "Ce consentement vous est donc demandé " }
                        <strong>{ "séparément" }</strong>
                        { ", jamais par une case unique valant acceptation de tout le reste. Vous pouvez le retirer à tout moment depuis l'application ; le service continue de fonctionner, avec un fil non filtré sur ce critère." }
                    </p>
                    <p>
                        { "Le retrait est enregistré avec sa date. Nous conservons la trace de la période pendant laquelle le consentement était actif — c'est ce qui permet de démontrer que le traitement était licite lorsqu'il a eu lieu, et rien de plus." }
                    </p>
                </>
            },
        },
        Article {
            id: "localisation",
            titre: "Votre position",
            contenu: html! {
                <>
                    <p>
                        { "L'application " }
                        <strong>{ "ne demande jamais l'accès à votre position" }</strong>
                        { ". Elle ne possède aucune autorisation de localisation, et ne peut donc vous suivre ni quand elle est ouverte ni quand elle est fermée." }
                    </p>
                    <p>
                        { "Vous " }
                        <strong>{ "saisissez une ville" }</strong>
                        { ". Votre appareil la convertit lui-même en coordonnées, par le géocodeur du système : rien ne sort de votre téléphone vers nous à cette étape. Ces coordonnées sont " }
                        <strong>{ "arrondies sur votre appareil avant l'envoi" }</strong>
                        { ", puis stockées arrondies. Nos serveurs ne disposent à aucun moment d'une position plus précise : ce n'est pas une politique de rétention, c'est une donnée que nous n'avons pas." }
                    </p>
                    <p>
                        { "Les autres utilisateurs ne voient jamais que la ville et une distance arrondie." }
                    </p>
                </>
            },
        },
        Article {
            id: "cookies",
            titre: "Cookies et mesure d'audience",
            contenu: html! {
                <>
                    <p>
                        <strong>{ "Ce site ne dépose aucun cookie" }</strong>
                        { " et n'embarque aucun traceur, aucune régie publicitaire, aucun outil de mesure d'audience tiers. Il n'affiche donc pas de bandeau de consentement : il n'y a rien à consentir." }
                    </p>
                    <p>
                        { "Il ne charge pas non plus de police, de script ni d'image depuis un domaine tiers. Votre visite n'est connue que de notre hébergeur, à travers les journaux techniques nécessaires au fonctionnement du serveur." }
                    </p>
                    <p>
                        { "Si une mesure d'audience était mise en place un jour, elle serait annoncée ici et soumise à votre consentement préalable." }
                    </p>
                </>
            },
        },
        Article {
            id: "destinataires",
            titre: "Qui reçoit ces données",
            contenu: html! {
                <>
                    <p>
                        { "Les autres utilisateurs voient ce que vous publiez volontairement : votre profil public, vos plans ouverts, et le message des demandes que vous envoyez. Rien d'autre." }
                    </p>
                    <p>{ "Au-delà, interviennent uniquement :" }</p>
                    <ul>
                        <li>
                            <strong>{ "Notre hébergeur" }</strong>
                            { ", " }
                            <AC>{ EDITEUR.hebergeur.nom }</AC>
                            { ", qui exécute le service et stocke la base de données." }
                        </li>
                        <li>
                            <strong>{ "Apple" }</strong>
                            { ", pour l'acheminement des notifications, et pour les paiements." }
                        </li>
                        <li>
                            <strong>{ "L'autorité judiciaire" }</strong>
                            { ", sur réquisition régulière, dans les limites de ce que la loi impose." }
                        </li>
                    </ul>
                    <p>
                        <strong>{ "Nous ne vendons aucune donnée, à personne." }</strong>
                        { " Weave n'affiche pas de publicité et ne travaille avec aucun courtier en données. Le service est financé par les abonnements et les achats à l'unité — c'est l'unique modèle économique, et il est intentionnel." }
                    </p>
                </>
            },
        },
        Article {
            id: "paiements",
            titre: "Paiements",
            contenu: html! {
                <>
                    <p>
                        { "Les abonnements et les achats passent par l'App\u{a0}Store. " }
                        <strong>{ "Aucun moyen de paiement ne transite par nos serveurs" }</strong>
                        { " : nous ne voyons ni votre numéro de carte, ni votre adresse de facturation." }
                    </p>
                    <p>
                        { "Nous conservons l'identifiant de transaction communiqué par Apple, la référence du produit acheté et son montant — ce qui est nécessaire pour ouvrir vos droits, traiter un remboursement et tenir la comptabilité." }
                    </p>
                </>
            },
        },
        Article {
            id: "transferts",
            titre: "Transferts hors de l'Union européenne",
            contenu: html! {
                <>
                    <p>
                        { "La base de données et les serveurs de l'application sont hébergés " }
                        <AC>{ EDITEUR.hebergeur.region }</AC>
                        { "." }
                    </p>
                    <p>
                        { "L'acheminement des notifications par Apple peut impliquer un transfert vers les États-Unis, encadré par les clauses contractuelles types de la Commission européenne. Rappelons que " }
                        <strong>
                            { "aucun contenu de conversation ne transite par les notifications ni ne s'affiche sur l'écran verrouillé" }
                        </strong>
                        { " : un intitulé de plan et deux compteurs, jamais un message, jamais un prénom." }
                    </p>
                </>
            },
        },
        Article {
            id: "securite",
            titre: "Comment ces données sont protégées",
            contenu: html! {
                <>
                    <ul>
                        <li>
                            { "Les codes de connexion sont " }
                            <strong>{ "hachés" }</strong>
                            { " par une fonction conçue pour résister aux attaques par force brute\u{a0}; ils ne sont jamais stockés en clair." }
                        </li>
                        <li>
                            { "Les jetons de session sont hachés et " }
                            <strong>{ "tournent à chaque usage" }</strong>
                            { " : un jeton intercepté cesse de valoir dès sa première réutilisation." }
                        </li>
                        <li>
                            { "Les photos ne sont accessibles que par une " }
                            <strong>{ "adresse signée à durée limitée" }</strong>
                            { ", impossible à deviner et périmée après quelques minutes." }
                        </li>
                        <li>
                            { "Les échanges sont chiffrés en transit. Les journaux techniques sont volontairement pauvres en données personnelles : jamais d'adresse e-mail en clair." }
                        </li>
                    </ul>
                    <p>
                        { "En cas de violation de données susceptible d'engendrer un risque pour vos droits, nous notifierons la CNIL dans les 72 heures, et vous informerons directement lorsque le risque est élevé." }
                    </p>
                </>
            },
        },
        Article {
            id: "droits",
            titre: "Vos droits, et comment les exercer",
            contenu: html! {
                <>
                    <p>
                        { "Le règlement vous reconnaît des droits que vous pouvez exercer à tout moment, gratuitement." }
                    </p>
                    <Tableau
                        titre="Vos droits, ce qu'ils permettent et où les exercer"
                        entetes={vec!["Droit", "Ce que vous pouvez faire", "Où"]}
                        lignes={lignes_des_droits()}
                    />
                    <p>
                        { "Nous répondons dans un délai d'un mois, prolongeable de deux mois pour les demandes complexes — vous en seriez informé. Si notre réponse ne vous satisfait pas, vous pouvez saisir la " }
                        <strong>{ "CNIL" }</strong>
                        { " (3 place de Fontenoy, 75007 Paris — " }
                        <a href="https://www.cnil.fr" rel="noreferrer">{ "cnil.fr" }</a>
                        { ")." }
                    </p>
                </>
            },
        },
        Article {
            id: "mineurs",
            titre: "Âge minimum",
            contenu: html! {
                <>
                    <p>
                        { "Weave est une application de rencontre : elle est " }
                        <strong>{ format!("réservée aux personnes de {MIN_AGE} ans et plus") }</strong>
                        { ". La date de naissance est demandée à l'inscription et l'accès est refusé en dessous de cet âge." }
                    </p>
                    <p>
                        { "Un signalement indiquant qu'un compte appartient à une personne mineure est traité en priorité, entraîne la suspension immédiate du compte, et le cas échéant un signalement aux autorités compétentes." }
                    </p>
                </>
            },
        },
        Article {
            id: "modifications",
            titre: "Modifications de cette politique",
            contenu: html! {
                <>
                    <p>
                        { "Chaque version de ce texte porte une date. En cas de changement substantiel, vous en êtes informé dans l'application avant son entrée en vigueur, et un nouveau consentement vous est demandé lorsque le changement l'exige." }
                    </p>
                    <p>
                        { "Les versions successives sont conservées : nous devons pouvoir établir quel texte vous aviez accepté, et à quelle date." }
                    </p>
                </>
            },
        },
    ]
}

#[function_component]
pub fn Confidentialite() -> Html {
    html! {
        <Page
            titre="Politique de confidentialité"
            chapeau="Une application où l'on donne son heure et son lieu manipule ce qu'il y a de plus sensible. Voici exactement ce que nous collectons, pourquoi, combien de temps, et ce que nous nous interdisons."
            mise_a_jour={mise_a_jour("confidentialite")}
            articles={articles()}
        />
    }
}
