import {
  MAX_ACTIVE_THREADS,
  PLANS,
  PLAN_TIERS,
  UNIT_PRODUCTS,
  UNIT_SKUS,
  formatPrice,
  type CriteriaDepth,
} from "@weave/contracts";
import { useState } from "react";
import { Carte, Etiquette, Section } from "../composants.tsx";

type Periode = "mensuel" | "annuel";

/** Libellés affichables des profondeurs de critères (les valeurs sont en ASCII). */
const CRITERES: Record<CriteriaDepth, string> = {
  base: "De base",
  etendue: "Étendus",
  precise: "Précis",
};

/** Délai de regarnissage, rendu en langage courant. */
function delai(minutes: number): string {
  if (minutes >= 24 * 60) return "à la prochaine heure de tissage";
  if (minutes >= 60) return `sous ${minutes / 60} h`;
  return `sous ${minutes} min`;
}

export function Offres() {
  const [periode, setPeriode] = useState<Periode>("mensuel");

  return (
    <Section
      id="offres"
      titre="Quatre abonnements, et tout à l'unité"
      chapeau={`Aucun palier n'augmente le nombre de fils : ${MAX_ACTIVE_THREADS} pour tout le monde. Ce qui se paie, c'est la finesse des critères et la vitesse à laquelle une place libérée est regarnie.`}
    >
      <div
        className="inline-flex rounded-full p-1"
        role="group"
        aria-label="Choisir la périodicité"
        style={{ border: "1px solid var(--bordure)" }}
      >
        {(["mensuel", "annuel"] as const).map((valeur) => (
          <button
            key={valeur}
            type="button"
            aria-pressed={periode === valeur}
            onClick={() => setPeriode(valeur)}
            className="rounded-full px-4 py-2 text-sm font-semibold capitalize"
            style={
              periode === valeur
                ? { background: "var(--accent)", color: "var(--color-lin)" }
                : { color: "var(--texte-doux)" }
            }
          >
            {valeur}
          </button>
        ))}
      </div>

      <div className="mt-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {PLAN_TIERS.map((tier) => {
          const plan = PLANS[tier];
          const gratuit = plan.monthlyPriceCents === 0;
          const cents =
            periode === "annuel" && plan.yearlyPriceCents !== null
              ? plan.yearlyPriceCents
              : plan.monthlyPriceCents;
          const suffixe = gratuit ? "" : periode === "annuel" && plan.yearlyPriceCents !== null ? " / an" : " / mois";

          return (
            <Carte key={tier} accentuee={tier === "chaine"}>
              <div className="flex items-baseline justify-between gap-3">
                <h3 className="text-2xl" style={{ fontFamily: "var(--font-titre)" }}>
                  {plan.name}
                </h3>
                {tier === "chaine" && <Etiquette>Le plus choisi</Etiquette>}
              </div>

              <p className="mt-2 text-sm" style={{ color: "var(--texte-doux)" }}>
                {plan.tagline}
              </p>

              <p className="mt-5">
                <span className="text-3xl font-semibold tabular-nums">
                  {gratuit ? "Gratuit" : formatPrice(cents)}
                </span>
                <span className="text-sm" style={{ color: "var(--texte-doux)" }}>
                  {suffixe}
                </span>
              </p>

              <dl className="mt-5 space-y-1.5 text-sm">
                <div className="flex justify-between gap-3">
                  <dt style={{ color: "var(--texte-doux)" }}>Fils actifs</dt>
                  <dd className="font-semibold tabular-nums">{plan.entitlements.activeThreads}</dd>
                </div>
                <div className="flex justify-between gap-3">
                  <dt style={{ color: "var(--texte-doux)" }}>Regarnissage</dt>
                  <dd className="text-right font-semibold">
                    {delai(plan.entitlements.refillDelayMinutes)}
                  </dd>
                </div>
                <div className="flex justify-between gap-3">
                  <dt style={{ color: "var(--texte-doux)" }}>Critères</dt>
                  <dd className="font-semibold">{CRITERES[plan.entitlements.criteria]}</dd>
                </div>
              </dl>

              <ul className="mt-5 space-y-2 text-sm">
                {plan.highlights.map((point) => (
                  <li key={point} className="flex gap-2.5">
                    <span aria-hidden="true" style={{ color: "var(--accent)" }}>
                      —
                    </span>
                    <span style={{ color: "var(--texte-doux)" }}>{point}</span>
                  </li>
                ))}
              </ul>
            </Carte>
          );
        })}
      </div>

      <h3 className="mt-14 text-2xl" style={{ fontFamily: "var(--font-titre)" }}>
        Sans abonnement, à l'unité
      </h3>
      <p className="mt-3 max-w-2xl leading-relaxed" style={{ color: "var(--texte-doux)" }}>
        Chaque avantage d'un abonnement s'achète aussi séparément. On peut utiliser Weave des mois
        durant sans jamais s'abonner.
      </p>

      <ul className="mt-6 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {UNIT_SKUS.map((sku) => {
          const produit = UNIT_PRODUCTS[sku];
          return (
            <li
              key={sku}
              className="rounded-xl p-4"
              style={{ background: "var(--carte)", border: "1px solid var(--bordure)" }}
            >
              <div className="flex items-baseline justify-between gap-3">
                <span className="font-semibold">{produit.name}</span>
                <span className="text-sm font-semibold tabular-nums" style={{ color: "var(--accent)" }}>
                  {formatPrice(produit.priceCents)}
                </span>
              </div>
              <p className="mt-2 text-sm leading-relaxed" style={{ color: "var(--texte-doux)" }}>
                {produit.description}
              </p>
            </li>
          );
        })}
      </ul>

      <p className="mt-8 text-sm" style={{ color: "var(--texte-doux)" }}>
        Les abonnements se souscrivent et se résilient depuis les réglages de votre compte Apple.
        Aucun moyen de paiement ne transite par nos serveurs.
      </p>
    </Section>
  );
}
