/*
 * Conditions générales d'utilisation.
 *
 * Elles décrivent le service tel qu'il est réellement : les plafonds viennent
 * de `@weave/contracts`, donc du même endroit que le code qui les applique. Un
 * contrat qui annoncerait un plafond différent de celui appliqué serait une
 * clause inexacte, et c'est la version écrite qui ferait foi contre l'éditeur.
 */

import {
  MAX_OPEN_PLANS,
  MESSAGE_RETENTION_DAYS,
  MIN_AGE,
  PLAN_CAPACITY_GROUP_MAX,
  REQUESTS_PER_DAY_FLOOR,
} from "@weave/contracts";
import { AC, Page, type Article } from "./Page.tsx";
import { CONTACT, EDITEUR, MISE_A_JOUR } from "./identite.ts";

const ARTICLES: readonly Article[] = [
  {
    id: "objet",
    titre: "Objet",
    contenu: (
      <>
        <p>
          Les présentes conditions régissent l'utilisation de Weave, application éditée par{" "}
          <AC>{EDITEUR.raisonSociale}</AC>. Créer un compte vaut acceptation.
        </p>
        <p>
          Weave permet de publier des <strong>plans</strong> — ce que vous comptez faire, où et
          quand — et de demander à rejoindre ceux des autres en expliquant pourquoi. Le service ne
          propose pas de profils à balayer ni de classement payant.
        </p>
      </>
    ),
  },
  {
    id: "acces",
    titre: "Accès au service",
    contenu: (
      <>
        <p>
          Weave est <strong>réservé aux personnes de {MIN_AGE} ans et plus</strong>. La date de
          naissance est demandée à l'inscription ; toute déclaration inexacte entraîne la fermeture
          du compte.
        </p>
        <p>
          Un compte est personnel. Vous êtes responsable de son usage et de l'exactitude des
          informations que vous y publiez. La connexion se fait par un code envoyé à votre adresse
          e-mail : gardez cette boîte protégée, elle vaut mot de passe.
        </p>
      </>
    ),
  },
  {
    id: "regles",
    titre: "Les règles du service",
    contenu: (
      <>
        <p>
          Certaines limites ne sont pas des réglages : elles définissent le produit et s'appliquent
          à tous les paliers, y compris payants.
        </p>
        <ul>
          <li>
            <strong>Le nombre de demandes par jour est plafonné</strong> — au minimum{" "}
            {REQUESTS_PER_DAY_FLOOR} par jour. Aucun abonnement ne lève ce plafond : personne ne
            peut acheter le droit d'envoyer la même phrase à cinquante personnes.
          </li>
          <li>
            <strong>Aucun achat n'améliore le classement.</strong> Le fil est trié par imminence
            puis par proximité. Ni palier ni achat ne fait remonter un plan devant celui d'un autre.
          </li>
          <li>
            <strong>{MAX_OPEN_PLANS} plans ouverts au maximum</strong> par personne, et jusqu'à{" "}
            {PLAN_CAPACITY_GROUP_MAX} participants sur un plan de groupe.
          </li>
          <li>
            Une demande s'écrit : elle doit comporter un message. On ne rejoint pas un plan d'un
            simple geste.
          </li>
        </ul>
      </>
    ),
  },
  {
    id: "contenus",
    titre: "Vos contenus",
    contenu: (
      <>
        <p>
          Vos plans, messages et photos restent les vôtres. Vous nous concédez uniquement le droit
          de les stocker et de les afficher aux personnes concernées, pour la durée nécessaire au
          fonctionnement du service. Nous ne les exploitons à aucune autre fin — ni publicité, ni
          revente, ni entraînement de modèles.
        </p>
        <p>
          Vous garantissez disposer des droits sur ce que vous publiez, et que vos photos vous
          représentent.
        </p>
      </>
    ),
  },
  {
    id: "interdits",
    titre: "Ce qui est interdit",
    contenu: (
      <>
        <p>Sont notamment proscrits :</p>
        <ul>
          <li>le harcèlement, les menaces, les propos haineux ou discriminatoires ;</li>
          <li>
            les contenus à caractère sexuel explicite, et tout contenu impliquant une personne
            mineure ;
          </li>
          <li>
            l'usurpation d'identité, les faux profils, les photos qui ne vous représentent pas ;
          </li>
          <li>
            la sollicitation commerciale, la prostitution, les escroqueries — en particulier les
            arnaques sentimentales et les placements financiers ;
          </li>
          <li>
            la collecte automatisée de données, le contournement des plafonds, la création de
            comptes multiples pour les dépasser ;
          </li>
          <li>
            la diffusion des informations personnelles d'un tiers, y compris un lieu de rendez-vous
            qui vous a été confié en conversation.
          </li>
        </ul>
      </>
    ),
  },
  {
    id: "moderation",
    titre: "Signalement et modération",
    contenu: (
      <>
        <p>
          Chaque profil, plan et conversation peut être signalé.{" "}
          <strong>Un signalement entraîne toujours un blocage immédiat</strong> : personne n'a à
          revoir les plans de quelqu'un qu'il vient de signaler pendant l'examen du dossier.
        </p>
        <p>
          Le blocage coupe tout des deux côtés — demandes en attente closes, conversation fermée,
          plans retirés du fil — <strong>sans notification à la personne bloquée</strong>. Les
          messages déjà échangés restent lisibles par la personne qui a bloqué : les effacer
          détruirait aussi les preuves d'un comportement qu'elle vient de signaler.
        </p>
        <p>
          Selon la gravité, nous pouvons avertir, suspendre ou fermer un compte. Toute décision peut
          être contestée à{" "}
          <a href={`mailto:${CONTACT.signalements}`}>
            <AC>{CONTACT.signalements}</AC>
          </a>
          , et nous réexaminons le dossier.
        </p>
      </>
    ),
  },
  {
    id: "securite",
    titre: "Votre sécurité lors des rencontres",
    contenu: (
      <>
        <p>
          Weave met en relation des personnes qui se retrouvent dans la vie réelle.{" "}
          <strong>
            Nous ne vérifions pas l'identité des utilisateurs et ne procédons à aucun contrôle
            d'antécédents.
          </strong>
        </p>
        <p>Quelques précautions valent d'être rappelées :</p>
        <ul>
          <li>préférez un lieu public pour un premier rendez-vous ;</li>
          <li>dites à quelqu'un où vous allez et avec qui ;</li>
          <li>
            ne communiquez l'adresse exacte qu'aux personnes que vous avez acceptées — c'est
            précisément pourquoi le fil n'affiche jamais qu'une ville et une distance ;
          </li>
          <li>n'envoyez jamais d'argent à quelqu'un rencontré sur l'application.</li>
        </ul>
      </>
    ),
  },
  {
    id: "disponibilite",
    titre: "Disponibilité et évolutions",
    contenu: (
      <>
        <p>
          Nous nous efforçons d'assurer la continuité du service sans pouvoir la garantir : des
          interruptions peuvent survenir pour maintenance ou pour cause extérieure. Le service peut
          évoluer ; un changement substantiel des présentes conditions vous est annoncé avant son
          entrée en vigueur.
        </p>
        <p>
          Notre responsabilité ne saurait être engagée pour le comportement des utilisateurs entre
          eux, ni pour ce qui advient lors d'une rencontre. Aucune clause ne limite votre
          responsabilité en cas de faute lourde ou de dol, ni ne prive un consommateur des droits
          que la loi lui reconnaît.
        </p>
      </>
    ),
  },
  {
    id: "fin",
    titre: "Fin du contrat",
    contenu: (
      <>
        <p>
          Vous pouvez fermer votre compte à tout moment — la{" "}
          <a href="/suppression-compte">marche à suivre est décrite ici</a>. Les messages déjà
          envoyés restent lisibles par leurs destinataires jusqu'à la purge de leur conversation,{" "}
          {MESSAGE_RETENTION_DAYS} jours après sa clôture.
        </p>
        <p>
          Nous pouvons fermer un compte en cas de manquement grave ou répété aux présentes
          conditions, après information de la personne concernée sauf lorsque la sécurité d'autrui
          impose d'agir sans délai.
        </p>
      </>
    ),
  },
  {
    id: "droit",
    titre: "Droit applicable et litiges",
    contenu: (
      <>
        <p>
          Les présentes conditions sont soumises au droit français. En cas de différend,
          adressez-vous d'abord à{" "}
          <a href={`mailto:${CONTACT.support}`}>
            <AC>{CONTACT.support}</AC>
          </a>{" "}
          : la plupart se règlent ainsi.
        </p>
        <p>
          À défaut d'accord, un consommateur peut recourir gratuitement à un médiateur de la
          consommation, ou saisir la plateforme européenne de règlement en ligne des litiges. Les
          tribunaux compétents sont ceux désignés par les règles de droit commun ; un consommateur
          peut toujours saisir la juridiction de son lieu de résidence.
        </p>
      </>
    ),
  },
];

export function Cgu() {
  return (
    <Page
      titre="Conditions générales d'utilisation"
      chapeau="Ce que Weave vous propose, ce que nous attendons de vous, et ce qui arrive quand les règles ne sont pas tenues."
      miseAJour={MISE_A_JOUR}
      articles={ARTICLES}
    />
  );
}
