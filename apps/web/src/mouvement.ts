/*
 * Le mouvement de l'accueil : ce que la feuille de style ne sait pas faire seule.
 *
 * - les sections apparaissent au défilement, par vagues ;
 * - les fils de l'ouverture ondulent et s'écartent sous le pointeur ;
 * - les boutons se laissent attirer par le pointeur, les cartes s'inclinent ;
 * - un anneau suit le pointeur, avec un temps de retard.
 *
 * ## Ce que ce module s'interdit
 *
 * Il ne réécrit aucun texte et n'ajoute aucune balise dans ce que React rend :
 * il pose des attributs `data-*` et des propriétés CSS, que React ne connaît
 * pas et ne viendra donc pas effacer. Les deux seuls éléments qu'il crée — la
 * toile des fils et l'anneau — sont décoratifs, cachés aux lecteurs d'écran,
 * et retirés à l'arrêt.
 *
 * Il ne démarre qu'après l'hydratation (depuis un effet) : toucher au DOM avant
 * ferait diverger la page de ce que React s'attend à reprendre.
 *
 * Et il ne fait rien du tout pour qui a demandé moins de mouvement.
 *
 * ## Pourquoi pas GSAP ni Lenis
 *
 * C'est ce qu'emploient les sites de ce genre. Mais tout ce qu'on en garde
 * tient en un lissage (`approcher`) et une boucle d'animation : une dépendance
 * de 70 ko pour cela alourdirait un site consulté surtout en 4G. Et le
 * défilement reste celui du système — le remplacer par un défilement « lissé »
 * casse l'inertie d'un iPhone, le clavier et les ancres.
 */

/** Rapproche `valeur` de `cible`, d'une fraction par image : le lissage. */
const approcher = (valeur: number, cible: number, fraction: number) =>
  Math.abs(cible - valeur) < 0.01 ? cible : valeur + (cible - valeur) * fraction;

/** Démarre le mouvement, et rend de quoi tout défaire. */
export function demarrerMouvement(): () => void {
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return () => {};

  const arrets: (() => void)[] = [];
  const taches = new Set<(temps: number) => void>();

  let image = 0;
  const boucle = (temps: number) => {
    for (const tache of taches) tache(temps);
    image = requestAnimationFrame(boucle);
  };
  image = requestAnimationFrame(boucle);
  arrets.push(() => cancelAnimationFrame(image));

  arrets.push(apparitions());
  arrets.push(metier(taches));

  // Le reste suit un pointeur : sur un écran tactile, il n'y en a pas.
  if (window.matchMedia("(hover: hover) and (pointer: fine)").matches) {
    arrets.push(aimants(taches));
    arrets.push(inclinaisons(taches));
    arrets.push(curseur(taches));
  }

  return () => {
    for (const arret of arrets.reverse()) arret();
    taches.clear();
  };
}

/* — Les apparitions ————————————————————————————————————————————— */

/**
 * Les sections apparaissent quand on y arrive : le titre sort d'une fente, le
 * trait de couleur se déroule, les cartes montent l'une après l'autre.
 *
 * Seul ce qui est encore sous l'écran est caché. Arriver par une ancre
 * (`/#offres`) ou recharger au milieu de la page ne fait rien disparaître de
 * ce qu'on avait déjà sous les yeux.
 */
