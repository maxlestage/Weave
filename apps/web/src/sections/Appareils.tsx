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
            Votre prochain plan sur l'écran verrouillé, avec le compte à rebours. Quand quelqu'un
            demande à venir, la bannière apparaît d'elle-même — c'est le seul moment où Weave se
            manifeste sans qu'on l'ait ouvert. Elle n'affiche ni nom, ni photo, ni message : ce qui
            est visible sur un écran verrouillé doit pouvoir être lu par quelqu'un d'autre.
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
            Une complication sur le cadran : le prochain plan et ce qui attend une réponse. De quoi
            accepter une demande à la volée, ou dicter deux phrases avant de repartir.
          </p>
          <ul className="mt-5 space-y-2.5 text-sm" style={{ color: "var(--texte-doux)" }}>
            <li>• Complication : prochain plan et demandes à traiter</li>
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
      aria-label="Aperçu de la Live Activity : prochain plan samedi 10 h, 2 personnes veulent venir"
      style={{ background: "var(--color-encre)", color: "#f4efe7" }}
    >
      <div className="flex items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <svg width="24" height="24" viewBox="0 0 32 32" aria-hidden="true">
            <g strokeWidth="2.4" strokeLinecap="round" fill="none">
              <path d="M8 6v20" stroke="var(--color-framboise-nuit)" />
              <path d="M16 6v20" stroke="var(--color-menthe-nuit)" />
              <path d="M24 6v20" stroke="var(--color-iris-nuit)" />
            </g>
          </svg>
          <div>
            <p className="text-sm font-semibold">Marché puis brunch</p>
            <p className="text-xs" style={{ color: "#b6aa9c" }}>
              2 personnes veulent venir
            </p>
          </div>
        </div>
        <div className="text-right">
          <p
            className="text-lg font-semibold tabular-nums"
            style={{ color: "var(--color-safran-nuit)" }}
          >
            Sam. 10 h
          </p>
          <p className="text-xs" style={{ color: "#b6aa9c" }}>
            dans 2 jours
          </p>
        </div>
      </div>
    </div>
  );
}
