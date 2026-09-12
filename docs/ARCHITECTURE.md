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
        │        API Weave — Rust + Axum            │
        │  composition du fil · droits · APNs       │
        └───────┬───────────────────────┬───────────┘
                │                       │
        ┌───────▼────────┐      ┌───────▼──────────┐
        │     Redis      │      │   PostgreSQL     │
        │ QUOTA DU JOUR  │      │  comptes, plans  │
        │  fil composé   │      │  demandes, msgs  │
        └────────────────┘      └──────────────────┘

        ┌────────────────┐
        │  Site vitrine  │  React, bundlé par Bun — présentation
        │  (mobile first)│  servi par le même binaire
        └────────────────┘
```

## Choix techniques, et pourquoi

### L'API en Rust, le reste en Bun

L'API est un binaire Rust autonome : Axum pour le routage, SeaORM pour l'accès
aux données, un client Redis natif. C'est lui que le dyno exécute, et il ne
dépend d'aucun environnement d'exécution à installer à côté.

Bun reste le gestionnaire de paquets et le bundler du site vitrine, et sert la
chaîne de développement. Le site et l'API ne partagent que
[`@weave/contracts`](../packages/contracts) — des constantes, pas du code.

### Les migrations sont du SQL, et rien d'autre

`apps/api-rs/migrations/` pour PostgreSQL, `apps/api-rs/migrations-sqlite/` pour
SQLite : deux jeux de fichiers `.sql` versionnés, appliqués par
`weave-api migrate` avant que la nouvelle version ne reçoive du trafic.

Le binaire tient son journal dans la table `_prisma_migrations`, au format que
Prisma utilisait. Ce n'est pas de la nostalgie : la base de production porte
déjà cet état, et le relire évite de rejouer des migrations déjà appliquées.

Chaque dialecte a son propre jeu. Servir le SQL de PostgreSQL à SQLite avait
déjà fait échouer une publication, sur un `near "(": syntax error` — un test
garde ce cas.

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
│   ├── api-rs/  Axum · SeaORM · Redis · APNs — le binaire du dyno
│   ├── web/     React 19 · Tailwind 4, bundlés par Bun — vitrine
│   └── ios/     Swift 6.2 · SwiftUI · ActivityKit · watchOS
├── packages/
│   └── contracts/   invariants, catalogue, types partagés
└── docs/
```

### `@weave/contracts` : une seule définition

Les invariants (`REQUESTS_PER_DAY_FLOOR`, `PAID_VISIBILITY`, `MAX_OPEN_PLANS`),
le catalogue des offres et les types de transport sont définis une fois et
consommés par l'API **et** par le site. Le site ne peut donc pas afficher un
tarif ou un plafond que l'API n'applique pas.

Côté Rust comme côté Swift, les mêmes valeurs sont redéfinies à la main —
on ne partage pas de types entre TypeScript, Rust et Swift. Les
tests de `WeaveKit` décodent des charges utiles réelles de l'API pour vérifier
que le miroir n'a pas dérivé.

## La composition du fil

`apps/api-rs/src/routes/fil.rs`

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

`apps/api-rs/src/cache.rs` et `apps/api-rs/src/routes/requests.rs`

Le compteur de demandes du jour vit dans Redis, expire à minuit dans le fuseau
de la personne, et se décrémente par un script Lua atomique. Le quota est
prélevé **avant** l'écriture de la demande, et rendu si l'insertion échoue.

Toutes les lectures passent par `quota_journalier()`, qui tient compte des
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
