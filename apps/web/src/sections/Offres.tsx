import {
  PLAN_TIERS,
  REQUESTS_PER_DAY_FLOOR,
  TIERS,
  UNIT_PRODUCTS,
  UNIT_SKUS,
  formatPrice,
  type FilterDepth,
} from "@weave/contracts";
import { useState } from "react";
import { Carte, Etiquette, filVar, Section, type Fil } from "../composants.tsx";

type Periode = "mensuel" | "annuel";

/** Chaque palier porte sa couleur. */
const COULEURS: Record<string, Fil> = {
  depart: 4,
  viree: 3,
  escapade: 6,
  expedition: 5,
  grandtour: 1,
};

/** Libellés affichables de la finesse des critères (les valeurs sont en ASCII). */
const CRITERES: Record<FilterDepth, string> = {
  base: "De base",
  etendus: "Étendus",
  precis: "Précis",
};

/** Horizon de publication, rendu en langage courant. */
function horizon(jours: number): string {
  if (jours >= 30) return "un mois à l'avance";
  if (jours >= 7) return `${jours / 7} semaine${jours >= 14 ? "s" : ""} à l'avance`;
  return `${jours} jours à l'avance`;
}

export function Offres() {
  const [periode, setPeriode] = useState<Periode>("mensuel");

  return (
    <Section
      id="offres"
      fil={6}
      titre="Quatre abonnements, et tout à l'unité"
      chapeau={`Aucune offre n'achète de visibilité : payer ne fait jamais remonter un plan. Ce qui se paie, c'est l'horizon de publication, la finesse des critères et les plans de groupe. Le nombre de demandes reste borné partout — au minimum ${REQUESTS_PER_DAY_FLOOR} par jour.`}
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
                ? { background: "var(--accent)", color: "var(--sur-accent)" }
                : { color: "var(--texte-doux)" }
            }
          >
            {valeur}
          </button>
        ))}
      </div>

      <div className="mt-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {PLAN_TIERS.map((tier) => {
          const offre = TIERS[tier];
          const gratuit = offre.monthlyPriceCents === 0;
          const cents =
            periode === "annuel" && offre.yearlyPriceCents !== null
              ? offre.yearlyPriceCents
              : offre.monthlyPriceCents;
          const suffixe = gratuit
            ? ""
            : periode === "annuel" && offre.yearlyPriceCents !== null
              ? " / an"
              : " / mois";

          const couleur = COULEURS[tier] ?? 6;

          return (
            <Carte key={tier} fil={couleur} accentuee={tier === "escapade"}>
              <div className="flex items-baseline justify-between gap-3">
                <h3
                  className="text-2xl font-bold"
                  style={{ fontFamily: "var(--font-titre)", color: filVar(couleur) }}
                >
                  {offre.name}
                </h3>
                {tier === "escapade" && <Etiquette fil={couleur}>Le plus choisi</Etiquette>}
              </div>

              <p className="mt-2 text-sm" style={{ color: "var(--texte-doux)" }}>
                {offre.tagline}
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
                  <dt style={{ color: "var(--texte-doux)" }}>Demandes par jour</dt>
                  <dd className="font-semibold tabular-nums">
                    {offre.entitlements.requestsPerDay}
                  </dd>
                </div>
                <div className="flex justify-between gap-3">
                  <dt style={{ color: "var(--texte-doux)" }}>Publier</dt>
                  <dd className="text-right font-semibold">
                    {horizon(offre.entitlements.daysAhead)}
                  </dd>
                </div>
                <div className="flex justify-between gap-3">
                  <dt style={{ color: "var(--texte-doux)" }}>Critères</dt>
                  <dd className="font-semibold">{CRITERES[offre.entitlements.filters]}</dd>
                </div>
                <div className="flex justify-between gap-3">
                  <dt style={{ color: "var(--texte-doux)" }}>Plans de groupe</dt>
                  <dd className="font-semibold">
                    {offre.entitlements.groupPlans ? "Oui" : "À l'unité"}
                  </dd>
                </div>
              </dl>

              <ul className="mt-5 space-y-2 text-sm">
                {offre.highlights.map((point) => (
                  <li key={point} className="flex gap-2.5">
                    <span aria-hidden="true" style={{ color: filVar(couleur) }}>
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

      <h3 className="mt-14 text-2xl font-bold" style={{ fontFamily: "var(--font-titre)" }}>
        Sans abonnement, à l'unité
      </h3>
      <p className="mt-3 max-w-2xl leading-relaxed" style={{ color: "var(--texte-doux)" }}>
        Chaque avantage d'un abonnement s'achète aussi séparément — parce qu'à vingt ans, on ne
        s'abonne pas à tout. On peut utiliser Weave des mois durant sans jamais s'abonner.
      </p>

      <ul className="mt-6 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {UNIT_SKUS.map((sku, index) => {
          const produit = UNIT_PRODUCTS[sku];
          const couleur = ((index % 6) + 1) as Fil;
          return (
            <li
              key={sku}
              className="rounded-2xl p-4"
              style={{ background: "var(--carte)", border: `2px solid ${filVar(couleur)}` }}
            >
              <div className="flex items-baseline justify-between gap-3">
                <span className="font-bold">{produit.name}</span>
                <span className="text-sm font-bold tabular-nums" style={{ color: filVar(couleur) }}>
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
