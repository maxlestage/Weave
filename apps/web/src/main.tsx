import { StrictMode } from "react";
import { createRoot, hydrateRoot } from "react-dom/client";
import { App } from "./App.tsx";
import "./styles.css";

const racine = document.getElementById("racine");
if (racine === null) throw new Error("Élément racine introuvable.");

const arbre = (
  <StrictMode>
    <App />
  </StrictMode>
);

/*
 * Hydrater ce qui a été pré-rendu, plutôt que de le remplacer.
 *
 * La construction rend désormais la page d'accueil en HTML, comme elle le
 * faisait déjà des pages juridiques. `createRoot` jetterait ce balisage pour
 * le reconstruire à l'identique — la page clignoterait, et le travail du
 * serveur serait perdu.
 *
 * Le serveur de développement, lui, sert un `index.html` dont la racine est
 * vide : il n'y a rien à hydrater, et `hydrateRoot` s'en plaindrait. On
 * regarde donc ce qu'on a reçu plutôt que de supposer.
 */
if (racine.firstChild === null) {
  createRoot(racine).render(arbre);
} else {
  hydrateRoot(racine, arbre);
}
