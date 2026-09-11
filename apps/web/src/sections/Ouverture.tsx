import { MAX_ACTIVE_THREADS, MIN_AGE, THREAD_TTL_SECONDS } from "@weave/contracts";
import { Etiquette, filVar, type Fil } from "../composants.tsx";

const HEURES = THREAD_TTL_SECONDS / 3600;

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
          <span style={{ color: filVar(1) }}>{MAX_ACTIVE_THREADS}</span>{" "}
          <span style={{ color: filVar(2) }}>fils</span>{" "}
          <span style={{ color: filVar(4) }}>par</span>{" "}
          <span style={{ color: filVar(5) }}>jour.</span>
          <br />
          Pas un de plus.
        </h1>

        <p className="mt-6 max-w-xl text-lg leading-relaxed sm:text-xl">
          Weave ne vous donne pas une pile sans fin à faire défiler. Il vous en propose{" "}
          {MAX_ACTIVE_THREADS}, à l'heure que vous avez choisie, et s'arrête là. Ils vivent {HEURES}{" "}
          heures, puis se dénouent. Pour en engager un, il faut écrire une réponse — pas faire un
          geste.
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

        <Metier />
      </div>
    </section>
  );
}

/**
 * Représentation du métier : trois fils parmi ceux du jour, dont un déjà engagé.
 * C'est une illustration, pas une capture — aucune donnée réelle n'y figure.
 */
function Metier() {
  const fils: { nom: string; motif: string; reste: string; etat: string; fil: Fil }[] = [
    {
      nom: "Théo",
      motif: "escalade · vinyles · ciné-club",
      reste: "18 h",
      etat: "à vous de répondre",
      fil: 1,
    },
    {
      nom: "Sofia",
      motif: "jazz · céramique · vélo",
      reste: "9 h",
      etat: "engagé",
      fil: 4,
    },
    {
      nom: "Alex",
      motif: "impro · rando · jeux de société",
      reste: "23 h",
      etat: "à vous de répondre",
      fil: 5,
    },
  ];

  return (
    <div
      className="mt-14"
      aria-label={`Illustration du métier : trois fils parmi les ${MAX_ACTIVE_THREADS} du jour`}
      role="img"
    >
      <div className="grid gap-4 sm:grid-cols-3">
        {fils.map((fil) => (
          <article
            key={fil.nom}
            className="overflow-hidden rounded-3xl"
            style={{ background: "var(--carte)", border: `2px solid ${filVar(fil.fil)}` }}
          >
            <div className="h-2" style={{ background: filVar(fil.fil) }} aria-hidden="true" />

            <div className="p-5">
              <div className="flex items-baseline justify-between gap-3">
                <span className="text-lg font-bold">{fil.nom}</span>
                <span
                  className="shrink-0 text-sm font-bold tabular-nums"
                  style={{ color: filVar(fil.fil) }}
                >
                  {fil.reste}
                </span>
              </div>

              <p className="mt-2 text-sm" style={{ color: "var(--texte-doux)" }}>
                {fil.motif}
              </p>

              <div
                className="mt-4 h-16 rounded-xl"
                aria-hidden="true"
                style={{
                  background: `linear-gradient(120deg, color-mix(in oklab, ${filVar(fil.fil)} 55%, transparent), color-mix(in oklab, ${filVar((((fil.fil + 2) % 6) + 1) as Fil)} 45%, transparent))`,
                  filter: "blur(7px)",
                }}
              />

              <p
                className="mt-4 text-xs font-bold tracking-wide uppercase"
                style={{ color: "var(--texte-doux)" }}
              >
                {fil.etat}
              </p>
            </div>
          </article>
        ))}
      </div>

      <p className="mt-4 text-sm" style={{ color: "var(--texte-doux)" }}>
        Trois de vos {MAX_ACTIVE_THREADS} fils du jour. La photo reste floue au premier contact ;
        elle se dévoile au fil des échanges.
      </p>
    </div>
  );
}
