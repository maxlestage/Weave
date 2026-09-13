import { MAX_OPEN_PLANS, MIN_AGE, PLAN_CATEGORY_LABELS_PAR_LANGUE } from "@weave/contracts";
import { Etiquette, filVar, type Fil } from "../composants.tsx";
import { useTraduction } from "../contexte-langue.tsx";
import type { Traduit } from "../langues.ts";

const TEXTES: Traduit<{
  bientot: string;
  titre: readonly string[];
  accroche: string;
  offres: string;
  principe: string;
  mentions: (age: number) => string;
  illustration: string;
  tri: (plans: number) => string;
}> = {
  fr: {
    bientot: "Bientôt sur iPhone et Apple Watch",
    titre: ["Des", "plans,", "pas", "des", "profils."],
    accroche:
      "Sur Weave, on ne se décrit pas : on écrit ce qu'on compte faire jeudi soir. Les autres demandent à venir — en disant pourquoi. Pas de cartes à balayer, pas de « il/elle vous a remarqué », pas de file d'attente.",
    offres: "Voir les offres",
    principe: "Comprendre le principe",
    mentions: (age) => `Gratuit pour commencer · Sans publicité · Réservé aux ${age} ans et plus`,
    illustration: "Illustration du fil : trois plans à venir",
    tri: (plans) =>
      `Le fil est trié par ce qui arrive le plus tôt, puis par ce qui est le plus près. Rien d'autre : aucun abonnement ne fait remonter un plan. Et vous publiez au plus ${plans} plans à la fois.`,
  },
  en: {
    bientot: "Coming soon to iPhone and Apple Watch",
    titre: ["Plans,", "not", "profiles."],
    accroche:
      "On Weave you don't describe yourself: you write down what you're doing on Thursday evening. Other people ask to come — and say why. No cards to swipe, no “someone noticed you”, no queue.",
    offres: "See the plans",
    principe: "How it works",
    mentions: (age) => `Free to start · No advertising · ${age} and over only`,
    illustration: "Illustration of the feed: three upcoming plans",
    tri: (plans) =>
      `The feed is sorted by what happens soonest, then by what is nearest. Nothing else: no subscription pushes a plan up. And you post at most ${plans} plans at a time.`,
  },
  es: {
    bientot: "Pronto en iPhone y Apple Watch",
    titre: ["Planes,", "no", "perfiles."],
    accroche:
      "En Weave no te describes: escribes lo que piensas hacer el jueves por la noche. Los demás piden venir — y dicen por qué. Sin tarjetas que deslizar, sin « alguien se ha fijado en ti », sin cola de espera.",
    offres: "Ver las suscripciones",
    principe: "Cómo funciona",
    mentions: (age) => `Gratis para empezar · Sin publicidad · Solo mayores de ${age} años`,
    illustration: "Ilustración del muro: tres planes próximos",
    tri: (plans) =>
      `El muro se ordena por lo que ocurre antes, y luego por lo que está más cerca. Nada más: ninguna suscripción sube un plan. Y publicas como máximo ${plans} planes a la vez.`,
  },
};

/** Les couleurs des mots du titre, dans l'ordre. */
const FILS_DU_TITRE: readonly Fil[] = [1, 2, 4, 5, 6];

