import { MAX_OPEN_PLANS, MIN_AGE, PLAN_CATEGORY_LABELS } from "@weave/contracts";
import { Etiquette, filVar, type Fil } from "../composants.tsx";

export function Ouverture() {
  return (
    <section
      id="haut"
      className="relative overflow-hidden px-5 pt-12 pb-16 sm:px-8 sm:pt-20 sm:pb-24"
    >
      <div className="trame pointer-events-none absolute inset-0 opacity-40" aria-hidden="true" />

      <div className="relative mx-auto w-full max-w-5xl">
        <Etiquette fil={6}>Bientôt sur iPhone et Apple Watch</Etiquette>

        {/*
          Un fil par mot, plutôt qu'un dégradé : le dégradé coupait les lettres
          au milieu et la couleur paraissait accidentelle. Là, elle est voulue.
        */}
        <h1
          className="mt-6 text-[2.3rem] leading-[1.05] font-bold tracking-tight sm:text-6xl lg:text-7xl"
          style={{ fontFamily: "var(--font-titre)" }}
        >
          <span style={{ color: filVar(1) }}>Des</span>{" "}
          <span style={{ color: filVar(2) }}>plans,</span>{" "}
          <span style={{ color: filVar(4) }}>pas</span>{" "}
          <span style={{ color: filVar(5) }}>des</span>{" "}
          <span style={{ color: filVar(6) }}>profils.</span>
        </h1>

        <p className="mt-6 max-w-xl text-lg leading-relaxed sm:text-xl">
          Sur Weave, on ne se décrit pas : on écrit ce qu'on compte faire jeudi soir. Les autres
          demandent à venir — en disant pourquoi. Pas de cartes à balayer, pas de « il/elle vous a
          remarqué », pas de file d'attente.
        </p>

        <div className="mt-9 flex flex-col gap-3 sm:flex-row sm:items-center">
          <a
            href="#offres"
            className="inline-flex items-center justify-center rounded-full px-7 py-4 text-base font-bold"
            style={{ background: "var(--accent)", color: "var(--sur-accent)" }}
          >
            Voir les offres
          </a>
          <a
            href="#principe"
            className="inline-flex items-center justify-center rounded-full px-7 py-4 text-base font-bold"
            style={{ border: `2px solid ${filVar(4)}`, color: "var(--texte)" }}
          >
            Comprendre le principe
          </a>
        </div>

        <p className="mt-5 text-sm font-medium" style={{ color: "var(--texte-doux)" }}>
          Gratuit pour commencer · Sans publicité · Réservé aux {MIN_AGE} ans et plus
        </p>

        <Fil />
      </div>
    </section>
  );
}

/**
 * Représentation du fil : trois plans à venir, tels qu'on les verrait.
 * C'est une illustration, pas une capture — aucune donnée réelle n'y figure.
 */
function Fil() {
  const plans: {
    titre: string;
    note: string;
    auteur: string;
    quand: string;
    ou: string;
    places: string;
    categorie: keyof typeof PLAN_CATEGORY_LABELS;
    fil: Fil;
  }[] = [
    {
      titre: "Bloc au mur de 19 h, niveau débutant",
      note: "Je grimpe depuis six mois, très mal.",
      auteur: "Théo, 23 ans",
      quand: "Jeudi 19 h",
      ou: "à 2 km",
      places: "1 place",
      categorie: "sport",
      fil: 1,
    },
    {
      titre: "Concert d'un groupe que personne ne connaît",
      note: "Petite salle, 8 € à l'entrée.",
      auteur: "Sofia, 21 ans",
      quand: "Vendredi 20 h 30",
      ou: "à 4 km",
      places: "2 places",
      categorie: "musique",
      fil: 4,
    },
    {
      titre: "Marché puis brunch, sans se presser",
      note: "Venez si vous aimez goûter dix choses avant d'acheter.",
      auteur: "Alex, 24 ans",
      quand: "Samedi 10 h",
      ou: "à 1 km",
      places: "2 places",
      categorie: "repas",
      fil: 5,
    },
  ];

  return (
    <div className="mt-14" aria-label="Illustration du fil : trois plans à venir" role="img">
      <div className="grid gap-4 sm:grid-cols-3">
        {plans.map((plan) => (
          <article
            key={plan.titre}
            className="flex flex-col overflow-hidden rounded-3xl"
            style={{ background: "var(--carte)", border: `2px solid ${filVar(plan.fil)}` }}
          >
            <div className="h-2" style={{ background: filVar(plan.fil) }} aria-hidden="true" />

            <div className="flex flex-1 flex-col p-5">
              <div className="flex items-baseline justify-between gap-3">
                <span
                  className="text-xs font-bold tracking-wide uppercase"
                  style={{ color: filVar(plan.fil) }}
                >
                  {PLAN_CATEGORY_LABELS[plan.categorie]}
                </span>
                <span
                  className="shrink-0 text-sm font-bold tabular-nums"
                  style={{ color: "var(--texte-doux)" }}
                >
                  {plan.ou}
                </span>
              </div>

              <h3 className="mt-2 text-base leading-snug font-bold">{plan.titre}</h3>

              <p className="mt-2 flex-1 text-sm" style={{ color: "var(--texte-doux)" }}>
                {plan.note}
              </p>

              <p className="mt-4 text-sm font-bold" style={{ color: filVar(plan.fil) }}>
                {plan.quand} · {plan.places}
              </p>
              <p className="mt-1 text-sm" style={{ color: "var(--texte-doux)" }}>
                {plan.auteur}
              </p>
            </div>
          </article>
        ))}
      </div>

      <p className="mt-4 text-sm" style={{ color: "var(--texte-doux)" }}>
        Le fil est trié par ce qui arrive le plus tôt, puis par ce qui est le plus près. Rien
        d'autre : aucun abonnement ne fait remonter un plan. Et vous publiez au plus{" "}
        {MAX_OPEN_PLANS} plans à la fois.
      </p>
    </div>
  );
}
