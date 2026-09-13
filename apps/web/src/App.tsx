import { Appareils } from "./sections/Appareils.tsx";
import { Confiance } from "./sections/Confiance.tsx";
import { Deroule } from "./sections/Deroule.tsx";
import { Entete } from "./sections/Entete.tsx";
import { Offres } from "./sections/Offres.tsx";
import { Ouverture } from "./sections/Ouverture.tsx";
import { PiedDePage } from "./sections/PiedDePage.tsx";
import { Principe } from "./sections/Principe.tsx";
import { Questions } from "./sections/Questions.tsx";
import { FournisseurDeLangue, useTraduction } from "./contexte-langue.tsx";
import { LANGUE_PAR_DEFAUT, type Langue, type Traduit } from "./langues.ts";

const TEXTES: Traduit<{ contenu: string }> = {
  fr: { contenu: "Aller au contenu" },
  en: { contenu: "Skip to content" },
  es: { contenu: "Ir al contenido" },
};

/**
 * La page d'accueil, dans la langue qu'on lui donne.
 *
 * La langue est un accessoire et non une détection : la construction rend la
 * même application trois fois, une par adresse, et le client reprend celle
 * que l'adresse indique. Deviner d'après `navigator.language` produirait un
 * balisage différent de celui rendu au serveur, et l'hydratation échouerait.
 */
export function App({ langue = LANGUE_PAR_DEFAUT }: { langue?: Langue }) {
  return (
    <FournisseurDeLangue langue={langue}>
      <Page />
    </FournisseurDeLangue>
  );
}

function Page() {
  const t = useTraduction(TEXTES);

  return (
    <>
      <a
        href="#contenu"
        className="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-[100] focus:rounded-full focus:px-4 focus:py-2"
        style={{ background: "var(--accent)", color: "var(--sur-accent)" }}
      >
        {t.contenu}
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
