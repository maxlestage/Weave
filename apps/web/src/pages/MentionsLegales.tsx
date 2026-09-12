/*
 * Mentions légales.
 *
 * Obligation : article 6-III-1 de la loi pour la confiance dans l'économie
 * numérique. Un éditeur professionnel doit publier son identité, ses
 * coordonnées, son immatriculation et celles de son hébergeur, de manière
 * directement accessible. L'absence de ces mentions est pénalement
 * sanctionnée — d'où le surlignage voyant de tout ce qui manque.
 */

import { AC, Page, type Article } from "./Page.tsx";
import { CONTACT, EDITEUR, MISE_A_JOUR } from "./identite.ts";

const ARTICLES: readonly Article[] = [
  {
    id: "editeur",
    titre: "Éditeur du site et de l'application",
    contenu: (
      <>
        <ul>
          <li>
            Dénomination : <AC>{EDITEUR.raisonSociale}</AC>
          </li>
          <li>
            Forme juridique et capital : <AC>{EDITEUR.formeJuridique}</AC>
          </li>
          <li>
            Siège social : <AC>{EDITEUR.adresse}</AC>
          </li>
          <li>
            Immatriculation : <AC>{EDITEUR.immatriculation}</AC>
          </li>
          <li>
            TVA intracommunautaire : <AC>{EDITEUR.tva}</AC>
          </li>
          <li>
            Téléphone : <AC>{EDITEUR.telephone}</AC>
          </li>
          <li>
            Courriel :{" "}
            <a href={`mailto:${CONTACT.support}`}>
              <AC>{CONTACT.support}</AC>
            </a>
          </li>
        </ul>
      </>
    ),
  },
  {
    id: "publication",
    titre: "Directeur de la publication",
    contenu: (
      <p>
        <AC>{EDITEUR.directeurPublication}</AC>
      </p>
    ),
  },
  {
    id: "hebergeur",
    titre: "Hébergeur",
    contenu: (
      <ul>
        <li>
          Nom : <AC>{EDITEUR.hebergeur.nom}</AC>
        </li>
        <li>
          Adresse : <AC>{EDITEUR.hebergeur.adresse}</AC>
        </li>
        <li>
          Téléphone : <AC>{EDITEUR.hebergeur.telephone}</AC>
        </li>
      </ul>
    ),
  },
  {
    id: "signalements",
    titre: "Point de contact et signalements",
    contenu: (
      <>
        <p>
          Le règlement européen sur les services numériques impose un point de contact unique pour
          les signalements de contenus illicites. Vous pouvez nous écrire à{" "}
          <a href={`mailto:${CONTACT.signalements}`}>
            <AC>{CONTACT.signalements}</AC>
          </a>
          .
        </p>
        <p>
          Dans l'application, chaque profil, plan et conversation dispose d'une fonction de
          signalement, qui entraîne toujours un blocage immédiat de la personne signalée. Les
          décisions de modération peuvent être contestées à la même adresse.
        </p>
      </>
    ),
  },
  {
    id: "donnees",
    titre: "Données personnelles",
    contenu: (
      <p>
        Le traitement de vos données est décrit dans la{" "}
        <a href="/confidentialite">politique de confidentialité</a>, qui précise les finalités, les
        bases légales, les durées de conservation et la manière d'exercer vos droits.
      </p>
    ),
  },
  {
    id: "propriete",
    titre: "Propriété intellectuelle",
    contenu: (
      <>
        <p>
          La marque Weave, le nom de domaine, le logo, l'identité visuelle, les textes et le code de
          l'application sont la propriété de <AC>{EDITEUR.raisonSociale}</AC>, sauf mention
          contraire.
        </p>
        <p>
          Les contenus que vous publiez — plans, messages, photos — restent les vôtres. Vous nous
          concédez uniquement le droit de les afficher aux personnes concernées dans le cadre du
          fonctionnement du service, conformément aux{" "}
          <a href="/cgu">conditions générales d'utilisation</a>.
        </p>
        <p>Apple, iPhone et Apple Watch sont des marques déposées d'Apple Inc.</p>
      </>
    ),
  },
  {
    id: "accessibilite",
    titre: "Accessibilité",
    contenu: (
      <p>
        Ce site est conçu pour rester utilisable au clavier, avec un lecteur d'écran, et à fort
        grossissement. Si vous rencontrez un obstacle, écrivez-nous à{" "}
        <a href={`mailto:${CONTACT.support}`}>
          <AC>{CONTACT.support}</AC>
        </a>{" "}
        : nous le corrigerons.
      </p>
    ),
  },
];

export function MentionsLegales() {
  return (
    <Page
      titre="Mentions légales"
      chapeau="Qui édite Weave, qui l'héberge, et à qui écrire."
      miseAJour={MISE_A_JOUR}
      articles={ARTICLES}
    />
  );
}
