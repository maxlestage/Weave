# Modèle économique

Freemium : un socle gratuit, **quatre abonnements**, et **chaque avantage
également achetable à l'unité**.

Le catalogue est défini une seule fois, dans
[`packages/contracts/src/catalog.ts`](../packages/contracts/src/catalog.ts), et
consommé par l'API, le site et l'application. Il n'y a pas de tarif écrit deux
fois.

## La règle qui tient tout

> **Aucun palier n'augmente le nombre de fils.**

Douze pour tout le monde, du gratuit au plus cher. Ce qui se vend :

- la **vitesse de regarnissage** d'une place libérée ;
- la **finesse des critères** de composition ;
- des **facilités** ponctuelles — prolonger un fil, en rappeler un dénoué.

Vendre du volume reviendrait à défaire le produit. C'est la contrainte
commerciale la plus forte du projet, et elle est assumée : elle plafonne le
revenu par utilisateur, mais c'est ce plafond qui rend l'offre crédible.

## Les paliers

| | **Fil** | **Trame** | **Chaîne** | **Navette** | **Métier** |
| --- | --- | --- | --- | --- | --- |
| Prix mensuel | Gratuit | 6,99 € | 12,99 € | 19,99 € | 34,99 € |
| Prix annuel | — | 59,90 € | 109,90 € | 169,90 € | 299,90 € |
| **Fils actifs** | **3** | **3** | **3** | **3** | **3** |
| Regarnissage | prochaine heure | 6 h | 3 h | 1 h | 15 min |
| Critères | de base | étendus | précis | précis | précis |
| Fragments vocaux | — | ✓ | ✓ | ✓ | ✓ |
| Écho / mois | 0 | 1 | 3 | 6 | 15 |
| Prolonge / mois | 0 | 2 | 5 | 10 | 30 |
| Escale / mois | 0 | 0 | 1 | 2 | 4 |
| Accusé de lecture | — | — | ✓ | ✓ | ✓ |
| Rapport Atelier | — | — | — | ✓ | ✓ |
| Assistance prioritaire | — | — | — | — | ✓ |

## À l'unité

Consommables StoreKit, sans engagement. On peut utiliser Weave des mois durant
sans jamais s'abonner.

| Produit | Prix | Ce que ça fait |
| --- | --- | --- |
| **Écho** | 2,49 € | Rappeler une fois un fil laissé se dénouer |
| **Prolonge** | 1,49 € | +24 h sur un fil en cours, une fois par fil |
| **Relais** | 1,99 € | Regarnir une place tout de suite, sans attendre son délai |
| **Motif** | 0,99 € | Retisser ses cinq mots à partir de nouvelles réponses |
| **Escale** | 4,99 € | Tisser depuis une autre ville pendant sept jours |
| **Atelier** | 3,49 € | Un rapport ponctuel sur la résonance de ses fragments |

Le plafond de 48 h de vie d'un fil s'applique quel que soit le nombre de
« Prolonge » achetées : on ne peut pas retenir indéfiniment quelqu'un.

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

## Refus explicites

Quand une action demande un crédit absent, l'API répond `402` avec de quoi
proposer l'achat — le SKU, l'identifiant StoreKit, le prix, et les paliers qui
l'incluent. Le client n'a pas à deviner quoi proposer, et n'a pas à coder en dur
un tarif.
