import { MAX_ACTIVE_THREADS, MIN_AGE } from "@weave/contracts";
import { filVar, Section } from "../composants.tsx";

const QUESTIONS = [
  {
    q: "À qui s'adresse Weave ?",
    r: `Aux jeunes adultes, d'abord : les questions, le vocabulaire et le rythme sont écrits pour ce moment de la vie où l'on change de ville, de travail et de cercle d'amis. L'accès est réservé aux ${MIN_AGE} ans et plus, sans exception — c'est une application de rencontre, elle n'a rien à faire entre les mains de mineurs.`,
  },
  {
    q: `Pourquoi seulement ${MAX_ACTIVE_THREADS} profils ?`,
    r: `Parce qu'un nombre fini change tout. ${MAX_ACTIVE_THREADS} fils, on en voit le bout : on peut lire chaque trame et répondre à celles qui comptent. Une pile sans fin, on la fait défiler — ce n'est plus la même activité, et ce n'est plus la même façon d'écrire.`,
  },
  {
    q: "Puis-je en avoir plus en payant ?",
    r: `Non, et c'est délibéré. ${MAX_ACTIVE_THREADS} pour tout le monde, du gratuit au plus cher. Les abonnements accélèrent le remplacement d'un fil dénoué, ils n'élargissent jamais le métier : vendre du volume reviendrait à défaire le produit.`,
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
    <Section id="questions" fil={3} titre="Questions fréquentes">
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
                  stroke={filVar(3)}
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
