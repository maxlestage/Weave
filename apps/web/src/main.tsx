import { StrictMode } from "react";
import { createRoot, hydrateRoot } from "react-dom/client";
import { App } from "./App.tsx";
import { langueDuChemin } from "./langues.ts";
import "./styles.css";

const racine = document.getElementById("racine");
if (racine === null) throw new Error("Élément racine introuvable.");

/*
 * La langue vient de l'adresse, et de rien d'autre.
 *
 * `navigator.language` aurait paru plus attentionné, mais il donnerait une
 * page différente de celle que la construction a rendue à cette adresse :
 * l'hydratation échouerait et React reconstruirait tout. Un visiteur qui veut
 * une autre langue suit le lien du choix de langue — et l'adresse suit.
 */
const arbre = (
  <StrictMode>
    <App langue={langueDuChemin(window.location.pathname)} />
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
