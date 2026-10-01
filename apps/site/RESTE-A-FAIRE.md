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

## Le wasm et l'hydratation, mesurés

Demandés, construits, et voici ce qu'ils pèsent réellement — une seule section
portée, la fondation complète :

| | gzip |
|---|---|
| `weave_site_bg.wasm` | 98 711 |
| `weave_site.js` (colle wasm-bindgen) | 6 357 |
| **total téléchargé en plus** | **105 068** |
| Pour mémoire, le paquet React remplacé | 85 237 |

Soit **+23 %**. Et ce total est un plancher : il est surtout fait du coût fixe
de Yew, mais les treize fichiers de contenu restants s'y ajouteront.

Le HTML hydratable coûte en plus **745 octets de marqueurs par page** —
mesurés en rendant deux fois le même arbre, avec et sans.

### `wasm-opt` n'est pas lancé, et c'est mesuré

`-Oz`, `-Os` et `-O2` réduisent tous le fichier brut — 273 Kio tombent à 240 —
mais ils **augmentent** le compressé : 98 730 octets deviennent 100 386, 100 554
et 101 378. Ils réorganisent le code d'une façon qui compresse moins bien, et
c'est le compressé que le visiteur télécharge.

### Ce que le wasm apporte, et ce qu'il n'apporte pas

Il n'accélère pas l'affichage : la page est rendue en HTML et lisible avant
qu'un octet de wasm n'arrive. L'hydratation vient après, et ajoute du travail.

Ce qu'il apporte est une **capacité** : de l'état côté client. Aujourd'hui elle
sert au seul menu de téléphone ; elle sera là le jour où une page en demandera
davantage.

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
