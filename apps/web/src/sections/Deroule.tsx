import { MAX_OPEN_PLANS, PLAN_MIN_LEAD_MINUTES, REQUESTS_PER_DAY_FLOOR } from "@weave/contracts";
import { Pastille, Section, type Fil } from "../composants.tsx";
import { useTraduction } from "../contexte-langue.tsx";
import type { Traduit } from "../langues.ts";

type Etape = { readonly titre: string; readonly texte: string };

const ETAPES: Traduit<readonly Etape[]> = {
  fr: [
    {
      titre: "Vous dites où vous êtes, et c'est tout",
      texte:
        "Une ville, un genre, une phrase si vous voulez. Pas de questionnaire, pas de description de vous-même à rédiger : ce n'est pas ce qu'on va lire.",
    },
    {
      titre: "Vous publiez un plan",
      texte: `Ce que vous comptez faire, quand, et combien de personnes peuvent venir. Au moins ${PLAN_MIN_LEAD_MINUTES} minutes à l'avance, et au plus ${MAX_OPEN_PLANS} plans ouverts en même temps.`,
    },
    {
      titre: "Vous lisez le fil des autres",
      texte:
        "Les plans autour de vous, du plus imminent au plus lointain. Pas de pile à balayer : une liste qui se vide toute seule quand les dates passent.",
    },
    {
      titre: "Vous demandez à venir, en écrivant",
      texte: `Quelques lignes qui disent pourquoi ce plan-là. Vous en avez un nombre limité par jour — au minimum ${REQUESTS_PER_DAY_FLOOR}, même sans payer — et elles reviennent à minuit.`,
    },
    {
      titre: "La personne accepte, ou non",
      texte:
        "Un refus ne se commente pas et ne notifie rien d'accusateur. Une demande retirée avant d'avoir été lue vous est rendue.",
    },
    {
      titre: "La conversation s'ouvre",
      texte:
        "Seulement après un oui, et seulement à deux — même sur un plan de groupe. Elle se ferme quand vous voulez, et les messages sont purgés ensuite.",
    },
  ],
  en: [
    {
      titre: "You say where you are, and that's it",
      texte:
        "A town, a gender, a sentence if you feel like it. No questionnaire, no self-description to write: that isn't what anyone is going to read.",
    },
    {
      titre: "You post a plan",
      texte: `What you're doing, when, and how many people can come. At least ${PLAN_MIN_LEAD_MINUTES} minutes ahead, and at most ${MAX_OPEN_PLANS} plans open at once.`,
    },
    {
      titre: "You read other people's feed",
      texte:
        "The plans around you, soonest first. No pile to swipe through: a list that empties itself as the dates pass.",
    },
    {
      titre: "You ask to come, in writing",
      texte: `A few lines saying why that plan. You get a limited number each day — at least ${REQUESTS_PER_DAY_FLOOR}, even without paying — and they come back at midnight.`,
    },
    {
      titre: "They say yes, or they don't",
      texte:
        "A refusal carries no comment and sends nothing accusing. A request withdrawn before it was read is given back to you.",
    },
    {
      titre: "The conversation opens",
      texte:
        "Only after a yes, and only between two people — even on a group plan. You close it whenever you like, and the messages are purged afterwards.",
    },
  ],
  es: [
    {
      titre: "Dices dónde estás, y ya está",
      texte:
        "Una ciudad, un género, una frase si te apetece. Sin cuestionario, sin descripción de ti mismo que redactar: no es eso lo que se va a leer.",
    },
    {
      titre: "Publicas un plan",
      texte: `Lo que piensas hacer, cuándo, y cuántas personas pueden venir. Con al menos ${PLAN_MIN_LEAD_MINUTES} minutos de antelación, y como máximo ${MAX_OPEN_PLANS} planes abiertos a la vez.`,
    },
    {
      titre: "Lees el muro de los demás",
      texte:
        "Los planes a tu alrededor, del más próximo al más lejano. Sin pila que deslizar: una lista que se vacía sola cuando pasan las fechas.",
    },
    {
      titre: "Pides venir, escribiendo",
      texte: `Unas líneas que digan por qué ese plan. Tienes un número limitado al día — al menos ${REQUESTS_PER_DAY_FLOOR}, incluso sin pagar — y vuelven a medianoche.`,
    },
    {
      titre: "La persona acepta, o no",
      texte:
        "Un rechazo no se comenta y no notifica nada acusador. Una petición retirada antes de ser leída se te devuelve.",
    },
    {
      titre: "Se abre la conversación",
      texte:
        "Solo tras un sí, y solo entre dos — incluso en un plan de grupo. Se cierra cuando quieras, y los mensajes se purgan después.",
    },
  ],
};

const TEXTES: Traduit<{ titre: string; chapeau: string }> = {
  fr: {
    titre: "Comment ça se passe",
    chapeau: "Six étapes, et vous pouvez en rester à la troisième aussi longtemps que vous voulez.",
  },
  en: {
    titre: "How it goes",
    chapeau: "Six steps, and you can stop at the third one for as long as you like.",
  },
  es: {
    titre: "Cómo funciona",
    chapeau: "Seis pasos, y puedes quedarte en el tercero todo el tiempo que quieras.",
  },
};

export function Deroule() {
  const t = useTraduction(TEXTES);
  const etapes = useTraduction(ETAPES);

  return (
    <Section id="deroule" fil={2} titre={t.titre} chapeau={t.chapeau}>
      <ol className="space-y-6">
        {etapes.map((etape, index) => (
          <li key={etape.titre} className="flex gap-5">
            <Pastille fil={((index % 6) + 1) as Fil}>{index + 1}</Pastille>
            <div className="pt-1">
              <h3 className="text-lg font-bold">{etape.titre}</h3>
              <p className="mt-2 leading-relaxed" style={{ color: "var(--texte-doux)" }}>
                {etape.texte}
              </p>
            </div>
          </li>
        ))}
      </ol>
    </Section>
  );
}
