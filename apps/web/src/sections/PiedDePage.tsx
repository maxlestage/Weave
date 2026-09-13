import { MIN_AGE } from "@weave/contracts";
import { DOCUMENTS } from "../pages/documents.ts";

export function PiedDePage() {
  return (
    <footer className="px-5 py-12 sm:px-8" style={{ background: "var(--fond-alterne)" }}>
      <div className="mx-auto w-full max-w-5xl">
        <div className="tissage mb-8 h-1.5 rounded-full" aria-hidden="true" />

        <p className="text-2xl font-bold" style={{ fontFamily: "var(--font-titre)" }}>
          Weave
        </p>
        <p className="mt-2 max-w-md leading-relaxed" style={{ color: "var(--texte-doux)" }}>
          Des plans, pas des profils. On publie ce qu'on compte faire, les autres demandent à venir
          — en écrivant pourquoi.
        </p>

        {/*
          Les ancres sont préfixées par « / » : ce pied de page s'affiche aussi
          sur les pages juridiques, où « #offres » seul ne désignerait rien.
        */}
        <nav aria-label="Le site" className="mt-8">
          <h2 className="text-sm font-semibold tracking-wide uppercase">Le site</h2>
          <ul className="mt-3 flex flex-wrap gap-x-8 gap-y-3 text-sm">
            {[
              { href: "/#principe", texte: "Le principe" },
              { href: "/#deroule", texte: "Comment ça se passe" },
              { href: "/#offres", texte: "Offres" },
              { href: "/#questions", texte: "Questions" },
            ].map((lien) => (
              <li key={lien.href}>
                <a href={lien.href} className="hover:underline underline-offset-4">
                  {lien.texte}
                </a>
              </li>
            ))}
          </ul>
        </nav>

        <nav aria-label="Informations légales" className="mt-7">
          <h2 className="text-sm font-semibold tracking-wide uppercase">Informations légales</h2>
          <ul className="mt-3 flex flex-wrap gap-x-8 gap-y-3 text-sm">
            {DOCUMENTS.map((doc) => (
              <li key={doc.slug}>
                <a href={`/${doc.slug}`} className="hover:underline underline-offset-4">
                  {doc.lien}
                </a>
              </li>
            ))}
          </ul>
        </nav>

        <p className="mt-10 text-sm" style={{ color: "var(--texte-doux)" }}>
          Weave est réservé aux personnes de {MIN_AGE} ans et plus.
        </p>
        {/*
          L'année est calculée au rendu : elle vaut celle de la CONSTRUCTION
          côté serveur, et celle de la consultation côté client. Les deux ne
          diffèrent qu'au passage d'une année, et React reprendrait la valeur
          du client en signalant l'écart. `suppressHydrationWarning` dit que
          cet écart-là est voulu — c'est exactement ce à quoi il sert.
        */}
        <p className="mt-2 text-sm" style={{ color: "var(--texte-doux)" }} suppressHydrationWarning>
          © {new Date().getFullYear()} Weave. Apple, iPhone et Apple Watch sont des marques déposées
          d'Apple Inc.
        </p>
      </div>
    </footer>
  );
}
