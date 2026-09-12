/*
 * Importer une feuille de style pour son seul effet de bord — `import
 * "./styles.css"` — n'est pas du TypeScript standard : c'est le bundler qui
 * donne un sens à cette ligne. TypeScript a donc besoin qu'on lui déclare la
 * forme de ces modules, sans quoi il refuse l'import.
 *
 * Vite fournissait cette déclaration dans `vite/client`. Maintenant que Bun
 * bundle seul, elle nous revient.
 */

declare module "*.css";
