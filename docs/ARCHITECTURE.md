# Architecture

## Vue d'ensemble

```
┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│   iPhone     │   │ Live Activity│   │ Apple Watch  │
│   SwiftUI    │   │  WidgetKit   │   │  + cadran    │
└──────┬───────┘   └──────┬───────┘   └──────┬───────┘
       │                  │                  │
       └──────────┬───────┴──────────────────┘
                  │  HTTPS + JWT          ▲
                  ▼                       │ APNs (liveactivity)
        ┌───────────────────────────────────────────┐
        │        API Weave — Bun + Elysia           │
        │  composition du fil · droits · APNs       │
        └───────┬───────────────────────┬───────────┘
                │                       │
        ┌───────▼────────┐      ┌───────▼──────────┐
        │     Redis      │      │   PostgreSQL     │
        │ QUOTA DU JOUR  │      │  comptes, plans  │
        │  fil composé   │      │  demandes, msgs  │
        └────────────────┘      └──────────────────┘

        ┌────────────────┐
        │  Site vitrine  │  React + Vite — présentation seulement,
        │  (mobile first)│  ne touche pas à l'API
        └────────────────┘
```

## Choix techniques, et pourquoi

### Bun partout, pas de Node.js

Bun 1.3 sert de gestionnaire de paquets, d'exécuteur, de lanceur de tests et de
bundler. Il apporte aussi, en natif, deux briques qui auraient sinon exigé des
dépendances : **un client Redis** (`Bun.RedisClient`) et **un moteur SQLite**
(`bun:sqlite`). Moins de dépendances, c'est moins de surface d'attaque et moins
de mises à jour à suivre.

Une conséquence a demandé du travail : l'adaptateur Prisma officiel pour SQLite
repose sur `better-sqlite3`, un module natif Node.js qui ne se charge pas sous
Bun (`ERR_DLOPEN_FAILED`). Weave embarque donc son propre adaptateur,
[`@weave/prisma-bun-sqlite`](../packages/prisma-bun-sqlite), bâti sur
`bun:sqlite`. Il implémente l'interface publique `SqlMigrationAwareDriverAdapterFactory`
de Prisma et reproduit sa sémantique de conversion (types de colonnes, dates ISO
8601, entiers 64 bits, transactions sérialisées). Vingt tests le couvrent.

En production, rien de tout cela n'intervient : PostgreSQL via `@prisma/adapter-pg`.

### Elysia

Le typage de bout en bout est la raison principale : les schémas de validation
sont aussi les types TypeScript, et la documentation OpenAPI en est dérivée
plutôt que maintenue à côté. Sur Bun, c'est aussi le routeur le plus rapide de
l'écosystème.

### Un schéma Prisma portable, deux moteurs

Le schéma est écrit sans `enum`, sans liste scalaire, sans type natif propre à
un moteur. Seul le bloc `datasource` diffère. `bun run db:sqlite` dérive le
schéma SQLite du schéma PostgreSQL et **refuse la dérivation** si une
construction non portable a été introduite entre-temps.

Prisma 7 compile les requêtes pour un moteur donné : chaque schéma produit son
propre client, et `src/lib/prisma.ts` charge le bon au démarrage selon
`WEAVE_DB`. SQLite est refusé en production par une garde explicite dans
`env.ts`.

### Redis n'est pas un cache d'accélération

Dans la plupart des services, retirer le cache dégrade les performances. Ici, il
emporte l'invariant central : **le quota de demandes du jour n'existe nulle part
ailleurs.** C'est pourquoi la sonde `/health` déclare `cache.required: true` et
bascule en 503 si Redis ne répond pas — un service qui accepterait des demandes
sans pouvoir les compter mentirait sur sa règle principale.

Les plans, eux, vivent en base : ce sont des engagements datés que leurs auteurs
ont écrits. Le fil n'est qu'une vue calculée par-dessus, mise en cache cinq
minutes.

Détails dans [CACHE.md](./CACHE.md).

## Structure

```
weave/
├── apps/
│   ├── api/     Elysia · Prisma · Redis · APNs
│   ├── web/     React 19 · Vite 8 · Tailwind 4 — vitrine
│   └── ios/     Swift 6.2 · SwiftUI · ActivityKit · watchOS
├── packages/
│   ├── contracts/           invariants, catalogue, types partagés
│   └── prisma-bun-sqlite/   adaptateur Prisma pour bun:sqlite
├── scripts/                 dérivation du schéma, passe-plat Prisma
└── docs/
```

### `@weave/contracts` : une seule définition

Les invariants (`REQUESTS_PER_DAY_FLOOR`, `PAID_VISIBILITY`, `MAX_OPEN_PLANS`),
le catalogue des offres et les types de transport sont définis une fois et
consommés par l'API **et** par le site. Le site ne peut donc pas afficher un
tarif ou un plafond que l'API n'applique pas.

Côté Swift, les mêmes structures sont redéfinies dans `WeaveKit/Models` — un
miroir manuel, puisqu'on ne partage pas de types entre TypeScript et Swift. Les
tests de `WeaveKit` décodent des charges utiles réelles de l'API pour vérifier
que le miroir n'a pas dérivé.

## La composition du fil

`apps/api/src/modules/plans.service.ts`

1. **Préfiltre en SQL** — boîte englobante géographique (pas d'extension
   géospatiale : le schéma doit rester portable), fenêtre de dates bornée par
   l'horizon du palier, âge de l'auteur, catégories retenues, comptes bloqués
   dans les deux sens.
2. **Filtre fin en mémoire** — distance orthodromique exacte, places restantes.
   Un plan complet reste visible : le masquer donnerait l'impression qu'il n'y a
   rien, alors qu'il s'y passe justement quelque chose.
3. **Tri à deux termes** — imminence, puis proximité à moins de douze heures
   d'écart. **Aucun troisième terme**, et surtout aucun terme achetable. C'est
   le point du code qu'il faut relire avant d'accepter toute demande
   d'« amélioration du classement ».
4. **Cache cinq minutes**, invalidé à la publication ou à l'annulation d'un plan.

Il n'existe pas de module de « score » : il n'y a rien à scorer. Le fil est une
requête, pas un algorithme de recommandation.

## L'invariant de quota

`apps/api/src/lib/cache.ts` et `apps/api/src/modules/requests.routes.ts`

Le compteur de demandes du jour vit dans Redis, expire à minuit dans le fuseau
de la personne, et se décrémente par un script Lua atomique. Le quota est
prélevé **avant** l'écriture de la demande, et rendu si l'insertion échoue.

Toutes les lectures passent par `dailyRequestQuota()`, qui tient compte des
« Renforts » achetés : calculer `requestsPerDay` seul quelque part afficherait
un compteur faux.

## Sécurité

- **Sans mot de passe** : code à six chiffres, dix minutes, cinq tentatives,
  haché en Argon2id (intégré à Bun).
- **Jetons de rafraîchissement rotatifs**, stockés hachés en SHA-256. Une
  réutilisation est rejetée.
- **Médias sous URL signée**, à durée de vie courte.
- **Localisation arrondie à ~1 km au dépôt**, jamais stockée plus précisément.
  Un plan n'affiche jamais d'adresse dans le fil : une ville, une distance
  arrondie. L'endroit exact se dit dans la conversation, à qui l'on a accepté.
- **Limitation de débit adossée à Redis**, par compte et par adresse IP.

## Observabilité

Journalisation structurée en JSON, une ligne par événement. `Server-Timing` est
activé hors production. La sonde `/health` distingue base et cache.
