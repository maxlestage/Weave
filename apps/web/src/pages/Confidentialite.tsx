/*
 * Politique de confidentialité.
 *
 * Chaque ligne du tableau des données correspond à une colonne réelle du
 * schéma de l'API. Ce n'est pas une précaution de style : une politique qui
 * décrit autre chose que ce que le service fait est une déclaration inexacte,
 * et c'est elle qu'on oppose à l'éditeur en cas de contrôle.
 *
 * Les durées viennent de `@weave/contracts`. Si l'une d'elles change dans le
 * produit, ce texte change avec — il n'y a pas de second endroit à mettre à
 * jour, et donc pas de dérive silencieuse possible.
 */

import { ACCOUNT_PURGE_DAYS, MESSAGE_RETENTION_DAYS, MIN_AGE } from "@weave/contracts";
import { AC, Page, Tableau, type Article } from "./Page.tsx";
import { CONTACT, EDITEUR, MISE_A_JOUR } from "./identite.ts";

const ARTICLES: readonly Article[] = [
  {
    id: "responsable",
    titre: "Qui traite vos données",
    contenu: (
      <>
        <p>
          Le responsable du traitement est <AC>{EDITEUR.raisonSociale}</AC>, dont les coordonnées
          complètes figurent dans les <a href="/mentions-legales">mentions légales</a>.
        </p>
        <p>
          Pour toute question relative à vos données, ou pour exercer vos droits :{" "}
          <a href={`mailto:${CONTACT.confidentialite}`}>
            <AC>{CONTACT.confidentialite}</AC>
          </a>
          .
        </p>
        <p>
          Un délégué à la protection des données doit être désigné avant l'ouverture du service au
          public : Weave traite des données sensibles à grande échelle, ce qui rend la désignation
          obligatoire. Ses coordonnées seront publiées ici.
        </p>
      </>
    ),
  },
  {
    id: "principe",
    titre: "Le principe qui gouverne le reste",
    contenu: (
      <>
        <p>
          Weave publie des <strong>plans</strong> : un lieu, une heure, une intention. C'est une
          donnée de déplacement <em>à venir</em>, plus sensible qu'une position passée, parce
          qu'elle dit où vous serez.
        </p>
        <p>
          Trois règles en découlent, et elles sont appliquées par le code, pas par la promesse :
        </p>
        <ul>
          <li>
            <strong>Votre position est arrondie avant d'être enregistrée.</strong> Nous ne stockons
            jamais une précision meilleure que l'ordre du kilomètre. Une adresse exacte ne quitte
            jamais votre téléphone.
          </li>
          <li>
            <strong>Un plan n'affiche jamais d'adresse</strong> dans le fil : une ville et une
            distance arrondie. Le lieu précis se dit dans la conversation, aux personnes que vous
            avez acceptées.
          </li>
          <li>
            <strong>Ce que vous consultez n'est pas archivé.</strong> Le fil est recalculé à la
            demande et vit quelques minutes en cache. Nous ne constituons pas d'historique de ce que
            vous avez regardé.
          </li>
        </ul>
      </>
    ),
  },
  {
    id: "donnees",
    titre: "Ce que nous collectons, et pourquoi",
    contenu: (
      <>
        <p>
          Le tableau suit le service dans l'ordre où vous le rencontrez. La base légale est indiquée
          pour chaque finalité : le contrat quand la donnée est nécessaire pour vous fournir le
          service, le consentement quand elle ne l'est pas, l'obligation légale quand la loi
          l'impose, l'intérêt légitime pour la sécurité du service.
        </p>
        <Tableau
          entetes={["Données", "Pourquoi", "Base légale", "Conservation"]}
          lignes={[
            [
              "Adresse e-mail, empreinte de l'adresse",
              "Créer le compte, envoyer les codes de connexion",
              "Contrat",
              `Durée du compte, puis ${ACCOUNT_PURGE_DAYS} jours`,
            ],
            [
              "Code de connexion à usage unique",
              "Vérifier que l'adresse est bien la vôtre",
              "Contrat",
              "Quelques minutes, puis effacé",
            ],
            [
              "Pseudonyme, nom affiché, date de naissance",
              `Identité publique dans l'application et contrôle de l'âge minimum (${MIN_AGE} ans)`,
              "Contrat, obligation légale",
              `Durée du compte, puis ${ACCOUNT_PURGE_DAYS} jours`,
            ],
            [
              "Ville, position arrondie, genre, présentation, photo",
              "Composer le fil autour de vous et vous présenter aux autres",
              "Contrat ; consentement distinct pour les données sensibles",
              `Durée du compte, puis ${ACCOUNT_PURGE_DAYS} jours`,
            ],
            [
              "Critères de recherche (âge, distance, catégories, personnes recherchées)",
              "Filtrer le fil selon ce que vous cherchez",
              "Consentement — ces critères peuvent révéler l'orientation sexuelle",
              `Durée du compte, puis ${ACCOUNT_PURGE_DAYS} jours`,
            ],
            [
              "Plans publiés : intitulé, note, catégorie, date, ville, position arrondie",
              "Le service lui-même",
              "Contrat",
              `Durée du compte, puis ${ACCOUNT_PURGE_DAYS} jours`,
            ],
            [
              "Demandes envoyées et reçues, avec leur message",
              "Vous permettre de demander à venir, et de répondre",
              "Contrat",
              `Durée du compte, puis ${ACCOUNT_PURGE_DAYS} jours`,
            ],
            [
              "Messages échangés",
              "La conversation qui suit une demande acceptée",
              "Contrat",
              `${MESSAGE_RETENTION_DAYS} jours après la clôture de la conversation`,
            ],
            [
              "Appareil : modèle, version du système, version de l'application, jeton de notification",
              "Envoyer les notifications et les activités en direct que vous avez autorisées",
              "Consentement (autorisation système)",
              "Tant que l'appareil reste lié au compte",
            ],
            [
              "Activités en direct : jeton de mise à jour, dernier état affiché",
              "Tenir à jour l'écran verrouillé et le cadran de la montre pendant un plan",
              "Consentement (autorisation système)",
              "Effacées dès la fin de l'activité",
            ],
            [
              "Blocages et signalements",
              "Votre sécurité et celle des autres, traitement des signalements",
              "Intérêt légitime, obligation légale",
              "Effacés avec votre compte ; mais tant qu'un signalement vous vise et reste ouvert, la purge de votre compte est reportée",
            ],
            [
              "Abonnements et achats : identifiant de transaction, référence du produit, montant",
              "Ouvrir les droits que vous avez payés et traiter les remboursements",
              "Contrat",
              `Durée du compte, puis ${ACCOUNT_PURGE_DAYS} jours. Les justificatifs comptables sont établis et conservés par Apple, qui encaisse`,
            ],
            [
              "Traces techniques : identifiant de compte, action, adresse IP",
              "Détecter les abus, prouver ce qui s'est passé en cas de litige",
              "Intérêt légitime",
              "12 mois ; la suppression du compte détache la trace de vous, elle ne l'efface pas",
            ],
            [
              "Consentements : objet, version du texte, date d'octroi et de retrait",
              "Prouver que le traitement était licite au moment où il a eu lieu",
              "Obligation légale",
              "5 ans après le retrait",
            ],
          ]}
        />
        <p>
          <strong>Ce que nous ne collectons pas :</strong> vos contacts, votre carnet d'adresses,
          vos identifiants publicitaires, votre position en arrière-plan, ni aucune donnée acquise
          auprès d'un tiers.
        </p>
      </>
    ),
  },
  {
    id: "sensibles",
    titre: "Les données sensibles, et le consentement à part",
    contenu: (
      <>
        <p>
          Les personnes que vous cherchez, rapprochées de votre genre, peuvent révéler votre
          orientation sexuelle. Le règlement européen range cette information parmi les catégories
          particulières de l'article&nbsp;9 : elle ne peut être traitée que sur un{" "}
          <strong>consentement explicite et distinct</strong>.
        </p>
        <p>
          Ce consentement vous est donc demandé <strong>séparément</strong>, jamais par une case
          unique valant acceptation de tout le reste. Vous pouvez le retirer à tout moment depuis
          l'application ; le service continue de fonctionner, avec un fil non filtré sur ce critère.
        </p>
        <p>
          Le retrait est enregistré avec sa date. Nous conservons la trace de la période pendant
          laquelle le consentement était actif — c'est ce qui permet de démontrer que le traitement
          était licite lorsqu'il a eu lieu, et rien de plus.
        </p>
      </>
    ),
  },
  {
    id: "localisation",
    titre: "Votre position",
    contenu: (
      <>
        <p>
          L'application demande votre position <strong>pendant son utilisation</strong> uniquement.
          Elle ne la demande jamais en arrière-plan, et ne peut donc pas vous suivre quand elle est
          fermée.
        </p>
        <p>
          La position est <strong>arrondie sur votre appareil avant l'envoi</strong>, puis stockée
          arrondie. Nos serveurs ne disposent à aucun moment de vos coordonnées exactes : ce n'est
          pas une politique de rétention, c'est une donnée que nous n'avons pas.
        </p>
        <p>
          Les autres utilisateurs ne voient jamais que la ville et une distance arrondie. Vous
          pouvez refuser la géolocalisation : il faut alors saisir une ville à la main.
        </p>
      </>
    ),
  },
  {
    id: "cookies",
    titre: "Cookies et mesure d'audience",
    contenu: (
      <>
        <p>
          <strong>Ce site ne dépose aucun cookie</strong> et n'embarque aucun traceur, aucune régie
          publicitaire, aucun outil de mesure d'audience tiers. Il n'affiche donc pas de bandeau de
          consentement : il n'y a rien à consentir.
        </p>
        <p>
          Il ne charge pas non plus de police, de script ni d'image depuis un domaine tiers. Votre
          visite n'est connue que de notre hébergeur, à travers les journaux techniques nécessaires
          au fonctionnement du serveur.
        </p>
        <p>
          Si une mesure d'audience était mise en place un jour, elle serait annoncée ici et soumise
          à votre consentement préalable.
        </p>
      </>
    ),
  },
  {
    id: "destinataires",
    titre: "Qui reçoit ces données",
    contenu: (
      <>
        <p>
          Les autres utilisateurs voient ce que vous publiez volontairement : votre profil public,
          vos plans ouverts, et le message des demandes que vous envoyez. Rien d'autre.
        </p>
        <p>Au-delà, interviennent uniquement :</p>
        <ul>
          <li>
            <strong>Notre hébergeur</strong>, <AC>{EDITEUR.hebergeur.nom}</AC>, qui exécute le
            service et stocke la base de données.
          </li>
          <li>
            <strong>Apple</strong>, pour l'acheminement des notifications, et pour les paiements.
          </li>
          <li>
            <strong>L'autorité judiciaire</strong>, sur réquisition régulière, dans les limites de
            ce que la loi impose.
          </li>
        </ul>
        <p>
          <strong>Nous ne vendons aucune donnée, à personne.</strong> Weave n'affiche pas de
          publicité et ne travaille avec aucun courtier en données. Le service est financé par les
          abonnements et les achats à l'unité — c'est l'unique modèle économique, et il est
          intentionnel.
        </p>
      </>
    ),
  },
  {
    id: "paiements",
    titre: "Paiements",
    contenu: (
      <>
        <p>
          Les abonnements et les achats passent par l'App&nbsp;Store.{" "}
          <strong>Aucun moyen de paiement ne transite par nos serveurs</strong> : nous ne voyons ni
          votre numéro de carte, ni votre adresse de facturation.
        </p>
        <p>
          Nous conservons l'identifiant de transaction communiqué par Apple, la référence du produit
          acheté et son montant — ce qui est nécessaire pour ouvrir vos droits, traiter un
          remboursement et tenir la comptabilité.
        </p>
      </>
    ),
  },
  {
    id: "transferts",
    titre: "Transferts hors de l'Union européenne",
    contenu: (
      <>
        <p>
          La base de données et les serveurs de l'application sont hébergés{" "}
          <AC>{EDITEUR.hebergeur.region}</AC>.
        </p>
        <p>
          L'acheminement des notifications par Apple peut impliquer un transfert vers les
          États-Unis, encadré par les clauses contractuelles types de la Commission européenne.
          Rappelons que{" "}
          <strong>
            aucun contenu de conversation ne transite par les notifications ni ne s'affiche sur
            l'écran verrouillé
          </strong>{" "}
          : un intitulé de plan et deux compteurs, jamais un message, jamais un prénom.
        </p>
      </>
    ),
  },
  {
    id: "securite",
    titre: "Comment ces données sont protégées",
    contenu: (
      <>
        <ul>
          <li>
            Les codes de connexion sont <strong>hachés</strong> par une fonction conçue pour
            résister aux attaques par force brute&nbsp;; ils ne sont jamais stockés en clair.
          </li>
          <li>
            Les jetons de session sont hachés et <strong>tournent à chaque usage</strong> : un jeton
            intercepté cesse de valoir dès sa première réutilisation.
          </li>
          <li>
            Les photos ne sont accessibles que par une{" "}
            <strong>adresse signée à durée limitée</strong>, impossible à deviner et périmée après
            quelques minutes.
          </li>
          <li>
            Les échanges sont chiffrés en transit. Les journaux techniques sont volontairement
            pauvres en données personnelles : jamais d'adresse e-mail en clair.
          </li>
        </ul>
        <p>
          En cas de violation de données susceptible d'engendrer un risque pour vos droits, nous
          notifierons la CNIL dans les 72 heures, et vous informerons directement lorsque le risque
          est élevé.
        </p>
      </>
    ),
  },
  {
    id: "droits",
    titre: "Vos droits, et comment les exercer",
    contenu: (
      <>
        <p>
          Le règlement vous reconnaît des droits que vous pouvez exercer à tout moment,
          gratuitement.
        </p>
        <Tableau
          entetes={["Droit", "Ce que vous pouvez faire", "Où"]}
          lignes={[
            [
              "Accès et portabilité",
              "Obtenir une copie de vos données dans un format lisible par machine",
              "Réglages › Mes données, ou par courriel",
            ],
            [
              "Rectification",
              "Corriger votre profil, vos critères, votre compte",
              "Directement dans l'application",
            ],
            [
              "Effacement",
              "Supprimer votre compte et tout ce qui s'y rattache",
              <a href="/suppression-compte">Voir la marche à suivre</a>,
            ],
            [
              "Opposition et limitation",
              "Mettre le compte en pause : vos plans sortent du fil, personne ne peut vous écrire, rien n'est supprimé — tout revient à la reprise",
              "Réglages › Mettre en pause",
            ],
            [
              "Retrait du consentement",
              "Retirer le consentement aux données sensibles, sans perdre le compte",
              "Réglages › Confidentialité",
            ],
          ]}
        />
        <p>
          Nous répondons dans un délai d'un mois, prolongeable de deux mois pour les demandes
          complexes — vous en seriez informé. Si notre réponse ne vous satisfait pas, vous pouvez
          saisir la <strong>CNIL</strong> (3 place de Fontenoy, 75007 Paris —{" "}
          <a href="https://www.cnil.fr" rel="noreferrer">
            cnil.fr
          </a>
          ).
        </p>
      </>
    ),
  },
  {
    id: "mineurs",
    titre: "Âge minimum",
    contenu: (
      <>
        <p>
          Weave est une application de rencontre : elle est{" "}
          <strong>réservée aux personnes de {MIN_AGE} ans et plus</strong>. La date de naissance est
          demandée à l'inscription et l'accès est refusé en dessous de cet âge.
        </p>
        <p>
          Un signalement indiquant qu'un compte appartient à une personne mineure est traité en
          priorité, entraîne la suspension immédiate du compte, et le cas échéant un signalement aux
          autorités compétentes.
        </p>
      </>
    ),
  },
  {
    id: "modifications",
    titre: "Modifications de cette politique",
    contenu: (
      <>
        <p>
          Chaque version de ce texte porte une date. En cas de changement substantiel, vous en êtes
          informé dans l'application avant son entrée en vigueur, et un nouveau consentement vous
          est demandé lorsque le changement l'exige.
        </p>
        <p>
          Les versions successives sont conservées : nous devons pouvoir établir quel texte vous
          aviez accepté, et à quelle date.
        </p>
      </>
    ),
  },
];

export function Confidentialite() {
  return (
    <Page
      titre="Politique de confidentialité"
      chapeau="Une application où l'on donne son heure et son lieu manipule ce qu'il y a de plus sensible. Voici exactement ce que nous collectons, pourquoi, combien de temps, et ce que nous nous interdisons."
      miseAJour={MISE_A_JOUR}
      articles={ARTICLES}
    />
  );
}
