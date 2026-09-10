import { MAX_ACTIVE_THREADS } from "@weave/contracts";
import { Section } from "../composants.tsx";

const QUESTIONS = [
  {
    q: `Pourquoi seulement ${MAX_ACTIVE_THREADS} profils ?`,
    r: "Parce qu'au-delà, on ne lit plus. Trois fils tiennent dans une soirée : on peut vraiment répondre à chacun, ce qui change la nature de ce qu'on écrit.",
  },
  {
    q: "Puis-je en avoir plus en payant ?",
    r: "Non, et c'est délibéré. Les abonnements accélèrent le remplacement d'un fil dénoué, ils n'élargissent jamais le métier. Vendre du volume reviendrait à défaire le produit.",
  },
  {
    q: "Que se passe-t-il si je ne réponds pas ?",
    r: "Le fil se dénoue au bout de vingt-quatre heures et disparaît. L'autre personne ne reçoit pas de refus : elle voit simplement le fil s'éteindre.",
  },
  {
    q: "Pourquoi la photo est-elle floue ?",
    r: "Parce qu'elle prend toute la place quand elle arrive en premier. Chez Weave, elle se dévoile à mesure que la conversation avance — après trois échanges, elle est nette.",
  },
  {
    q: "Puis-je récupérer un fil que j'ai laissé passer ?",
    r: "Une fois, avec un « Écho ». C'est inclus dans les abonnements à partir de Trame, et achetable à l'unité. Au-delà, non : le temps a fait son travail.",
  },
  {
    q: "Y a-t-il une version Android ou web ?",
    r: "Pas au lancement. Weave commence sur iPhone et Apple Watch, en Swift natif ; ce site présente l'application, il ne la remplace pas.",
  },
] as const;

export function Questions() {
  return (
    <Section id="questions" titre="Questions fréquentes">
      <div className="divide-y" style={{ borderColor: "var(--bordure)" }}>
        {QUESTIONS.map((item) => (
          <details key={item.q} className="group py-4" style={{ borderColor: "var(--bordure)" }}>
            <summary className="flex cursor-pointer list-none items-center justify-between gap-4 text-lg font-medium">
              {item.q}
              <svg
                width="20"
                height="20"
                viewBox="0 0 20 20"
                className="shrink-0 transition-transform group-open:rotate-45"
                aria-hidden="true"
              >
                <path
                  d="M10 4v12M4 10h12"
                  stroke="var(--accent)"
                  strokeWidth="1.8"
                  strokeLinecap="round"
                />
              </svg>
            </summary>
            <p className="mt-3 max-w-2xl leading-relaxed" style={{ color: "var(--texte-doux)" }}>
              {item.r}
            </p>
          </details>
        ))}
      </div>
    </Section>
  );
}
