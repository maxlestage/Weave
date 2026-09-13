/*
 * Rend l'icône de l'écran d'accueil (180×180) à partir de `icone.html`.
 *
 * Même raison que pour l'image de partage : Chromium n'a rien à faire dans une
 * chaîne de déploiement, et l'icône ne change qu'avec la marque. Le PNG est
 * donc versionné, et ce script sert à le régénérer.
 *
 *     cd apps/web
 *     bun add -d playwright && bun run outils/rendre-icone.ts && bun remove playwright
 *
 * 180 points est la taille demandée par les iPhone récents ; iOS réduit
 * lui-même pour les autres. Une seule image, plutôt que la demi-douzaine de
 * tailles que l'on voit souvent recopiées.
 */

import { chromium } from "playwright";

const racine = new URL("..", import.meta.url).pathname;
const navigateur = await chromium.launch();
const page = await navigateur.newPage({
  viewport: { width: 180, height: 180 },
  deviceScaleFactor: 1,
});
await page.goto(`file://${racine}outils/icone.html`);
await page.screenshot({ path: `${racine}public/apple-touch-icon.png` });
await navigateur.close();
console.log("public/apple-touch-icon.png régénéré");
