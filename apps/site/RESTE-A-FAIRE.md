# Reprise du front en Yew — où en est le travail

Le front React (`apps/web`) est en cours de remplacement par Yew
(`apps/site`). **Rien n'est débranché** : `apps/web` construit toujours le
site livré, et le fera jusqu'à ce que le portage soit complet et comparé.

## Ce qui est éprouvé

| | |
|---|---|
| Yew | 0.23.0 — la dernière publiée (mars 2026), MSRV 1.84 |
| Rendu | `ServerRenderer` natif, aucune cible wasm, aucun `trunk` |
| Marqueurs d'hydratation | actifs ; leur coût est mesuré à chaque rendu |
| Contrat partagé | **engendré** depuis `packages/contracts`, pas recopié |
| Textes de palier | engendrés eux aussi, dans les trois langues |
| Catégories de plan | engendrées ; `Categorie` est une énumération |
| Pages juridiques | engendrées depuis `documents.ts` — adresses, libellés, dates |
| Année du droit d'auteur | figée par `build.rs`, donc la même aux deux rendus |
| Droits (demandes, horizon, critères) | engendrés ; `Criteres` est une énumération |
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

## Tout le site est porté, et prouvé

**81 comparaisons, 0 divergence** : les trois accueils et les cinq pages
juridiques, corps et coquille, texte et attributs.

## La page d'accueil

Son **corps entier** est identique à celui de React — 1 279 nœuds en français,
1 291 en anglais et en espagnol, attributs compris :

- [x] `Entete.tsx` — l'en-tête, et le seul état du site
- [x] `Ouverture.tsx` — section `#haut`
- [x] `Principe.tsx` — section `#principe`
- [x] `Deroule.tsx` — section `#deroule`
- [x] `Appareils.tsx` — section `#montre`
- [x] `Offres.tsx` — section `#offres`
- [x] `Confiance.tsx` — section `#confiance`
- [x] `Questions.tsx` — section `#questions`
- [x] `PiedDePage.tsx` — le pied de page

Et sa **coquille HTML** l'est aussi : 27 nœuds identiques, titre, description,
Open Graph et données structurées comprises. Les textes viennent de
`metadonnees.rs`, engendré depuis `apps/web/src/pages/metadonnees.ts`.

Deux choses seulement diffèrent encore, et aucune n'est du contenu :

- **Le script qui charge l'application.** React charge `/chunk-w0zhmwrj.js`, Yew
  `/demarrer.js`. Les deux chaînes chargent forcément deux paquets différents —
  c'est l'objet du remplacement. Il n'est pas ignoré pour autant : chaque côté
  doit avoir exactement une balise de module avec une adresse, et celle de Yew
  doit désigner un fichier qui existe. C'est plus sévère qu'une comparaison, et
  cela attrape la panne qu'on ne voit pas : une page qui s'affiche très bien et
  ne s'hydrate jamais.
- **Les noms empreints de la feuille de style et de l'icône**
  (`chunk-5jqgjvep.css`). La chaîne Rust ne sait pas recalculer ces empreintes :
  il faudrait refaire le même empaquetage, au même octet. Elle les **lit** donc
  dans `apps/web/dist`. C'est un étai, dit comme tel dans `rendre.rs`, et il
  disparaît le jour où la chaîne Rust produira elle-même l'habillage.

## Les pages juridiques sont portées

Du texte dense, dont chaque caractère devait survivre. Il a survécu :

| page | corps | coquille |
|---|---|---|
| `Confidentialite.tsx` — 456 lignes | **920 nœuds identiques** | 18 |
| `Cgv.tsx` — 229 | **579 identiques** | 18 |
| `Cgu.tsx` — 300 | **542 identiques** | 18 |
| `MentionsLegales.tsx` — 153 | **399 identiques** | 18 |
| `SuppressionCompte.tsx` — 154 | **357 identiques** | 18 |
| `Page.tsx` — 246 (la charpente) | — | — |

Les tableaux compris : celui des données de la politique de confidentialité
fait quatorze lignes et quatre colonnes, et ses 237 nœuds coïncident.

Ces pages n'embarquent **aucun script**, des deux côtés. Elles n'ont pas
d'état, donc rien à hydrater ; leur envoyer un script ferait télécharger cent
kilo-octets de wasm à qui vient lire des CGU. L'outil l'exige : zéro balise de
module de chaque côté, et un script apparu là voudrait dire que la page a cessé
d'être statique.

**Les quinze valeurs à compléter restent surlignées**, et viennent d'un
`identite.rs` engendré depuis `apps/web/src/pages/identite.ts` — le seul fichier
à remplir. Une mention légale qui annoncerait un SIREN différent de celui du
site React serait une fausse déclaration, et personne ne s'en apercevrait,
puisqu'elle aurait l'air complète.

## Ce qui reste : la chaîne de construction

Les huit pages du site sont portées et prouvées identiques — **81
comparaisons, 0 divergence**. Ce qui reste n'est plus du contenu, c'est
`apps/web/build.ts` : 700 lignes dont une quinzaine de gardes.

- l'**habillage empreint** : la feuille de style et l'icône portent une
  empreinte de contenu que la chaîne Rust ne sait pas recalculer. Elle les lit
  dans `apps/web/dist`, et c'est le dernier étai ;
- `robots.txt`, `site.webmanifest`, les images de partage ;
- les **adresses canoniques** et les `hreflang`, qui n'apparaissent que si
  `SITE_ORIGINE` est posé ;