function apparitions(): () => void {
  if (!("IntersectionObserver" in window)) return () => {};

  type Genre = "titre" | "trait" | "bloc";
  // Le guetteur est ce qu'on observe. Un titre caché derrière son masque n'a
  // plus de surface visible : l'observer lui-même, c'est attendre qu'il se
  // montre pour le montrer — il ne se montrait jamais. On guette donc le haut
  // de sa section, qui, lui, ne bouge pas.
  const cibles: { element: HTMLElement; genre: Genre; guetteur: HTMLElement }[] = [];

  for (const section of document.querySelectorAll<HTMLElement>("main section:not(#haut)")) {
    const titre = section.querySelector<HTMLElement>("h2");
    if (titre === null) continue;
    const enTete = titre.parentElement ?? section;

    const trait = titre.previousElementSibling;
    if (trait instanceof HTMLElement && trait.getAttribute("aria-hidden") === "true") {
      cibles.push({ element: trait, genre: "trait", guetteur: enTete });
    }
    cibles.push({ element: titre, genre: "titre", guetteur: enTete });

    // Ce qui suit le titre : le chapeau, puis le contenu. Une grille ou une
    // liste ne monte pas d'un bloc — ce sont ses cartes qui montent.
    let suivant = titre.nextElementSibling;
    while (suivant instanceof HTMLElement) {
      for (const bloc of blocs(suivant)) {
        cibles.push({ element: bloc, genre: "bloc", guetteur: bloc });
      }
      suivant = suivant.nextElementSibling;
    }
  }

  const hauteur = window.innerHeight;
  const cachees = cibles.filter(({ element }) => element.getBoundingClientRect().top > hauteur);

  const parGuetteur = new Map<Element, HTMLElement[]>();
  for (const { element, guetteur } of cachees) {
    parGuetteur.set(guetteur, [...(parGuetteur.get(guetteur) ?? []), element]);
  }

  const observateur = new IntersectionObserver(
    (entrees) => {
      // Ce qui entre ensemble part en vague : chacun un peu après le
      // précédent, sans qu'une longue liste fasse attendre le dernier.
      let rang = 0;
      for (const entree of entrees) {
        if (!entree.isIntersecting) continue;
        for (const element of parGuetteur.get(entree.target) ?? []) {
          element.style.setProperty("--delai", `${Math.min(rang, 5) * 90}ms`);
          element.dataset.vu = "oui";
          rang += 1;
        }
        observateur.unobserve(entree.target);
      }
    },
    { rootMargin: "0px 0px -8% 0px" },
  );

  for (const { element, genre } of cachees) {
    element.dataset.revele = genre;
    element.dataset.vu = "non";
  }
  for (const guetteur of parGuetteur.keys()) observateur.observe(guetteur);

  return () => {
    observateur.disconnect();
    for (const { element } of cachees) {
      delete element.dataset.revele;
      delete element.dataset.vu;
      element.style.removeProperty("--delai");
    }
  };
}

/** Les blocs qui montent un à un dans un conteneur de section. */
function blocs(conteneur: HTMLElement): HTMLElement[] {
  const enfants = [...conteneur.children].filter((e): e is HTMLElement => e instanceof HTMLElement);
  // Un conteneur qui n'enveloppe qu'une grille ou une liste : ce sont ses
  // cartes qui comptent, pas l'enveloppe.
  if (enfants.length === 1 && enfants[0]!.children.length > 1) return blocs(enfants[0]!);
  if (enfants.length > 1 && enfants.length <= 12) return enfants;
  return [conteneur];
}

/* — Les fils de l'ouverture ————————————————————————————————————— */

/**
 * Six fils de trame traversent l'ouverture et ondulent, chacun à son rythme.
 * Le pointeur — ou le doigt — les écarte sur son passage, comme une main
 * passée dans un métier à tisser.
 */
