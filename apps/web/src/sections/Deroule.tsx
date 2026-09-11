import { MAX_ACTIVE_THREADS, MOTIF_TAGS, WEAVING_HOURS } from "@weave/contracts";
import { Pastille, Section, type Fil } from "../composants.tsx";

const ETAPES = [
  {
    titre: "Vous répondez à quelques questions",
    texte:
      "Pas de description de vous-même à rédiger. Des questions précises, auxquelles vous répondez par écrit ou à la voix, en huit secondes.",
  },
  {
    titre: `Vous tissez votre motif : ${MOTIF_TAGS} mots`,
    texte:
      "Cinq mots qui vous situent, tirés de vos réponses. C'est ce que les autres voient en premier — avant votre visage.",
  },
  {
    titre: "Vous choisissez votre heure de tissage",
    texte: `Un rendez-vous quotidien, à ${WEAVING_HOURS.slice(0, -1).join(" h, ")} h ou ${WEAVING_HOURS.at(-1)} h. C'est le seul moment où Weave vous sollicite.`,
  },
  {
    titre: `${MAX_ACTIVE_THREADS} fils arrivent`,
    texte:
      "Chacun se présente par sa trame : trois fragments et un motif. La photo est là, mais floue.",
  },
  {
    titre: "Vous répondez, ou vous laissez faire",
    texte:
      "Répondre engage le fil. Ne rien faire le laisse se dénouer — c'est une réponse aussi, et elle ne coûte rien à personne.",
  },
  {
    titre: "Le fil se tisse",
    texte:
      "Quand les deux personnes ont répondu, la conversation s'ouvre et la photo commence à se dévoiler.",
  },
] as const;

export function Deroule() {
  return (
    <Section
      id="deroule"
      fil={2}
      titre="Comment ça se passe"
      chapeau="Six étapes, dont une seule vous demande de décider quoi que ce soit."
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
