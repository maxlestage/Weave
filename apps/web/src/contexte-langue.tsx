import { createContext, useContext, type ReactNode } from "react";
import { LANGUE_PAR_DEFAUT, type Langue } from "./langues.ts";

/*
 * La langue courante, portée par un contexte.
 *
 * Elle est posée une fois — par la construction pour chaque page rendue, et
 * par l'adresse au démarrage côté client — plutôt que passée de composant en
 * composant : les sections sont imbriquées, et un accessoire traversant dix
 * niveaux se perd au premier oubli.
 */
const ContexteLangue = createContext<Langue>(LANGUE_PAR_DEFAUT);

export function FournisseurDeLangue({ langue, children }: { langue: Langue; children: ReactNode }) {
  return <ContexteLangue.Provider value={langue}>{children}</ContexteLangue.Provider>;
}

export function useLangue(): Langue {
  return useContext(ContexteLangue);
}

/** Choisit la variante correspondant à la langue courante. */
export function useTraduction<T>(contenu: Readonly<Record<Langue, T>>): T {
  return contenu[useLangue()];
}