export function Ouverture() {
  const t = useTraduction(TEXTES);

  return (
    <section
      id="haut"
      className="relative overflow-hidden px-5 pt-12 pb-16 sm:px-8 sm:pt-20 sm:pb-24"
    >
      <div className="trame pointer-events-none absolute inset-0 opacity-40" aria-hidden="true" />

      <div className="relative mx-auto w-full max-w-5xl">
        <Etiquette fil={6}>{t.bientot}</Etiquette>

        {/*
          Un fil par mot, plutôt qu'un dégradé : le dégradé coupait les lettres
          au milieu et la couleur paraissait accidentelle. Là, elle est voulue.
        */}
        <h1
          className="mt-6 text-[2.3rem] leading-[1.05] font-bold tracking-tight sm:text-6xl lg:text-7xl"
          style={{ fontFamily: "var(--font-titre)" }}
        >
          {t.titre.map((mot, index) => (
            <span key={mot} style={{ color: filVar(FILS_DU_TITRE[index % FILS_DU_TITRE.length]!) }}>
              {index > 0 ? " " : ""}
              {mot}
            </span>
          ))}
        </h1>

        <p className="mt-6 max-w-xl text-lg leading-relaxed sm:text-xl">{t.accroche}</p>

        <div className="mt-9 flex flex-col gap-3 sm:flex-row sm:items-center">
          <a
            href="#offres"
            className="inline-flex items-center justify-center rounded-full px-7 py-4 text-base font-bold"
            style={{ background: "var(--accent)", color: "var(--sur-accent)" }}
          >
            {t.offres}
          </a>
          <a
            href="#principe"
            className="inline-flex items-center justify-center rounded-full px-7 py-4 text-base font-bold"
            style={{ border: `2px solid ${filVar(4)}`, color: "var(--texte)" }}
          >
            {t.principe}
          </a>
        </div>

        <p className="mt-5 text-sm font-medium" style={{ color: "var(--texte-doux)" }}>
          {t.mentions(MIN_AGE)}
        </p>

        <Fil />
      </div>
    </section>
  );
}

/** Les trois plans de l'illustration, déclinés. */
const PLANS_ILLUSTRES: Traduit<
  readonly {
    titre: string;
    note: string;
    auteur: string;
    quand: string;
    ou: string;
    places: string;
    categorie: keyof (typeof PLAN_CATEGORY_LABELS_PAR_LANGUE)["fr"];
    fil: Fil;
  }[]
> = {
  fr: [
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
  ],
  en: [
    {
      titre: "Bouldering at the 7pm wall, beginners welcome",
      note: "I've been climbing for six months, very badly.",
      auteur: "Théo, 23",
      quand: "Thursday 7pm",
      ou: "2 km away",
      places: "1 spot",
      categorie: "sport",
      fil: 1,
    },
    {
      titre: "A gig by a band nobody has heard of",
      note: "Small venue, £8 on the door.",
      auteur: "Sofia, 21",
      quand: "Friday 8.30pm",
      ou: "4 km away",
      places: "2 spots",
      categorie: "musique",
      fil: 4,
    },
    {
      titre: "Market, then a slow brunch",
      note: "Come along if you like tasting ten things before buying one.",
      auteur: "Alex, 24",
      quand: "Saturday 10am",
      ou: "1 km away",
      places: "2 spots",
      categorie: "repas",
      fil: 5,
    },
  ],
  es: [
    {
      titre: "Bloque en el muro de las 19 h, nivel principiante",
      note: "Llevo seis meses escalando, fatal.",
      auteur: "Théo, 23 años",
      quand: "Jueves 19 h",
      ou: "a 2 km",
      places: "1 plaza",
      categorie: "sport",
      fil: 1,
    },
    {
      titre: "Concierto de un grupo que no conoce nadie",
      note: "Sala pequeña, 8 € en la puerta.",
      auteur: "Sofia, 21 años",
      quand: "Viernes 20.30 h",
      ou: "a 4 km",
      places: "2 plazas",
      categorie: "musique",
      fil: 4,
    },
    {
      titre: "Mercado y luego brunch, sin prisa",
      note: "Ven si te gusta probar diez cosas antes de comprar una.",
      auteur: "Alex, 24 años",
      quand: "Sábado 10 h",
      ou: "a 1 km",
      places: "2 plazas",
      categorie: "repas",
      fil: 5,
    },
  ],
};

/**
 * Représentation du fil : trois plans à venir, tels qu'on les verrait.
 * C'est une illustration, pas une capture — aucune donnée réelle n'y figure.
 */
function Fil() {
  const t = useTraduction(TEXTES);
  const plans = useTraduction(PLANS_ILLUSTRES);
  const categories = useTraduction(PLAN_CATEGORY_LABELS_PAR_LANGUE);

  return (
    <div className="mt-14" aria-label={t.illustration} role="img">
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
                  {categories[plan.categorie]}
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
        {t.tri(MAX_OPEN_PLANS)}
      </p>
    </div>
  );
}