function metier(taches: Set<(temps: number) => void>): () => void {
  const ouverture = document.getElementById("haut");
  if (ouverture === null) return () => {};

  const toile = document.createElement("canvas");
  toile.className = "metier";
  toile.setAttribute("aria-hidden", "true");
  ouverture.prepend(toile);
  const contexte = toile.getContext("2d");
  if (contexte === null) {
    toile.remove();
    return () => {};
  }

  let largeur = 0;
  let hauteur = 0;
  let densite = 1;
  const dimensionner = () => {
    densite = Math.min(window.devicePixelRatio || 1, 2);
    largeur = ouverture.clientWidth;
    hauteur = ouverture.clientHeight;
    toile.width = Math.round(largeur * densite);
    toile.height = Math.round(hauteur * densite);
  };
  dimensionner();
  const observateurTaille = new ResizeObserver(dimensionner);
  observateurTaille.observe(ouverture);

  // Les couleurs des fils suivent le thème : on les relit s'il change.
  let couleurs: string[] = [];
  const lireCouleurs = () => {
    const style = getComputedStyle(document.documentElement);
    couleurs = [1, 2, 3, 4, 5, 6].map((n) => style.getPropertyValue(`--fil-${n}`).trim());
  };
  lireCouleurs();
  const theme = window.matchMedia("(prefers-color-scheme: dark)");
  theme.addEventListener("change", lireCouleurs);

  // Le pointeur, tel qu'il est, et tel que les fils le sentent : lissé.
  const pointeur = { x: -1e4, y: -1e4, force: 0 };
  const senti = { x: -1e4, y: -1e4, force: 0 };
  const suivre = (evenement: PointerEvent) => {
    const cadre = toile.getBoundingClientRect();
    pointeur.x = evenement.clientX - cadre.left;
    pointeur.y = evenement.clientY - cadre.top;
    pointeur.force = 1;
    if (senti.force === 0) {
      senti.x = pointeur.x;
      senti.y = pointeur.y;
    }
  };
  const lacher = () => {
    pointeur.force = 0;
  };
  // Un doigt qui se lève, un pointeur qui sort de la fenêtre : les fils se
  // referment.
  const sortir = (evenement: PointerEvent) => {
    if (evenement.relatedTarget === null) lacher();
  };
  const lever = (evenement: PointerEvent) => {
    if (evenement.pointerType !== "mouse") lacher();
  };
  window.addEventListener("pointermove", suivre, { passive: true });
  window.addEventListener("pointerdown", suivre, { passive: true });
  window.addEventListener("pointerup", lever);
  window.addEventListener("pointercancel", lacher);
  window.addEventListener("pointerout", sortir);
  window.addEventListener("blur", lacher);

  // Hors de l'écran, on ne dessine rien.
  let visible = true;
  const observateurVue = new IntersectionObserver(([entree]) => {
    visible = entree?.isIntersecting ?? true;
  });
  observateurVue.observe(ouverture);

  const fils = couleurs.map((_, i) => ({
    hauteur: 0.34 + i * 0.12,
    amplitude: 10 + (i % 3) * 7,
    onde: 0.0045 + (i % 2) * 0.0022,
    vitesse: 0.00045 + i * 0.00008,
    phase: i * 1.7,
  }));

  const dessiner = (temps: number) => {
    if (!visible || largeur === 0) return;

    senti.x = approcher(senti.x, pointeur.x, 0.1);
    senti.y = approcher(senti.y, pointeur.y, 0.1);
    senti.force = approcher(senti.force, pointeur.force, 0.05);

    contexte.setTransform(densite, 0, 0, densite, 0, 0);
    contexte.clearRect(0, 0, largeur, hauteur);
    contexte.lineCap = "round";
    contexte.lineWidth = largeur < 640 ? 2 : 2.6;
    contexte.globalAlpha = 0.55;

    const portee = Math.max(90, Math.min(largeur, hauteur) * 0.22);
    const pas = largeur < 640 ? 14 : 10;

    fils.forEach((fil, i) => {
      const base = hauteur * fil.hauteur;
      contexte.strokeStyle = couleurs[i] ?? "currentColor";
      contexte.beginPath();
      for (let x = -pas; x <= largeur + pas; x += pas) {
        let y =
          base +
          Math.sin(x * fil.onde + temps * fil.vitesse + fil.phase) * fil.amplitude +
          Math.sin(x * fil.onde * 0.37 - temps * fil.vitesse * 0.6) * fil.amplitude * 0.6;

        // Le pointeur écarte le fil : d'autant plus qu'il en est près.
        const dx = x - senti.x;
        const dy = y - senti.y;
        const proximite = Math.exp(-(dx * dx + dy * dy) / (2 * portee * portee));
        y += Math.sign(dy || 1) * proximite * portee * 0.55 * senti.force;

        if (x === -pas) contexte.moveTo(x, y);
        else contexte.lineTo(x, y);
      }
      contexte.stroke();
    });
  };
  taches.add(dessiner);

  return () => {
    taches.delete(dessiner);
    observateurTaille.disconnect();
    observateurVue.disconnect();
    theme.removeEventListener("change", lireCouleurs);
    window.removeEventListener("pointermove", suivre);
    window.removeEventListener("pointerdown", suivre);
    window.removeEventListener("pointerup", lever);
    window.removeEventListener("pointercancel", lacher);
    window.removeEventListener("pointerout", sortir);
    window.removeEventListener("blur", lacher);
    toile.remove();
  };
}

/* — Les boutons aimantés ———————————————————————————————————————— */

/**
 * Les boutons de la page — les liens en forme de pilule — se laissent attirer
 * par le pointeur qui s'en approche, et reviennent en place quand il s'éloigne.
 */
function aimants(taches: Set<(temps: number) => void>): () => void {
  const boutons = [...document.querySelectorAll<HTMLElement>("main a.rounded-full")].map(
    (element) => ({ element, x: 0, y: 0, cibleX: 0, cibleY: 0 }),
  );
  if (boutons.length === 0) return () => {};

  const suivre = (evenement: PointerEvent) => {
    for (const bouton of boutons) {
      const cadre = bouton.element.getBoundingClientRect();
      const dx = evenement.clientX - (cadre.left + cadre.width / 2);
      const dy = evenement.clientY - (cadre.top + cadre.height / 2);
      // Le champ dépasse un peu le bouton : on le sent venir.
      const pres = Math.abs(dx) < cadre.width / 2 + 40 && Math.abs(dy) < cadre.height / 2 + 30;
      bouton.cibleX = pres ? dx * 0.3 : 0;
      bouton.cibleY = pres ? dy * 0.4 : 0;
    }
  };
  window.addEventListener("pointermove", suivre, { passive: true });

  for (const { element } of boutons) element.dataset.aimant = "";

  const deplacer = () => {
    for (const bouton of boutons) {
      if (bouton.x === bouton.cibleX && bouton.y === bouton.cibleY) continue;
      bouton.x = approcher(bouton.x, bouton.cibleX, 0.18);
      bouton.y = approcher(bouton.y, bouton.cibleY, 0.18);
      bouton.element.style.setProperty("--aimant-x", `${bouton.x.toFixed(2)}px`);
      bouton.element.style.setProperty("--aimant-y", `${bouton.y.toFixed(2)}px`);
    }
  };
  taches.add(deplacer);

  return () => {
    taches.delete(deplacer);
    window.removeEventListener("pointermove", suivre);
    for (const { element } of boutons) {
      delete element.dataset.aimant;
      element.style.removeProperty("--aimant-x");
      element.style.removeProperty("--aimant-y");
    }
  };
}

