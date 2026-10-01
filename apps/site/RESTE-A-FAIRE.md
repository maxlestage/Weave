# Reprise du front en Yew — où en est le travail

Le front React (`apps/web`) est en cours de remplacement par Yew
(`apps/site`). **Rien n'est débranché** : `apps/web` construit toujours le
site livré, et le fera jusqu'à ce que le portage soit complet et comparé.

## Ce qui est éprouvé

| | |
|---|---|
| Yew | 0.23.0 — la dernière publiée (mars 2026), MSRV 1.84 |
| Rendu | `ServerRenderer` natif, aucune cible wasm, aucun `trunk` |
| Marqueurs d'hydratation | désactivés (`hydratable(false)`) |
| Contrat partagé | **engendré** depuis `packages/contracts`, pas recopié |
| Prix | mis en forme par le vrai `formatPrice` : `4,99 €` / `€4.99` |

## Pourquoi aucun JavaScript n'est envoyé

Toute l'interactivité du site tient en **un menu de téléphone**. Les questions
dépliantes utilisaient déjà `<details>` natif. Le menu en devient un aussi.

Le navigateur ne reçoit donc qu'un HTML et une feuille de style, là où React
envoyait **84 Kio compressés** de JavaScript — sur un site que l'on consulte
surtout en 4G, et dont la construction mesure ce poids à chaque passage.

C'est l'inverse de ce qu'une reprise en wasm aurait donné : un binaire de
150 à 250 Kio compressés pour animer un menu.

## Ce qui reste à porter

Sections, par ordre de difficulté croissante :

- [ ] `Deroule.tsx` — 137 lignes
- [ ] `PiedDePage.tsx` — 155
- [ ] `Questions.tsx` — 153 (déjà en `<details>`, portage direct)
- [ ] `Appareils.tsx` — 170
- [ ] `Confiance.tsx` — 177
- [ ] `Entete.tsx` — 202 (le menu devient `<details>`)
- [ ] `Principe.tsx` — 227
- [x] `Offres.tsx` — 254 (fait : c'est la tranche qui a servi de preuve)
- [ ] `Ouverture.tsx` — 284

Pages juridiques — du texte dense, dont chaque caractère doit survivre :

- [ ] `MentionsLegales.tsx` — 153
- [ ] `SuppressionCompte.tsx` — 154
- [ ] `Cgv.tsx` — 229
- [ ] `Page.tsx` — 246 (la coquille commune)
- [ ] `Cgu.tsx` — 300
- [ ] `Confidentialite.tsx` — 456

Et la chaîne de construction, `apps/web/build.ts`, 700 lignes dont une
quinzaine de gardes à reprendre : adresses canoniques, `hreflang`, plan du
site, refus du domaine de remplacement, refus des mentions légales
incomplètes, mesure du poids compressé.

## Comment le portage se vérifie

Pas à l'œil. Les deux chaînes rendent le même site : on compare.

1. `apps/web` construit dans `dist/`, `apps/site` dans `dist-rs/`.
2. Pour chaque page et chaque langue, on extrait le texte visible et on le
   compare. C'est ce qui prouve qu'aucun mot de CGU n'a disparu.
3. On compare aussi la structure : titres, `id`, `href`, attributs `aria`.

Un `style` perdu ne se voit PAS dans une comparaison de texte : ma première
version de `composants.rs` posait `style="color-mix(…)"` sans nom de
propriété, et les sections alternées auraient perdu leur fond en silence. La
comparaison porte donc aussi sur les attributs.

## Les tests de contrat à rebrancher

Une douzaine de tests Rust lisent aujourd'hui les sources TypeScript du site :
`identite.ts`, `documents.ts`, les pages juridiques. Ils devront lire les
sources Rust, ou disparaître quand la génération les rend inutiles.

Celui des mentions légales, en particulier, n'a pas d'équivalent : c'est lui
qui refuse une mise en ligne sans SIREN.
