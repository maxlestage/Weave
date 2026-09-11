import { MAX_OPEN_PLANS, PLAN_MIN_LEAD_MINUTES, REQUESTS_PER_DAY_FLOOR } from "@weave/contracts";
import { Pastille, Section, type Fil } from "../composants.tsx";

const ETAPES = [
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
] as const;

export function Deroule() {
  return (
    <Section
      id="deroule"
      fil={2}
      titre="Comment ça se passe"
      chapeau="Six étapes, et vous pouvez en rester à la troisième aussi longtemps que vous voulez."
    >
      <ol className="space-y-6">
        {ETAPES.map((etape, index) => (
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
