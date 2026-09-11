import { MAX_ACTIVE_THREADS } from "@weave/contracts";
import { Carte, filVar, Section } from "../composants.tsx";

export function Appareils() {
  return (
    <Section
      id="montre"
      fil={5}
      titre="Sur l'iPhone, et au poignet"
      chapeau="L'application est écrite en Swift, nativement. La Live Activity et l'application Apple Watch ne sont pas des extras : elles servent précisément à ne pas ouvrir l'application."
      alterne
    >
      <div className="grid gap-4 lg:grid-cols-2">
        <Carte fil={5}>
          <h3
            className="text-xl font-bold"
            style={{ fontFamily: "var(--font-titre)", color: filVar(5) }}
          >
            Live Activity
          </h3>
          <p className="mt-3 leading-relaxed" style={{ color: "var(--texte-doux)" }}>
            À votre heure de tissage, une bannière apparaît d'elle-même sur l'écran verrouillé et
            dans l'île dynamique. Elle affiche le nombre de fils, le temps qu'il reste au plus
            pressé, et rien d'autre : ce qui est visible sur un écran verrouillé doit pouvoir être
            lu par quelqu'un d'autre.
          </p>
          <ApercuActivite />
        </Carte>

        <Carte fil={4}>
          <h3
            className="text-xl font-bold"
            style={{ fontFamily: "var(--font-titre)", color: filVar(4) }}
          >
            Apple Watch
          </h3>
          <p className="mt-3 leading-relaxed" style={{ color: "var(--texte-doux)" }}>
            Une complication sur le cadran, la liste des fils et leur échéance, et de quoi répondre
            à la voix ou par dictée. La montre reçoit un résumé compact : ni photo, ni fragment.
          </p>
          <ul className="mt-5 space-y-2.5 text-sm" style={{ color: "var(--texte-doux)" }}>
            <li>• Complication : fils en attente et compte à rebours</li>
            <li>• Réponse par dictée ou saisie manuscrite</li>
            <li>• Synchronisation par WatchConnectivity, et repli sur le réseau</li>
          </ul>
        </Carte>
      </div>

      <p className="mt-8 text-sm" style={{ color: "var(--texte-doux)" }}>
        Ce site n'est qu'une présentation : Weave se vit dans l'application.
      </p>
    </Section>
  );
}

/** Maquette statique de la bannière : illustration, pas capture d'écran. */
function ApercuActivite() {
  return (
    <div
      className="mt-6 rounded-2xl p-4"
      role="img"
      aria-label={`Aperçu de la Live Activity : ${MAX_ACTIVE_THREADS} fils, 1 en attente, 4 h 12 restantes`}
      style={{ background: "var(--color-encre)", color: "#f4efe7" }}
    >
      <div className="flex items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <svg width="24" height="24" viewBox="0 0 32 32" aria-hidden="true">
            <g
              stroke="var(--color-cuivre-clair)"
              strokeWidth="2.4"
              strokeLinecap="round"
              fill="none"
            >
              <path d="M8 6v20" />
              <path d="M16 6v20" />
              <path d="M24 6v20" />
            </g>
          </svg>
          <div>
            <p className="text-sm font-semibold">{MAX_ACTIVE_THREADS} fils sur le métier</p>
            <p className="text-xs" style={{ color: "#b6aa9c" }}>
              1 attend votre réponse
            </p>
          </div>
        </div>
        <div className="text-right">
          <p
            className="text-lg font-semibold tabular-nums"
            style={{ color: "var(--color-cuivre-clair)" }}
          >
            4 h 12
          </p>
          <p className="text-xs" style={{ color: "#b6aa9c" }}>
            avant dénouage
          </p>
        </div>
      </div>
    </div>
  );
}
