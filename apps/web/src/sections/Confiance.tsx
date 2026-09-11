import { ACCOUNT_PURGE_DAYS, MESSAGE_RETENTION_DAYS, MIN_AGE } from "@weave/contracts";
import { Carte, filVar, Section, type Fil } from "../composants.tsx";

const ENGAGEMENTS = [
  {
    titre: "Rien à vendre à personne",
    texte:
      "Pas de publicité, pas de courtier en données, pas de revente. Le produit est financé par celles et ceux qui s'abonnent ou achètent à l'unité.",
  },
  {
    titre: "Localisation au kilomètre",
    texte:
      "Votre position est arrondie avant d'être enregistrée, et un plan n'affiche jamais d'adresse dans le fil : une ville, une distance arrondie. L'endroit exact se dit dans la conversation, à qui vous avez accepté.",
  },
  {
    titre: "Ce que vous consultez n'est pas archivé",
    texte:
      "Le fil est recomposé à la demande et vit quelques minutes dans un cache. Nous ne constituons pas d'historique des plans que vous avez regardés, et personne ne peut le consulter.",
  },
  {
    titre: "Blocage immédiat",
    texte:
      "Bloquer ou signaler coupe tout des deux côtés — demandes en attente closes, conversation fermée — sans notification à l'autre personne. Un signalement entraîne toujours un blocage.",
  },
] as const;

export function Confiance() {
  return (
    <Section
      id="confiance"
      fil={1}
      titre="Ce à quoi nous nous engageons"
      chapeau="Une application où l'on donne son heure et son lieu manipule ce qu'il y a de plus sensible. Voici ce que nous nous interdisons."
      alterne
    >
      <div className="grid gap-4 sm:grid-cols-2">
        {ENGAGEMENTS.map((engagement, index) => (
          <Carte key={engagement.titre} fil={((index % 6) + 1) as Fil}>
            <h3 className="text-lg font-bold" style={{ color: filVar(((index % 6) + 1) as Fil) }}>
              {engagement.titre}
            </h3>
            <p className="mt-2.5 leading-relaxed" style={{ color: "var(--texte-doux)" }}>
              {engagement.texte}
            </p>
          </Carte>
        ))}
      </div>

      <dl className="mt-10 grid gap-6 sm:grid-cols-3">
        <div>
          <dt className="text-sm" style={{ color: "var(--texte-doux)" }}>
            Âge minimum
          </dt>
          <dd className="mt-1 text-2xl font-semibold tabular-nums">{MIN_AGE} ans</dd>
        </div>
        <div>
          <dt className="text-sm" style={{ color: "var(--texte-doux)" }}>
            Messages conservés
          </dt>
          <dd className="mt-1 text-2xl font-semibold tabular-nums">
            {MESSAGE_RETENTION_DAYS} jours
          </dd>
          <p className="mt-1 text-sm" style={{ color: "var(--texte-doux)" }}>
            après clôture d'une conversation
          </p>
        </div>
        <div>
          <dt className="text-sm" style={{ color: "var(--texte-doux)" }}>
            Compte supprimé
          </dt>
          <dd className="mt-1 text-2xl font-semibold tabular-nums">{ACCOUNT_PURGE_DAYS} jours</dd>
          <p className="mt-1 text-sm" style={{ color: "var(--texte-doux)" }}>
            avant purge définitive
          </p>
        </div>
      </dl>
    </Section>
  );
}
