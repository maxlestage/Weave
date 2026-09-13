import { Carte, filVar, Section } from "../composants.tsx";
import { useTraduction } from "../contexte-langue.tsx";
import type { Traduit } from "../langues.ts";

const TEXTES: Traduit<{
  titre: string;
  chapeau: string;
  activiteTexte: string;
  montre: string;
  montreTexte: string;
  montrePoints: readonly string[];
  fin: string;
  apercu: string;
  plan: string;
  demandes: string;
  quand: string;
  dans: string;
}> = {
  fr: {
    titre: "Sur l'iPhone, et au poignet",
    chapeau:
      "L'application est écrite en Swift, nativement. La Live Activity et l'application Apple Watch ne sont pas des extras : elles servent précisément à ne pas ouvrir l'application.",
    activiteTexte:
      "Votre prochain plan sur l'écran verrouillé, avec le compte à rebours. Quand quelqu'un demande à venir, la bannière apparaît d'elle-même — c'est le seul moment où Weave se manifeste sans qu'on l'ait ouvert. Elle n'affiche ni nom, ni photo, ni message : ce qui est visible sur un écran verrouillé doit pouvoir être lu par quelqu'un d'autre.",
    montre: "Apple Watch",
    montreTexte:
      "Une complication sur le cadran : le prochain plan et ce qui attend une réponse. De quoi accepter une demande à la volée, ou dicter deux phrases avant de repartir.",
    montrePoints: [
      "• Complication : prochain plan et demandes à traiter",
      "• Réponse par dictée ou saisie manuscrite",
      "• Synchronisation par WatchConnectivity, et repli sur le réseau",
    ],
    fin: "Ce site n'est qu'une présentation : Weave se vit dans l'application.",
    apercu: "Aperçu de la Live Activity : prochain plan samedi 10 h, 2 personnes veulent venir",
    plan: "Marché puis brunch",
    demandes: "2 personnes veulent venir",
    quand: "Sam. 10 h",
    dans: "dans 2 jours",
  },
  en: {
    titre: "On the iPhone, and on your wrist",
    chapeau:
      "The app is written in Swift, natively. The Live Activity and the Apple Watch app aren't extras: their whole point is to save you from opening the app.",
    activiteTexte:
      "Your next plan on the lock screen, with the countdown. When someone asks to come, the banner appears on its own — it's the only moment Weave speaks up without being opened. It shows no name, no photo, no message: whatever is visible on a lock screen has to be safe for someone else to read.",
    montre: "Apple Watch",
    montreTexte:
      "A complication on the watch face: your next plan and whatever is waiting for an answer. Enough to accept a request in passing, or dictate two sentences before moving on.",
    montrePoints: [
      "• Complication: next plan and requests to handle",
      "• Reply by dictation or handwriting",
      "• Sync over WatchConnectivity, falling back to the network",
    ],
    fin: "This site is only an introduction: Weave happens in the app.",
    apercu: "Preview of the Live Activity: next plan Saturday 10am, 2 people want to come",
    plan: "Market, then brunch",
    demandes: "2 people want to come",
    quand: "Sat 10am",
    dans: "in 2 days",
  },
  es: {
    titre: "En el iPhone, y en la muñeca",
    chapeau:
      "La aplicación está escrita en Swift, de forma nativa. La Live Activity y la aplicación para Apple Watch no son extras: sirven precisamente para no abrir la aplicación.",
    activiteTexte:
      "Tu próximo plan en la pantalla bloqueada, con la cuenta atrás. Cuando alguien pide venir, el aviso aparece solo — es el único momento en que Weave se manifiesta sin haberla abierto. No muestra ni nombre, ni foto, ni mensaje: lo que se ve en una pantalla bloqueada tiene que poder leerlo otra persona.",
    montre: "Apple Watch",
    montreTexte:
      "Una complicación en la esfera: el próximo plan y lo que espera respuesta. Lo justo para aceptar una petición al vuelo, o dictar dos frases antes de seguir.",
    montrePoints: [
      "• Complicación: próximo plan y peticiones pendientes",
      "• Respuesta por dictado o escritura a mano",
      "• Sincronización por WatchConnectivity, con respaldo por red",
    ],
    fin: "Este sitio es solo una presentación: Weave se vive en la aplicación.",
    apercu:
      "Vista previa de la Live Activity: próximo plan el sábado a las 10 h, 2 personas quieren venir",
    plan: "Mercado y luego brunch",
    demandes: "2 personas quieren venir",
    quand: "Sáb. 10 h",
    dans: "en 2 días",
  },
};

export function Appareils() {
  const t = useTraduction(TEXTES);

  return (
    <Section id="montre" fil={5} titre={t.titre} chapeau={t.chapeau} alterne>
      <div className="grid gap-4 lg:grid-cols-2">
        <Carte fil={5}>
          <h3
            className="text-xl font-bold"
            style={{ fontFamily: "var(--font-titre)", color: filVar(5) }}
          >
            Live Activity
          </h3>
          <p className="mt-3 leading-relaxed" style={{ color: "var(--texte-doux)" }}>
            {t.activiteTexte}
          </p>
          <ApercuActivite />
        </Carte>

        <Carte fil={4}>
          <h3
            className="text-xl font-bold"
            style={{ fontFamily: "var(--font-titre)", color: filVar(4) }}
          >
            {t.montre}
          </h3>
          <p className="mt-3 leading-relaxed" style={{ color: "var(--texte-doux)" }}>
            {t.montreTexte}
          </p>
          <ul className="mt-5 space-y-2.5 text-sm" style={{ color: "var(--texte-doux)" }}>
            {t.montrePoints.map((point) => (
              <li key={point}>{point}</li>
            ))}
          </ul>
        </Carte>
      </div>

      <p className="mt-8 text-sm" style={{ color: "var(--texte-doux)" }}>
        {t.fin}
      </p>
    </Section>
  );
}

/** Maquette statique de la bannière : illustration, pas capture d'écran. */
function ApercuActivite() {
  const t = useTraduction(TEXTES);

  return (
    <div
      className="mt-6 rounded-2xl p-4"
      role="img"
      aria-label={t.apercu}
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
            <p className="text-sm font-semibold">{t.plan}</p>
            <p className="text-xs" style={{ color: "#b6aa9c" }}>
              {t.demandes}
            </p>
          </div>
        </div>
        <div className="text-right">
          <p
            className="text-lg font-semibold tabular-nums"
            style={{ color: "var(--color-safran-nuit)" }}
          >
            {t.quand}
          </p>
          <p className="text-xs" style={{ color: "#b6aa9c" }}>
            {t.dans}
          </p>
        </div>
      </div>
    </div>
  );
}
