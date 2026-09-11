import {
  MAX_OPEN_PLANS,
  PLAN_CATEGORIES,
  PLAN_CATEGORY_LABELS,
  REQUESTS_PER_DAY_FLOOR,
  REQUEST_MIN_CHARS,
} from "@weave/contracts";
import { Carte, filVar, Section, type Fil } from "../composants.tsx";

const PILIERS = [
  {
    titre: "On ne peut pas arroser",
    texte: `Le nombre de demandes qu'on peut envoyer dans une journée est borné — au minimum ${REQUESTS_PER_DAY_FLOOR}, à toutes les offres, socle gratuit compris. Quand une demande coûte quelque chose, on la choisit.`,
  },
  {
    titre: "On ne peut pas acheter de visibilité",
    texte:
      "Le fil est trié par imminence puis par proximité. Aucun abonnement, aucun achat ne place un plan devant celui de quelqu'un d'autre. C'est la règle que Weave ne changera pas.",
  },
  {
    titre: "On demande en écrivant",
    texte: `Il n'existe aucun geste pour dire « je viens ». On écrit au moins ${REQUEST_MIN_CHARS} caractères qui disent pourquoi. C'est ce qui distingue une demande d'un réflexe.`,
  },
  {
    titre: "Un plan a une date, donc une fin",
    texte: `Passé le rendez-vous, le plan disparaît du fil. Rien ne s'accumule, rien ne traîne. Et vous n'en gardez que ${MAX_OPEN_PLANS} ouverts à la fois : ce que vous comptez vraiment faire.`,
  },
] as const;

const ABSENTS = [
  "Pile de cartes à balayer",
  "Liste des personnes qui vous ont remarqué",
  "Mise en avant payante dans le fil",
  "Compteur de « vues » de votre profil",
  "Publicité et revente de données",
  "Notification inventée pour vous faire revenir",
] as const;

export function Principe() {
  return (
    <Section
      id="principe"
      fil={4}
      titre="Le principe"
      chapeau="Une fiche dit qui on prétend être. Un plan dit ce qu'on fait jeudi. Weave ne garde que le second."
      alterne
    >
      <div className="grid gap-4 sm:grid-cols-2">
        {PILIERS.map((pilier, index) => (
          <Carte key={pilier.titre} fil={((index % 6) + 1) as Fil}>
            <h3
              className="text-xl font-bold"
              style={{ fontFamily: "var(--font-titre)", color: filVar(((index % 6) + 1) as Fil) }}
            >
              {pilier.titre}
            </h3>
            <p className="mt-3 leading-relaxed" style={{ color: "var(--texte-doux)" }}>
              {pilier.texte}
            </p>
          </Carte>
        ))}
      </div>

      <div className="mt-12 grid gap-8 sm:grid-cols-2">
        <div>
          <h3 className="text-xl font-semibold" style={{ fontFamily: "var(--font-titre)" }}>
            Ce que Weave ne fait pas
          </h3>
          <ul className="mt-4 space-y-2.5">
            {ABSENTS.map((absent) => (
              <li key={absent} className="flex items-start gap-3">
                <svg
                  width="20"
                  height="20"
                  viewBox="0 0 20 20"
                  className="mt-0.5 shrink-0"
                  aria-hidden="true"
                >
                  <path
                    d="M6 6l8 8M14 6l-8 8"
                    stroke="var(--texte-doux)"
                    strokeWidth="1.6"
                    strokeLinecap="round"
                  />
                </svg>
                <span style={{ color: "var(--texte-doux)" }}>{absent}</span>
              </li>
            ))}
          </ul>
        </div>

        <div>
          <h3 className="text-xl font-semibold" style={{ fontFamily: "var(--font-titre)" }}>
            Ce qu'on y publie
          </h3>
          <p className="mt-4 leading-relaxed" style={{ color: "var(--texte-doux)" }}>
            Rien de spectaculaire, et c'est voulu : ce qu'on allait faire de toute façon. Un plan
            réussi, c'est un samedi qu'on n'a pas passé seul.
          </p>
          <ul className="mt-5 flex flex-wrap gap-2">
            {PLAN_CATEGORIES.map((categorie, index) => (
              <li
                key={categorie}
                className="rounded-full px-4 py-2 text-sm font-bold"
                style={{
                  background: `color-mix(in oklab, ${filVar(((index % 6) + 1) as Fil)} 16%, var(--fond))`,
                  color: filVar(((index % 6) + 1) as Fil),
                }}
              >
                {PLAN_CATEGORY_LABELS[categorie]}
              </li>
            ))}
          </ul>
        </div>
      </div>
    </Section>
  );
}
