import { Appareils } from "./sections/Appareils.tsx";
import { Confiance } from "./sections/Confiance.tsx";
import { Deroule } from "./sections/Deroule.tsx";
import { Entete } from "./sections/Entete.tsx";
import { Offres } from "./sections/Offres.tsx";
import { Ouverture } from "./sections/Ouverture.tsx";
import { PiedDePage } from "./sections/PiedDePage.tsx";
import { Principe } from "./sections/Principe.tsx";
import { Questions } from "./sections/Questions.tsx";

export function App() {
  return (
    <>
      <a
        href="#contenu"
        className="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-[100] focus:rounded-full focus:px-4 focus:py-2"
        style={{ background: "var(--accent)", color: "var(--sur-accent)" }}
      >
        Aller au contenu
      </a>

      <Entete />

      <main id="contenu">
        <Ouverture />
        <Principe />
        <Deroule />
        <Appareils />
        <Offres />
        <Confiance />
        <Questions />
      </main>

      <PiedDePage />
    </>
  );
}
