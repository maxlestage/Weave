import { MIN_AGE } from "@weave/contracts";
import { useLangue, useTraduction } from "../contexte-langue.tsx";
import { chemin, LANGUE_PAR_DEFAUT, NOM_DE_LA_LANGUE, type Traduit } from "../langues.ts";
import { DOCUMENTS } from "../pages/documents.ts";

const TEXTES: Traduit<{
  accroche: string;
  site: string;
  liens: readonly { readonly ancre: string; readonly texte: string }[];
  legal: string;
  /** Prévient que les documents juridiques ne sont publiés qu'en français. */
  legalEnFrancais: string | null;
  age: (age: number) => string;
  marques: (annee: number) => string;
}> = {
  fr: {
    accroche:
      "Des plans, pas des profils. On publie ce qu'on compte faire, les autres demandent à venir — en écrivant pourquoi.",
    site: "Le site",
    liens: [
      { ancre: "principe", texte: "Le principe" },
      { ancre: "deroule", texte: "Comment ça se passe" },
      { ancre: "offres", texte: "Offres" },
      { ancre: "questions", texte: "Questions" },
    ],
    legal: "Informations légales",
    legalEnFrancais: null,
    age: (age) => `Weave est réservé aux personnes de ${age} ans et plus.`,
    marques: (annee) =>
      `© ${annee} Weave. Apple, iPhone et Apple Watch sont des marques déposées d'Apple Inc.`,
  },
  en: {
    accroche:
      "Plans, not profiles. You post what you're going to do, other people ask to come — and write why.",
    site: "The site",
    liens: [
      { ancre: "principe", texte: "The idea" },
      { ancre: "deroule", texte: "How it goes" },
      { ancre: "offres", texte: "Plans" },
      { ancre: "questions", texte: "Questions" },
    ],
    legal: "Legal information",
    legalEnFrancais:
      "Our legal documents are published in French only. The French text is the one that binds us.",
    age: (age) => `Weave is for people aged ${age} and over.`,
    marques: (annee) =>
      `© ${annee} Weave. Apple, iPhone and Apple Watch are registered trademarks of Apple Inc.`,
  },
  es: {
    accroche:
      "Planes, no perfiles. Publicas lo que piensas hacer, los demás piden venir — y escriben por qué.",
    site: "El sitio",
    liens: [
      { ancre: "principe", texte: "La idea" },
      { ancre: "deroule", texte: "Cómo funciona" },
      { ancre: "offres", texte: "Suscripciones" },
      { ancre: "questions", texte: "Preguntas" },
    ],
    legal: "Información legal",
    legalEnFrancais:
      "Nuestros documentos legales se publican solo en francés. El texto francés es el que nos obliga.",
    age: (age) => `Weave es para mayores de ${age} años.`,
    marques: (annee) =>
      `© ${annee} Weave. Apple, iPhone y Apple Watch son marcas registradas de Apple Inc.`,
  },
};

export function PiedDePage() {
  const langue = useLangue();
  const t = useTraduction(TEXTES);

  return (
    <footer className="px-5 py-12 sm:px-8" style={{ background: "var(--fond-alterne)" }}>
      <div className="mx-auto w-full max-w-5xl">
        <div className="tissage mb-8 h-1.5 rounded-full" aria-hidden="true" />

        <p className="text-2xl font-bold" style={{ fontFamily: "var(--font-titre)" }}>
          Weave
        </p>
        <p className="mt-2 max-w-md leading-relaxed" style={{ color: "var(--texte-doux)" }}>
          {t.accroche}
        </p>

        {/*
          Les ancres portent le préfixe de langue : ce pied de page s'affiche
          aussi sur les pages juridiques, où « #offres » seul ne désignerait
          rien, et sur « /en/ », où il faut revenir à la bonne page d'accueil.
        */}
        <nav aria-label={t.site} className="mt-8">
          <h2 className="text-sm font-semibold tracking-wide uppercase">{t.site}</h2>
          <ul className="mt-3 flex flex-wrap gap-x-8 gap-y-3 text-sm">
            {t.liens.map((lien) => (
              <li key={lien.ancre}>
                <a
                  href={`${chemin(langue)}#${lien.ancre}`}
                  className="hover:underline underline-offset-4"
                >
                  {lien.texte}
                </a>
              </li>
            ))}
          </ul>
        </nav>

        {/*
          Les documents juridiques ne sont publiés qu'en français, et leurs
          liens ne portent donc pas de préfixe de langue. Traduire des
          conditions générales n'est pas un travail de langue : une traduction
          non relue engagerait sur un texte que personne n'a validé. Le
          visiteur anglophone ou hispanophone est prévenu plutôt que mené vers
          une page dont il ne saurait pas qu'elle est dans une autre langue.
        */}
        <nav aria-label={t.legal} className="mt-7">
          <h2 className="text-sm font-semibold tracking-wide uppercase">{t.legal}</h2>
          <ul className="mt-3 flex flex-wrap gap-x-8 gap-y-3 text-sm">
            {DOCUMENTS.map((doc) => (
              <li key={doc.slug}>
                <a
                  href={`/${doc.slug}`}
                  hrefLang={LANGUE_PAR_DEFAUT}
                  lang={LANGUE_PAR_DEFAUT}
                  className="hover:underline underline-offset-4"
                >
                  {doc.lien}
                  {t.legalEnFrancais !== null && (
                    <span className="sr-only"> ({NOM_DE_LA_LANGUE[LANGUE_PAR_DEFAUT]})</span>
                  )}
                </a>
              </li>
            ))}
          </ul>
          {t.legalEnFrancais !== null && (
            <p className="mt-3 max-w-md text-sm" style={{ color: "var(--texte-doux)" }}>
              {t.legalEnFrancais}
            </p>
          )}
        </nav>

        <p className="mt-10 text-sm" style={{ color: "var(--texte-doux)" }}>
          {t.age(MIN_AGE)}
        </p>
        {/*
          L'année est calculée au rendu : elle vaut celle de la CONSTRUCTION
          côté serveur, et celle de la consultation côté client. Les deux ne
          diffèrent qu'au passage d'une année, et React reprendrait la valeur
          du client en signalant l'écart. `suppressHydrationWarning` dit que
          cet écart-là est voulu — c'est exactement ce à quoi il sert.
        */}
        <p className="mt-2 text-sm" style={{ color: "var(--texte-doux)" }} suppressHydrationWarning>
          {t.marques(new Date().getFullYear())}
        </p>
      </div>
    </footer>
  );
}
