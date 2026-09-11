import { MAX_OPEN_PLANS, MIN_AGE, REQUESTS_PER_DAY_FLOOR } from "@weave/contracts";
import { filVar, Section } from "../composants.tsx";

const QUESTIONS = [
  {
    q: "À qui s'adresse Weave ?",
    r: `Aux jeunes adultes, d'abord : le vocabulaire, les prix et le rythme sont écrits pour ce moment de la vie où l'on change de ville, de travail et de cercle d'amis, et où l'on se retrouve à ne connaître personne un vendredi soir. L'accès est réservé aux ${MIN_AGE} ans et plus, sans exception.`,
  },
  {
    q: "Ce n'est pas une application de rencontre, alors ?",
    r: "Si. Simplement, on ne commence pas par se décrire : on commence par proposer quelque chose à faire. Ce qui se passe ensuite ne regarde que les deux personnes concernées — amitié, relation, ou rien du tout.",
  },
  {
    q: "Pourquoi un nombre limité de demandes par jour ?",
    r: `Parce que sans limite, demander ne veut plus rien dire : on envoie le même message à trente personnes et on trie les réponses. Avec un quota — au minimum ${REQUESTS_PER_DAY_FLOOR} par jour, même sans payer — on choisit les plans auxquels on veut vraiment aller, et on écrit quelque chose qui tient debout.`,
  },
  {
    q: "Je peux payer pour que mon plan passe devant ?",
    r: "Non, et c'est la règle qui ne bougera pas. Le fil est trié par ce qui arrive le plus tôt, puis par ce qui est le plus près. Aucun abonnement, aucun achat n'intervient dans cet ordre. Ce qui se paie, c'est l'horizon de publication, la finesse des critères et les plans de groupe.",
  },
  {
    q: "Que se passe-t-il si ma demande est refusée ?",
    r: "Elle se ferme, sans motif et sans notification accusatrice. Si vous la retirez avant qu'elle ait été lue, elle vous est rendue — se raviser vite ne doit pas coûter la journée.",
  },
  {
    q: "Combien de plans puis-je publier ?",
    r: `${MAX_OPEN_PLANS} ouverts à la fois. Ce n'est pas une brimade : au-delà, ce ne sont plus des plans, c'est une annonce permanente. Quand une date passe, la place se libère.`,
  },
  {
    q: "Et si personne ne demande à venir ?",
    r: "Le plan passe et disparaît. Vous faites ce que vous aviez prévu — c'était l'idée de départ. Rien ne vous reproche un plan sans réponse, et aucun compteur ne vous le rappelle.",
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
