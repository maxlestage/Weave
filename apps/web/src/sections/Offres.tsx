import {
  LOCALE_DE_PRIX,
  PLAN_TIERS,
  REQUESTS_PER_DAY_FLOOR,
  TIERS,
  TIER_COPY_PAR_LANGUE,
  UNIT_DESCRIPTIONS_PAR_LANGUE,
  UNIT_PRODUCTS,
  UNIT_SKUS,
  formatPrice,
  type FilterDepth,
} from "@weave/contracts";
import { Carte, Etiquette, filVar, Section, type Fil } from "../composants.tsx";
import { useLangue, useTraduction } from "../contexte-langue.tsx";
import type { Traduit } from "../langues.ts";

/** Chaque palier porte sa couleur. */
const COULEURS: Record<string, Fil> = {
  depart: 4,
  viree: 3,
  escapade: 6,
  expedition: 5,
  grandtour: 1,
};

/**
 * L'horizon de publication est un nombre de jours ; on le dit en semaines ou
 * en mois quand c'en est un compte rond, parce que « 60 jours à l'avance »
 * ne se lit pas comme on le pense.
 */
type Horizon = { readonly unite: "jour" | "semaine" | "mois"; readonly nombre: number };

function decoupeHorizon(jours: number): Horizon {
  if (jours >= 30) return { unite: "mois", nombre: Math.round(jours / 30) };
  if (jours >= 7) return { unite: "semaine", nombre: Math.round(jours / 7) };
  return { unite: "jour", nombre: jours };
}

const TEXTES: Traduit<{
  titre: string;
  chapeau: (plancher: number) => string;
  leplus: string;
  gratuit: string;
  parMois: string;
  demandesParJour: string;
  publier: string;
  criteres: string;
  criteresLibelles: Record<FilterDepth, string>;
  plansDeGroupe: string;
  oui: string;
  aLUnite: string;
  horizon: (h: Horizon) => string;
  titreUnites: string;
  chapeauUnites: string;
  fin: string;
}> = {
  fr: {
    titre: "Quatre abonnements, et tout à l'unité",
    chapeau: (plancher) =>
      `Aucune offre n'achète de visibilité : payer ne fait jamais remonter un plan. Ce qui se paie, c'est l'horizon de publication, la finesse des critères et les plans de groupe. Le nombre de demandes reste borné partout — au minimum ${plancher} par jour.`,
    leplus: "Le plus choisi",
    gratuit: "Gratuit",
    parMois: " / mois",
    demandesParJour: "Demandes par jour",
    publier: "Publier",
    criteres: "Critères",
    criteresLibelles: { base: "De base", etendus: "Étendus", precis: "Précis" },
    plansDeGroupe: "Plans de groupe",
    oui: "Oui",
    aLUnite: "À l'unité",
    horizon: ({ unite, nombre }) => {
      if (unite === "mois")
        return nombre === 1 ? "un mois à l'avance" : `${nombre} mois à l'avance`;
      if (unite === "semaine")
        return nombre === 1 ? "une semaine à l'avance" : `${nombre} semaines à l'avance`;
      return nombre === 1 ? "un jour à l'avance" : `${nombre} jours à l'avance`;
    },
    titreUnites: "Sans abonnement, à l'unité",
    chapeauUnites:
      "Chaque avantage d'un abonnement s'achète aussi séparément — parce qu'à vingt ans, on ne s'abonne pas à tout. On peut utiliser Weave des mois durant sans jamais s'abonner.",
    fin: "Les abonnements se souscrivent et se résilient depuis les réglages de votre compte Apple. Aucun moyen de paiement ne transite par nos serveurs.",
  },
  en: {
    titre: "Four subscriptions, and everything sold singly",
    chapeau: (plancher) =>
      `No plan buys visibility: paying never pushes a plan up the feed. What you pay for is how far ahead you can post, how precise the filters are, and group plans. The number of requests stays capped everywhere — at least ${plancher} a day.`,
    leplus: "Most chosen",
    gratuit: "Free",
    parMois: " / month",
    demandesParJour: "Requests a day",
    publier: "Post",
    criteres: "Filters",
    criteresLibelles: { base: "Basic", etendus: "Extended", precis: "Precise" },
    plansDeGroupe: "Group plans",
    oui: "Yes",
    aLUnite: "Sold singly",
    horizon: ({ unite, nombre }) => {
      if (unite === "mois") return nombre === 1 ? "a month ahead" : `${nombre} months ahead`;
      if (unite === "semaine") return nombre === 1 ? "a week ahead" : `${nombre} weeks ahead`;
      return nombre === 1 ? "a day ahead" : `${nombre} days ahead`;
    },
    titreUnites: "Without a subscription, one at a time",
    chapeauUnites:
      "Every benefit of a subscription can also be bought on its own — because at twenty you don't subscribe to everything. You can use Weave for months without ever subscribing.",
    fin: "Subscriptions are taken out and cancelled from your Apple account settings. No payment details pass through our servers.",
  },
  es: {
    titre: "Cuatro suscripciones, y todo por unidades",
    chapeau: (plancher) =>
      `Ninguna suscripción compra visibilidad: pagar nunca sube un plan en el muro. Lo que se paga es con cuánta antelación puedes publicar, lo finos que son los criterios y los planes de grupo. El número de peticiones sigue limitado en todas — al menos ${plancher} al día.`,
    leplus: "La más elegida",
    gratuit: "Gratis",
    parMois: " / mes",
    demandesParJour: "Peticiones al día",
    publier: "Publicar",
    criteres: "Criterios",
    criteresLibelles: { base: "Básicos", etendus: "Ampliados", precis: "Precisos" },
    plansDeGroupe: "Planes de grupo",
    oui: "Sí",
    aLUnite: "Por unidades",
    horizon: ({ unite, nombre }) => {
      if (unite === "mois")
        return nombre === 1 ? "un mes de antelación" : `${nombre} meses de antelación`;
      if (unite === "semaine")
        return nombre === 1 ? "una semana de antelación" : `${nombre} semanas de antelación`;
      return nombre === 1 ? "un día de antelación" : `${nombre} días de antelación`;
    },
    titreUnites: "Sin suscripción, por unidades",
    chapeauUnites:
      "Cada ventaja de una suscripción se puede comprar también por separado — porque a los veinte años uno no se suscribe a todo. Se puede usar Weave durante meses sin suscribirse nunca.",
    fin: "Las suscripciones se contratan y se cancelan desde los ajustes de tu cuenta de Apple. Ningún medio de pago pasa por nuestros servidores.",
  },
};

