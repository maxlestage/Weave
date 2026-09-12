/*
 * Rend l'image de partage (1200×630) à partir de `partage.html`.
 *
 * Elle n'est pas produite à chaque construction : Chromium n'a rien à faire
 * dans une chaîne de déploiement, et l'image ne change qu'avec l'accroche.
 * Le PNG est donc versionné, et ce script sert à le régénérer. Playwright
 * n'est volontairement pas dans les dépendances du site : il serait installé
 * par l'intégration continue et par la construction de l'image Docker, qui
 * n'ont aucun usage d'un navigateur. On l'ajoute le temps du rendu :
 *
 *     cd apps/web
 *     bun add -d playwright && bun run outils/rendre-partage.ts && bun remove playwright
 */

import { chromium } from "playwright";

const racine = new URL("..", import.meta.url).pathname;
const navigateur = await chromium.launch();
const page = await navigateur.newPage({ viewport: { width: 1200, height: 630 } });
await page.goto(`file://${racine}outils/partage.html`);
await page.screenshot({ path: `${racine}public/partage.png` });
await navigateur.close();
console.log("public/partage.png régénéré");
