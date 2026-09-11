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
        │  moteur de tissage · droits · APNs        │
        └───────┬───────────────────────┬───────────┘
                │                       │
        ┌───────▼────────┐      ┌───────▼──────────┐
        │     Redis      │      │   PostgreSQL     │
        │  LES 12 FILS   │      │  comptes, motifs │
        │  (cache seul)  │      │  conversations   │
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
n'y a plus de produit : **les fils n'existent nulle part ailleurs**. C'est
pourquoi la sonde `/health` déclare `cache.required: true` et bascule en 503 si
Redis ne répond pas — un service qui accepterait des requêtes sans cache
mentirait sur ce qu'il peut faire.

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

Les invariants (`MAX_ACTIVE_THREADS`), le catalogue des offres et les types de
transport sont définis une fois et consommés par l'API **et** par le site. Le
site ne peut donc pas afficher un tarif ou un plafond que l'API n'applique pas.

Côté Swift, les mêmes structures sont redéfinies dans `WeaveKit/Models` — un
miroir manuel, puisqu'on ne partage pas de types entre TypeScript et Swift. Les
tests de `WeaveKit` décodent des charges utiles réelles de l'API pour vérifier
que le miroir n'a pas dérivé.

## Le moteur de tissage

`apps/api/src/modules/loom.service.ts`

1. **Vivier** — les candidats sont préfiltrés en SQL par boîte englobante
   géographique (pas d'extension géospatiale : le schéma doit rester portable),
   puis scorés en mémoire. Le vivier est mis en cache 30 minutes.
2. **Score sur 1000** — proximité de motif (550), proximité géographique (300),
   fraîcheur du profil (150). Aucun terme n'est achetable.
3. **Composition** — pour chaque place libre, la carte est écrite dans Redis par
   le script Lua qui applique le plafond de façon atomique, puis une ligne de
   registre est écrite en base. Si le registre refuse (personne déjà proposée),
   **la carte est retirée du cache** : les deux écritures restent cohérentes.
4. **Regarnissage** — un compte à rebours par personne, dont la durée dépend du
   palier. Le « Relais » l'efface ; il n'augmente jamais le plafond.

## Sécurité

- **Sans mot de passe** : code à six chiffres, dix minutes, cinq tentatives,
  haché en Argon2id (intégré à Bun).
- **Jetons de rafraîchissement rotatifs**, stockés hachés en SHA-256. Une
  réutilisation est rejetée.
- **Médias sous URL signée**, avec le niveau de flou inscrit dans la signature :
  il ne peut pas être contourné côté client.
- **Localisation arrondie à ~1 km au dépôt**, jamais stockée plus précisément.
- **Limitation de débit adossée à Redis**, par compte et par adresse IP.

## Observabilité

Journalisation structurée en JSON, une ligne par événement. `Server-Timing` est
activé hors production. La sonde `/health` distingue base et cache.