- le **plan du site** ;
- le **refus du domaine de remplacement** : mieux vaut ne rien déclarer que de
  désigner une adresse morte, qu'un moteur suivrait pour désindexer les pages ;
- le **refus des mentions légales incomplètes** : c'est lui qui empêche une
  mise en ligne sans SIREN, et il n'a pas d'équivalent ailleurs.

## Comment le portage se vérifie

Pas à l'œil. Les deux chaînes rendent le même site, donc on compare — et c'est
fait : `outils/comparer.ts`, branché dans `verifier.sh`.

    bun run apps/site/outils/comparer.ts

Il relève de chaque page les balises, leurs attributs et leur texte, les
découpe par section, et compare section par section et langue par langue.
Celles que Yew ne rend pas encore sont listées, pas comptées comme des fautes :
l'outil est utile pendant le portage, et non seulement à la fin.

Et dès qu'aucune section ne manque plus, il compare **la page entière** —
`<body>` d'un côté, `<head>` de l'autre. C'est la seule comparaison qui voie
l'en-tête, le pied de page, le lien d'évitement et l'ORDRE des sections : la
comparaison par section rapproche chaque section de sa jumelle où qu'elle soit,
et ne dirait rien si on les permutait. Elle s'allume d'elle-même, sans qu'on
ait à y penser le bon jour.

`<head>` et `<body>` sont comptés à part : la coquille vient de la chaîne de
construction, l'arbre du corps des composants. Les mêler ferait d'un `hreflang`
manquant et d'un pied de page manquant la même ligne, alors que ce ne sont pas
les mêmes fichiers à reprendre.

Deux angles morts ont été trouvés dans l'outil lui-même en l'étendant à la page
entière, et tous deux rendaient un « identique » faux :

- **les `<script>` disparaissaient du relevé, balise comprise.** React termine
  le corps par le script qui hydrate la page. Une adresse fausse, ou la balise
  absente, donne une page qui ne s'hydratera jamais — et l'outil annonçait
  « identique ». On relève désormais la balise et ses attributs, et on ne tait
  que le corps : du JavaScript minifié n'a pas à être comparé ligne à ligne.
- **les données structurées étaient tues avec le reste des `<script>`.**
  `application/ld+json` ne contient pas du code mais des données — nom, langue,
  description, prix — qu'un moteur lit et affiche. Le prix annoncé aux moteurs
  pouvait passer de 0 à autre chose sans que rien ne s'en plaigne.

Un `style` perdu ne se voit PAS dans une comparaison de texte : ma première
version de `composants.rs` posait `style="color-mix(…)"` sans nom de
propriété, et les sections alternées auraient perdu leur fond en silence. La
comparaison porte donc aussi sur les attributs — et l'outil a été éprouvé en
cassant exprès cinq choses :

| ce qu'on casse | ce que l'outil dit |
|---|---|
| deux mots retirés d'une étape | texte DIFFÉRENT, à la bonne phrase |
| `style="color: …"` devenu `style="…"` | texte identique, structure différente |
| une classe utilitaire perdue | texte identique, structure différente |
| un `aria-hidden` retiré | texte identique, structure différente |
| une `<Etiquette>` entière absente | texte DIFFÉRENT |
| un `aria-current` retiré du choix de langue | le corps de la page diverge |
| une ancre de pied de page sans préfixe de langue | le corps diverge (`/#principe` contre `#principe`) |
| l'avis « documents en français seulement » retiré | **le français reste identique**, l'anglais et l'espagnol divergent |
| deux mots retirés de la description de référencement | la coquille diverge, sur la bonne balise |
| le prix des données structurées passé de 0 à 4,99 | la coquille diverge |
| le fichier `demarrer.js` jamais écrit | « Yew charge « /demarrer.js », et ce fichier n'existe pas » |
| la balise de script retirée | « React en a 1, Yew 0 » |

Le dernier mérite un mot : cet avis n'existe pas en français — il n'y a rien à
prévenir à qui lit déjà la langue du texte. L'outil le sait sans qu'on le lui
dise, parce qu'il compare chaque langue séparément.

Et il a lui-même été pris en défaut deux fois en une heure : il décodait les
entités HTML d'un côté seulement — React échappe l'apostrophe, Yew non — et il
relevait le nom des balises fermantes dans un rappel où `tagName` ne vaut plus
rien. Les fermetures s'appelaient donc toutes `undefined`, ce qui ne faisait
pas échouer la comparaison (les deux relevés se trompaient pareil) mais
empêchait de trouver la fin d'une section : chacune s'étendait jusqu'au bas de
la page, et l'outil signalait des divergences inventées.

### Ce que la comparaison a trouvé du premier coup

`Offres` était notée faite plus haut dans ce fichier. Elle ne l'était pas :
c'était une liste de paliers simplifiée, sans le tableau des droits, sans les
produits à l'unité, sans les accroches traduites, avec un mauvais fil de
couleur et un fond alterné que React ne pose pas. Le titre affiché était « Les
offres » quand React dit « Quatre abonnements, et tout à l'unité ».

La comparaison l'a dit à sa première exécution. C'est exactement pour cela
qu'elle vient avant les sept sections restantes, et non après.

## Les tests de contrat à rebrancher

Une douzaine de tests Rust lisent aujourd'hui les sources TypeScript du site :
`identite.ts`, `documents.ts`, les pages juridiques. Ils devront lire les
sources Rust, ou disparaître quand la génération les rend inutiles.

Celui des mentions légales, en particulier, n'a pas d'équivalent : c'est lui
qui refuse une mise en ligne sans SIREN.
