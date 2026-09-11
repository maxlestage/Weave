import { useEffect, useState } from "react";

const LIENS = [
  { href: "#principe", texte: "Le principe" },
  { href: "#deroule", texte: "Comment ça se passe" },
  { href: "#montre", texte: "iPhone et Watch" },
  { href: "#offres", texte: "Offres" },
] as const;

export function Entete() {
  const [ouvert, setOuvert] = useState(false);

  // Le menu se referme dès qu'on navigue : sur téléphone, il occupe l'écran.
  useEffect(() => {
    if (!ouvert) return;
    const fermer = () => setOuvert(false);
    window.addEventListener("hashchange", fermer);
    return () => window.removeEventListener("hashchange", fermer);
  }, [ouvert]);

  return (
    <header
      className="sticky top-0 z-50 backdrop-blur-md"
      style={{ background: "color-mix(in oklab, var(--fond) 90%, transparent)" }}
    >
      {/* Les six fils, en bandeau : la marque tient en une ligne. */}
      <div className="tissage h-1.5" aria-hidden="true" />
      <div className="mx-auto flex w-full max-w-5xl items-center justify-between px-5 py-3 sm:px-8">
        <a href="#haut" className="flex items-center gap-2.5 font-semibold">
          <Logo />
          <span
            className="text-xl font-bold tracking-tight"
            style={{ fontFamily: "var(--font-titre)" }}
          >
            Weave
          </span>
        </a>

        <nav aria-label="Navigation principale" className="hidden gap-7 text-sm sm:flex">
          {LIENS.map((lien) => (
            <a key={lien.href} href={lien.href} className="hover:underline underline-offset-4">
              {lien.texte}
            </a>
          ))}
        </nav>

        <button
          type="button"
          className="-mr-2 p-2 sm:hidden"
          aria-expanded={ouvert}
          aria-controls="menu-mobile"
          onClick={() => setOuvert((v) => !v)}
        >
          <span className="sr-only">{ouvert ? "Fermer le menu" : "Ouvrir le menu"}</span>
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            {ouvert ? (
              <path
                d="M6 6l12 12M18 6L6 18"
                stroke="currentColor"
                strokeWidth="1.8"
                strokeLinecap="round"
              />
            ) : (
              <path
                d="M4 7h16M4 12h16M4 17h16"
                stroke="currentColor"
                strokeWidth="1.8"
                strokeLinecap="round"
              />
            )}
          </svg>
        </button>
      </div>

      {ouvert && (
        <nav
          id="menu-mobile"
          aria-label="Navigation principale"
          className="sm:hidden"
          style={{ borderTop: "2px solid var(--bordure)" }}
        >
          <ul className="px-5 py-2">
            {LIENS.map((lien) => (
              <li key={lien.href}>
                <a
                  href={lien.href}
                  className="block py-3 text-base"
                  onClick={() => setOuvert(false)}
                  style={{ borderBottom: "1px solid var(--bordure)" }}
                >
                  {lien.texte}
                </a>
              </li>
            ))}
          </ul>
        </nav>
      )}
    </header>
  );
}

function Logo() {
  return (
    <svg width="26" height="26" viewBox="0 0 32 32" aria-hidden="true">
      <g strokeWidth="3" strokeLinecap="round" fill="none">
        <path d="M7 5v22" stroke="var(--fil-1)" />
        <path d="M16 5v22" stroke="var(--fil-4)" />
        <path d="M25 5v22" stroke="var(--fil-6)" />
      </g>
      <g strokeWidth="2.4" strokeLinecap="round" fill="none">
        <path d="M4 12c4 3 8 3 12 0s8-3 12 0" stroke="var(--fil-2)" />
        <path d="M4 21c4 3 8 3 12 0s8-3 12 0" stroke="var(--fil-5)" />
      </g>
    </svg>
  );
}
