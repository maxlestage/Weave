# Modèle économique

Freemium : un socle gratuit, **quatre abonnements**, et **chaque avantage
également achetable à l'unité** — parce qu'à vingt ans, on ne s'abonne pas à
tout, et qu'un produit qui l'exige se prive de la moitié de son public.

Le catalogue est défini une seule fois, dans
[`packages/contracts/src/catalog.ts`](../packages/contracts/src/catalog.ts), et
consommé par l'API, le site et l'application. Il n'y a pas de tarif écrit deux
fois.

## La règle qui tient tout

> **Aucune offre n'achète de visibilité.**

Le fil est trié par imminence puis par proximité, et par rien d'autre. Aucun
palier, aucun achat ne place un plan devant celui de quelqu'un d'autre. Ce qui
se vend :

- l'**horizon de publication** — combien de jours à l'avance on peut poser un
  plan ;
- la **finesse des critères** du fil ;
- les **plans de groupe**, jusqu'à quatre personnes ;
- des **à-côtés** — publier depuis une autre ville, un bilan de ses plans.

Le second invariant tient aussi face à l'argent : **le nombre de demandes par
jour reste borné à tous les paliers**. Les paliers l'élèvent, un « Renfort »
l'assouplit au plus deux fois par jour, mais il n'existe aucune façon d'envoyer
la même phrase à cent personnes dans la journée.

C'est la contrainte commerciale la plus forte du projet, et elle est assumée :
elle plafonne le revenu par utilisateur, mais c'est ce plafond qui rend l'offre
crédible.

## Les paliers

| | **Départ** | **Virée** | **Escapade** | **Expédition** | **Grand Tour** |
| --- | --- | --- | --- | --- | --- |
| Prix mensuel | Gratuit | 4,99 € | 8,99 € | 14,99 € | 24,99 € |
| Prix annuel | — | 44,90 € | 79,90 € | 129,90 € | 209,90 € |
| **Remontée dans le fil** | **aucune** | **aucune** | **aucune** | **aucune** | **aucune** |
| Demandes / jour | 5 | 12 | 25 | 40 | 60 |
| Plans ouverts | 3 | 3 | 3 | 3 | 3 |
| Publier à l'avance | 7 j | 14 j | 30 j | 60 j | 90 j |
| Critères | de base | étendus | précis | précis | précis |
| Plans de groupe | — | ✓ | ✓ | ✓ | ✓ |
| Escale / mois | 0 | 0 | 1 | 2 | 4 |
| Bilan | — | — | — | ✓ | ✓ |
| Assistance prioritaire | — | — | — | — | ✓ |

Les prix sont volontairement bas pour le secteur : le public visé a vingt ans et
paie ses courses. Un abonnement de rencontre à 30 € par mois ne lui est pas
destiné, quoi qu'en dise le marché.

## À l'unité

Consommables StoreKit, sans engagement. On peut utiliser Weave des mois durant
sans jamais s'abonner.

| Produit | Prix | Ce que ça fait |
| --- | --- | --- |
| **Renfort** | 1,49 € | Cinq demandes de plus aujourd'hui |
| **Horizon** | 0,99 € | Publier un plan au-delà de son horizon, une fois |
| **Tablée** | 1,49 € | Un plan de groupe, une fois |
| **Escale** | 3,99 € | Publier et lire depuis une autre ville pendant sept jours |
| **Bilan** | 2,49 € | Un rapport ponctuel : quels plans attirent, et pourquoi |

Il n'existe **aucun produit de remontée** — ni « boost », ni « relance », ni
mise en avant. C'est la seule case du catalogue que le secteur remplit toujours
et que Weave laisse vide. Un test de `WeaveKit` vérifie qu'aucun SKU portant ce
genre de nom n'apparaît par inadvertance.

### Le plafond du Renfort

Un « Renfort » ajoute `RENFORT_GRANT` demandes à la journée en cours, **au plus
`MAX_RENFORTS_PER_DAY` fois par jour**. Ce second plafond n'est pas une
limitation commerciale timide : sans lui, « on ne peut pas arroser » deviendrait
« on ne peut pas arroser gratuitement », ce qui n'est pas la même règle.

La place de renfort est réservée avant que le crédit soit dépensé, et rendue si
le crédit manque — sinon une tentative refusée grignoterait le plafond du jour.
Un test d'intégration couvre les deux cas.

## Mise en œuvre

- **StoreKit 2** côté application. Aucun moyen de paiement ne transite par nos
  serveurs.
- Le client transmet la transaction signée à `POST /v1/billing/subscriptions`
  ou `POST /v1/billing/units`.
- En production, la charge utile JWS est vérifiée auprès de l'App Store Server
  API. Si la configuration Apple est absente **en production**, l'achat est
  refusé plutôt qu'accepté à l'aveugle.
- L'unicité de `transactionId` en base empêche qu'un même achat soit crédité
  deux fois.
- Les notifications serveur à serveur (V2) arrivent sur
  `POST /v1/billing/apple/notifications` : renouvellement, expiration,
  remboursement, période de grâce.
- Les crédits inclus dans un abonnement sont redotés au renouvellement ; les
  crédits **achetés à l'unité ne sont jamais remis à zéro**.
- La dépense d'un crédit passe par un `updateMany` conditionnel
  (`balance >= 1`) : deux requêtes simultanées ne peuvent pas dépenser le même
  crédit.

## Refus explicites

Quand une action demande un crédit absent, l'API répond `402` avec de quoi
proposer l'achat — le SKU, l'identifiant StoreKit, le prix, et les paliers qui
l'incluent. Le client n'a pas à deviner quoi proposer, et n'a pas à coder en dur
un tarif.