/* — Les cartes de l'ouverture —————————————————————————————————— */

/**
 * Les trois plans de l'illustration s'inclinent vers le pointeur, et glissent
 * à des vitesses différentes quand on fait défiler : un léger relief.
 */
function inclinaisons(taches: Set<(temps: number) => void>): () => void {
  const cartes = [...document.querySelectorAll<HTMLElement>("#haut article")].map(
    (element, rang) => ({ element, rang, x: 0, y: 0, cibleX: 0, cibleY: 0, parallaxe: 0 }),
  );
  if (cartes.length === 0) return () => {};

  const suivre = (evenement: PointerEvent) => {
    for (const carte of cartes) {
      const cadre = carte.element.getBoundingClientRect();
      const dedans =
        evenement.clientX >= cadre.left &&
        evenement.clientX <= cadre.right &&
        evenement.clientY >= cadre.top &&
        evenement.clientY <= cadre.bottom;
      carte.cibleX = dedans ? -((evenement.clientY - cadre.top) / cadre.height - 0.5) * 9 : 0;
      carte.cibleY = dedans ? ((evenement.clientX - cadre.left) / cadre.width - 0.5) * 12 : 0;
    }
  };
  window.addEventListener("pointermove", suivre, { passive: true });

  for (const { element } of cartes) element.dataset.incline = "";

  const deplacer = () => {
    const defilement = Math.min(window.scrollY, window.innerHeight * 1.5);
    for (const carte of cartes) {
      carte.x = approcher(carte.x, carte.cibleX, 0.12);
      carte.y = approcher(carte.y, carte.cibleY, 0.12);
      carte.parallaxe = approcher(carte.parallaxe, -defilement * (0.03 + carte.rang * 0.035), 0.15);
      const style = carte.element.style;
      style.setProperty("--incline-x", `${carte.x.toFixed(2)}deg`);
      style.setProperty("--incline-y", `${carte.y.toFixed(2)}deg`);
      style.setProperty("--parallaxe", `${carte.parallaxe.toFixed(1)}px`);
    }
  };
  taches.add(deplacer);

  return () => {
    taches.delete(deplacer);
    window.removeEventListener("pointermove", suivre);
    for (const { element } of cartes) {
      delete element.dataset.incline;
      for (const nom of ["--incline-x", "--incline-y", "--parallaxe"]) {
        element.style.removeProperty(nom);
      }
    }
  };
}

/* — L'anneau ———————————————————————————————————————————————————— */

/** Un anneau suit le pointeur, en retard, et grossit sur ce qui se clique. */
function curseur(taches: Set<(temps: number) => void>): () => void {
  const anneau = document.createElement("div");
  anneau.className = "curseur";
  anneau.setAttribute("aria-hidden", "true");
  document.body.append(anneau);

  const cible = { x: -100, y: -100 };
  const position = { x: -100, y: -100 };

  const suivre = (evenement: PointerEvent) => {
    if (evenement.pointerType !== "mouse") return;
    cible.x = evenement.clientX;
    cible.y = evenement.clientY;
    if (!("visible" in anneau.dataset)) {
      position.x = cible.x;
      position.y = cible.y;
      anneau.dataset.visible = "";
    }
    const survol =
      evenement.target instanceof Element &&
      evenement.target.closest("a, button, summary, [role='button']") !== null;
    if (survol) anneau.dataset.survol = "";
    else delete anneau.dataset.survol;
  };
  const cacher = (evenement: PointerEvent) => {
    if (evenement.relatedTarget === null) delete anneau.dataset.visible;
  };
  window.addEventListener("pointermove", suivre, { passive: true });
  window.addEventListener("pointerout", cacher);

  const deplacer = () => {
    position.x = approcher(position.x, cible.x, 0.2);
    position.y = approcher(position.y, cible.y, 0.2);
    anneau.style.transform = `translate3d(${position.x}px, ${position.y}px, 0)`;
  };
  taches.add(deplacer);

  return () => {
    taches.delete(deplacer);
    window.removeEventListener("pointermove", suivre);
    window.removeEventListener("pointerout", cacher);
    anneau.remove();
  };
}
