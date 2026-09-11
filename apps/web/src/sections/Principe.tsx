import { MAX_ACTIVE_THREADS, REVEAL_STEPS, THREAD_TTL_SECONDS } from "@weave/contracts";
import { Carte, filVar, Section, type Fil } from "../composants.tsx";

const PILIERS = [
  {
    titre: `${MAX_ACTIVE_THREADS} fils, jamais plus`,
    texte:
      "Le plafond est le même pour tout le monde, quel que soit l'abonnement. Aucune offre ne le relève : ce serait vendre exactement ce que Weave cherche à éviter.",
  },
  {
    titre: "En mémoire, pas en base",
    texte:
      "Les profils qui vous sont proposés n'existent que dans le cache du service, avec une durée de vie. Nous ne constituons pas de collection de profils consultés.",
  },
  {
    titre: "On engage en écrivant",
    texte:
      "Il n'y a pas de geste pour dire oui ou non. Un fil s'engage par une réponse à un fragment ; l'autre personne voit ce que vous avez écrit, pas un signal.",
  },
  {
    titre: "Le temps fait le tri",
    texte: `Un fil non engagé se dénoue après ${THREAD_TTL_SECONDS / 3600} heures et disparaît. Il n'y a pas de file d'attente qui grossit, pas de retour en arrière payant.`,
  },
] as const;

const ABSENTS = [
  "Pile de cartes à balayer",
  "Liste des personnes qui vous ont aimé",
  "Mise en avant payante dans le vivier",
  "Compteur de « vues » de votre profil",
  "Publicité et revente de données",
  "Relance automatique pour vous faire revenir",
] as const;

export function Principe() {
  return (
    <Section
      id="principe"
      fil={4}
      titre="Le principe"
      chapeau="Weave part d'une contrainte assumée : moins de profils, plus d'attention. Tout le reste en découle."
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
            La révélation progressive
          </h3>
          <p className="mt-4 leading-relaxed" style={{ color: "var(--texte-doux)" }}>
            La photo n'ouvre pas la rencontre, elle l'accompagne. À chaque échange abouti — une
            réponse de chaque côté — elle se précise d'un cran.
          </p>
          <ol className="mt-5 space-y-3">
            {REVEAL_STEPS.map((etape, index) => (
              <li key={etape} className="flex items-center gap-4">
                <span
                  className="w-14 shrink-0 text-right text-sm font-semibold tabular-nums"
                  style={{ color: filVar(6) }}
                >
                  {etape} %
                </span>
                <span
                  className="h-2 flex-1 rounded-full"
                  aria-hidden="true"
                  style={{ background: "var(--bordure)" }}
                >
                  <span
                    className="block h-2 rounded-full"
                    style={{
                      width: `${etape}%`,
                      background: `linear-gradient(90deg, ${filVar(1)}, ${filVar(4)}, ${filVar(6)})`,
                    }}
                  />
                </span>
                <span className="w-28 shrink-0 text-sm" style={{ color: "var(--texte-doux)" }}>
                  {index === 0 ? "au tissage" : `${index} échange${index > 1 ? "s" : ""}`}
                </span>
              </li>
            ))}
          </ol>
        </div>
      </div>
    </Section>
  );
}
