import { MAX_ACTIVE_THREADS, MIN_AGE } from "@weave/contracts";

export function PiedDePage() {
  return (
    <footer className="px-5 py-12 sm:px-8" style={{ background: "var(--fond-alterne)" }}>
      <div className="mx-auto w-full max-w-5xl">
        <div className="tissage mb-8 h-1.5 rounded-full" aria-hidden="true" />

        <p className="text-2xl font-bold" style={{ fontFamily: "var(--font-titre)" }}>
          Weave
        </p>
        <p className="mt-2 max-w-md leading-relaxed" style={{ color: "var(--texte-doux)" }}>
          {MAX_ACTIVE_THREADS} fils par jour. Une application de rencontre qui préfère la
          conversation à la collection.
        </p>

        <nav aria-label="Liens de pied de page" className="mt-8">
          <ul className="flex flex-wrap gap-x-8 gap-y-3 text-sm">
            <li>
              <a href="#principe" className="hover:underline underline-offset-4">
                Le principe
              </a>
            </li>
            <li>
              <a href="#offres" className="hover:underline underline-offset-4">
                Offres
              </a>
            </li>
            <li>
              <a href="#confiance" className="hover:underline underline-offset-4">
                Confidentialité
              </a>
            </li>
            <li>
              <a href="#questions" className="hover:underline underline-offset-4">
                Questions
              </a>
            </li>
          </ul>
        </nav>

        <p className="mt-10 text-sm" style={{ color: "var(--texte-doux)" }}>
          Weave est réservé aux personnes de {MIN_AGE} ans et plus.
        </p>
        <p className="mt-2 text-sm" style={{ color: "var(--texte-doux)" }}>
          © {new Date().getFullYear()} Weave. Apple, iPhone et Apple Watch sont des marques déposées
          d'Apple Inc.
        </p>
      </div>
    </footer>
  );
}