export function Offres() {
  const langue = useLangue();
  const t = useTraduction(TEXTES);
  const copies = TIER_COPY_PAR_LANGUE[langue];
  const descriptions = UNIT_DESCRIPTIONS_PAR_LANGUE[langue];
  const prix = (cents: number) => formatPrice(cents, LOCALE_DE_PRIX[langue]);

  return (
    <Section id="offres" fil={6} titre={t.titre} chapeau={t.chapeau(REQUESTS_PER_DAY_FLOOR)}>
      <div className="mt-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {PLAN_TIERS.map((tier) => {
          const offre = TIERS[tier];
          const copie = copies[tier];
          const gratuit = offre.monthlyPriceCents === 0;
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
                {tier === "escapade" && <Etiquette fil={couleur}>{t.leplus}</Etiquette>}
              </div>

              <p className="mt-2 text-sm" style={{ color: "var(--texte-doux)" }}>
                {copie.tagline}
              </p>

              <p className="mt-5">
                <span className="text-3xl font-semibold tabular-nums">
                  {gratuit ? t.gratuit : prix(offre.monthlyPriceCents)}
                </span>
                <span className="text-sm" style={{ color: "var(--texte-doux)" }}>
                  {gratuit ? "" : t.parMois}
                </span>
              </p>

              <dl className="mt-5 space-y-1.5 text-sm">
                <div className="flex justify-between gap-3">
                  <dt style={{ color: "var(--texte-doux)" }}>{t.demandesParJour}</dt>
                  <dd className="font-semibold tabular-nums">
                    {offre.entitlements.requestsPerDay}
                  </dd>
                </div>
                <div className="flex justify-between gap-3">
                  <dt style={{ color: "var(--texte-doux)" }}>{t.publier}</dt>
                  <dd className="text-right font-semibold">
                    {t.horizon(decoupeHorizon(offre.entitlements.daysAhead))}
                  </dd>
                </div>
                <div className="flex justify-between gap-3">
                  <dt style={{ color: "var(--texte-doux)" }}>{t.criteres}</dt>
                  <dd className="font-semibold">
                    {t.criteresLibelles[offre.entitlements.filters]}
                  </dd>
                </div>
                <div className="flex justify-between gap-3">
                  <dt style={{ color: "var(--texte-doux)" }}>{t.plansDeGroupe}</dt>
                  <dd className="font-semibold">
                    {offre.entitlements.groupPlans ? t.oui : t.aLUnite}
                  </dd>
                </div>
              </dl>

              <ul className="mt-5 space-y-2 text-sm">
                {copie.highlights.map((point) => (
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
        {t.titreUnites}
      </h3>
      <p className="mt-3 max-w-2xl leading-relaxed" style={{ color: "var(--texte-doux)" }}>
        {t.chapeauUnites}
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
                  {prix(produit.priceCents)}
                </span>
              </div>
              <p className="mt-2 text-sm leading-relaxed" style={{ color: "var(--texte-doux)" }}>
                {descriptions[sku]}
              </p>
            </li>
          );
        })}
      </ul>

      <p className="mt-8 text-sm" style={{ color: "var(--texte-doux)" }}>
        {t.fin}
      </p>
    </Section>
  );
}
