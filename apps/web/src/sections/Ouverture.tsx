import { MAX_ACTIVE_THREADS, THREAD_TTL_SECONDS } from "@weave/contracts";
import { Etiquette } from "../composants.tsx";

const HEURES = THREAD_TTL_SECONDS / 3600;

export function Ouverture() {
  return (
    <section id="haut" className="relative overflow-hidden px-5 pt-14 pb-16 sm:px-8 sm:pt-24 sm:pb-24">
      <div className="trame pointer-events-none absolute inset-0 opacity-60" aria-hidden="true" />

      <div className="relative mx-auto w-full max-w-5xl">
        <Etiquette>Bientôt sur iPhone et Apple Watch</Etiquette>

        <h1
          className="mt-6 text-[2.6rem] leading-[1.05] tracking-tight sm:text-6xl"
          style={{ fontFamily: "var(--font-titre)" }}
        >
          Trois fils par jour.
          <br />
          <span style={{ color: "var(--accent)" }}>Rien de plus.</span>
        </h1>

        <p className="mt-6 max-w-xl text-lg leading-relaxed sm:text-xl" style={{ color: "var(--texte-doux)" }}>
          Weave ne vous donne pas une pile de profils à faire défiler. Il vous en propose{" "}
          {MAX_ACTIVE_THREADS}, à l'heure que vous avez choisie. Ils vivent {HEURES} heures, puis se
          dénouent. Pour en engager un, il faut écrire une réponse — pas faire un geste.
        </p>

        <div className="mt-9 flex flex-col gap-3 sm:flex-row sm:items-center">
          <a
            href="#offres"
            className="inline-flex items-center justify-center rounded-full px-6 py-3.5 text-base font-semibold"
            style={{ background: "var(--accent)", color: "var(--color-lin)" }}
          >
            Voir les offres
          </a>
          <a
            href="#principe"
            className="inline-flex items-center justify-center rounded-full px-6 py-3.5 text-base font-semibold"
            style={{ border: "1px solid var(--bordure)", color: "var(--texte)" }}
          >
            Comprendre le principe
          </a>
        </div>

        <p className="mt-5 text-sm" style={{ color: "var(--texte-doux)" }}>
          Gratuit pour commencer. Sans publicité, et sans vendre vos données.
        </p>

        <Metier />
      </div>
    </section>
  );
}

/**
 * Représentation du métier à tisser : trois fils, dont un déjà engagé.
 * C'est une illustration, pas une capture — aucune donnée réelle n'y figure.
 */
function Metier() {
  const fils = [
    { nom: "Théo", motif: "gravure · voile · photo", reste: "18 h", etat: "à vous de répondre" },
    { nom: "Sofia", motif: "jazz · céramique · vélo", reste: "9 h", etat: "engagé" },
    { nom: "Alex", motif: "théâtre · botanique · course", reste: "23 h", etat: "à vous de répondre" },
  ];

  return (
    <div className="mt-14" aria-label="Illustration du métier : trois fils actifs" role="img">
      <div className="grid gap-3 sm:grid-cols-3">
        {fils.map((fil) => (
          <article
            key={fil.nom}
            className="rounded-2xl p-5"
            style={{ background: "var(--carte)", border: "1px solid var(--bordure)" }}
          >
            <div className="flex items-baseline justify-between gap-3">
              <span className="text-lg font-semibold">{fil.nom}</span>
              <span
                className="shrink-0 text-xs font-medium tabular-nums"
                style={{ color: "var(--accent)" }}
              >
                {fil.reste}
              </span>
            </div>
            <p className="mt-2 text-sm" style={{ color: "var(--texte-doux)" }}>
              {fil.motif}
            </p>
            <div
              className="mt-4 h-14 rounded-lg"
              aria-hidden="true"
              style={{
                background:
                  "linear-gradient(120deg, color-mix(in oklab, var(--accent) 22%, transparent), color-mix(in oklab, var(--texte) 10%, transparent))",
                filter: "blur(6px)",
              }}
            />
            <p className="mt-4 text-xs uppercase tracking-wide" style={{ color: "var(--texte-doux)" }}>
              {fil.etat}
            </p>
          </article>
        ))}
      </div>
      <p className="mt-4 text-sm" style={{ color: "var(--texte-doux)" }}>
        La photo reste floue au premier contact. Elle se dévoile au fil des échanges.
      </p>
    </div>
  );
}
