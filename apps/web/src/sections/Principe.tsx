import {
  MAX_OPEN_PLANS,
  PLAN_CATEGORIES,
  PLAN_CATEGORY_LABELS_PAR_LANGUE,
  REQUESTS_PER_DAY_FLOOR,
  REQUEST_MIN_CHARS,
} from "@weave/contracts";
import { Carte, filVar, Section, type Fil } from "../composants.tsx";
import { useTraduction } from "../contexte-langue.tsx";
import type { Traduit } from "../langues.ts";

type Pilier = { readonly titre: string; readonly texte: string };

const PILIERS: Traduit<readonly Pilier[]> = {
  fr: [
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
  ],
  en: [
    {
      titre: "You can't spray and pray",
      texte: `The number of requests you can send in a day is capped — at least ${REQUESTS_PER_DAY_FLOOR}, on every plan, the free tier included. When a request costs you something, you choose it.`,
    },
    {
      titre: "You can't buy visibility",
      texte:
        "The feed is sorted by how soon, then by how near. No subscription, no purchase puts one plan ahead of someone else's. That is the rule Weave will not change.",
    },
    {
      titre: "You ask by writing",
      texte: `There is no gesture that means “I'm coming”. You write at least ${REQUEST_MIN_CHARS} characters saying why. That is what separates a request from a reflex.`,
    },
    {
      titre: "A plan has a date, so it has an end",
      texte: `Once the time has passed, the plan leaves the feed. Nothing piles up, nothing lingers. And you keep only ${MAX_OPEN_PLANS} open at a time: the things you actually mean to do.`,
    },
  ],
  es: [
    {
      titre: "No se puede lanzar la caña a todo el mundo",
      texte: `El número de peticiones que puedes enviar al día está limitado — al menos ${REQUESTS_PER_DAY_FLOOR}, en todas las suscripciones, incluida la gratuita. Cuando una petición cuesta algo, la eliges.`,
    },
    {
      titre: "No se puede comprar visibilidad",
      texte:
        "El muro se ordena por lo que ocurre antes y luego por cercanía. Ninguna suscripción, ninguna compra coloca un plan por delante del de otra persona. Es la regla que Weave no va a cambiar.",
    },
    {
      titre: "Se pide escribiendo",
      texte: `No existe ningún gesto para decir « voy ». Escribes al menos ${REQUEST_MIN_CHARS} caracteres que digan por qué. Eso es lo que distingue una petición de un reflejo.`,
    },
    {
      titre: "Un plan tiene fecha, así que tiene final",
      texte: `Pasada la cita, el plan desaparece del muro. Nada se acumula, nada se queda. Y solo mantienes ${MAX_OPEN_PLANS} abiertos a la vez: lo que de verdad piensas hacer.`,
    },
  ],
};

const ABSENTS: Traduit<readonly string[]> = {
  fr: [
    "Pile de cartes à balayer",
    "Liste des personnes qui vous ont remarqué",
    "Mise en avant payante dans le fil",
    "Compteur de « vues » de votre profil",
    "Publicité et revente de données",
    "Notification inventée pour vous faire revenir",
  ],
  en: [
    "A pile of cards to swipe",
    "A list of people who noticed you",
    "Paid placement in the feed",
    "A counter of profile “views”",
    "Advertising and data resale",
    "A notification invented to bring you back",
  ],
  es: [
    "Una pila de tarjetas que deslizar",
    "Una lista de quién se ha fijado en ti",
    "Destacados de pago en el muro",
    "Un contador de « visitas » a tu perfil",
    "Publicidad y reventa de datos",
    "Una notificación inventada para hacerte volver",
  ],
};

const TEXTES: Traduit<{
  titre: string;
  chapeau: string;
  absents: string;
  publie: string;
  publieTexte: string;
}> = {
  fr: {
    titre: "Le principe",
    chapeau:
      "Une fiche dit qui on prétend être. Un plan dit ce qu'on fait jeudi. Weave ne garde que le second.",
    absents: "Ce que Weave ne fait pas",
    publie: "Ce qu'on y publie",
    publieTexte:
      "Rien de spectaculaire, et c'est voulu : ce qu'on allait faire de toute façon. Un plan réussi, c'est un samedi qu'on n'a pas passé seul.",
  },
  en: {
    titre: "The idea",
    chapeau:
      "A profile says who you claim to be. A plan says what you're doing on Thursday. Weave keeps only the second.",
    absents: "What Weave does not do",
    publie: "What people post",
    publieTexte:
      "Nothing spectacular, and that's the point: the thing you were going to do anyway. A plan that worked is a Saturday you didn't spend alone.",
  },
  es: {
    titre: "La idea",
    chapeau:
      "Un perfil dice quién dices ser. Un plan dice qué haces el jueves. Weave solo se queda con lo segundo.",
    absents: "Lo que Weave no hace",
    publie: "Lo que se publica",
    publieTexte:
      "Nada espectacular, y es a propósito: lo que ibas a hacer de todos modos. Un plan que sale bien es un sábado que no pasaste solo.",
  },
};

export function Principe() {
  const t = useTraduction(TEXTES);
  const piliers = useTraduction(PILIERS);
  const absents = useTraduction(ABSENTS);
  const categories = useTraduction(PLAN_CATEGORY_LABELS_PAR_LANGUE);

  return (
    <Section id="principe" fil={4} titre={t.titre} chapeau={t.chapeau} alterne>
      <div className="grid gap-4 sm:grid-cols-2">
        {piliers.map((pilier, index) => (
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
            {t.absents}
          </h3>
          <ul className="mt-4 space-y-2.5">
            {absents.map((absent) => (
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
            {t.publie}
          </h3>
          <p className="mt-4 leading-relaxed" style={{ color: "var(--texte-doux)" }}>
            {t.publieTexte}
          </p>
          <ul className="mt-5 flex flex-wrap gap-2">
            {PLAN_CATEGORIES.map((categorie, index) => (
              <li
                key={categorie}
                className="rounded-full px-4 py-2 text-sm font-bold"
                /*
                  Le texte est à la couleur du texte, pas à celle du fil.
                  
                  Il portait la couleur du fil sur un aplat du même fil à 16 % :
                  deux tons d'une même teinte, donc un contraste de 2,7 à 3,9
                  selon la couleur — sous le seuil de 4,5 que réclame un texte
                  de cette taille. Éclaircir l'aplat n'y suffisait pas : le
                  safran lui-même ne dépasse pas 3,0 sur du blanc.
                  
                  La couleur reste dite par l'aplat ET par le trait, qui
                  n'ont rien à lire. Le libellé, lui, se lit.
                */
                style={{
                  background: `color-mix(in oklab, ${filVar(((index % 6) + 1) as Fil)} 16%, var(--fond))`,
                  border: `1.5px solid ${filVar(((index % 6) + 1) as Fil)}`,
                  color: "var(--texte)",
                }}
              >
                {categories[categorie]}
              </li>
            ))}
          </ul>
        </div>
      </div>
    </Section>
  );
}
