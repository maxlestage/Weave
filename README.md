# Weave

> Trois fils par jour. Rien de plus.

Weave est une application de rencontre construite sur une contrainte : **au plus
trois profils à la fois, pour tout le monde**, en cache uniquement, pendant
vingt-quatre heures. On n'engage pas un fil par un geste, on y répond par écrit.

Ce dépôt contient l'API, l'application iOS et watchOS, et le site de présentation.

## Ce qu'il faut savoir en trois points

1. **Le plafond de trois est un invariant, pas un réglage.** Il est appliqué de
   façon atomique côté serveur par un script Lua dans Redis, et réappliqué à la
   réception côté client. Aucun palier d'abonnement ne le relève.
2. **Les profils proposés n'existent qu'en cache.** La base ne contient aucune
   copie d'un profil proposé — seulement un registre d'identifiants, pour ne
   jamais reproposer la même personne.
3. **Bun remplace Node.js de bout en bout**, y compris là où il a fallu écrire
   l'outillage manquant (voir `packages/prisma-bun-sqlite`).

## La pile

| | Choix | Version |
| --- | --- | --- |
| Exécution | Bun | 1.3 |
| Backend | Elysia | 1.4 |
| ORM | Prisma | 7 |
| Base | PostgreSQL (production) · SQLite (développement) | — |
| Cache | Redis, via le client natif de Bun | — |
| Site | React · Vite · Tailwind | 19 · 8 · 4 |
| Applications | Swift · SwiftUI · ActivityKit · WidgetKit | 6.2 |

Toutes les dépendances viennent de leurs canaux officiels : registre npm pour
JavaScript, gestionnaire de paquets Swift pour iOS, images Docker officielles
pour PostgreSQL et Redis. Aucun miroir, aucun fork.

## Démarrer

```sh
# 1. Redis et PostgreSQL en local (optionnel : SQLite suffit pour développer)
bun run infra:up

# 2. Dépendances, schéma, base de développement, jeu de données
bun run setup

# 3. L'API et le site, ensemble
bun run dev
```

- API : http://localhost:3000 — documentation OpenAPI sur `/openapi`
- Site : http://localhost:5173

L'API démarre par défaut sur SQLite. Pour PostgreSQL :

```sh
cd apps/api
cp .env.example .env          # renseignez DATABASE_URL
WEAVE_DB=postgres bun run db:deploy
WEAVE_DB=postgres bun run dev
```

SQLite est refusé en production par une garde explicite : ce n'est pas une
convention, c'est une erreur au démarrage.

## Tests

```sh
bun test                      # API + adaptateur Prisma (36 tests)
cd apps/ios/WeaveKit && swift test
```

Les tests d'intégration de l'API passent par les vraies routes HTTP, le vrai
cache Redis et la vraie base de développement. Ils vérifient notamment que le
plafond de trois fils tient sous appels concurrents, et qu'aucun contenu de
profil proposé ne se retrouve en base.

## Structure

```
weave/
├── apps/
│   ├── api/        Elysia · Prisma · Redis · APNs
│   ├── web/        Site vitrine, pensé pour le téléphone d'abord
│   └── ios/        iPhone · Live Activity · Apple Watch
├── packages/
│   ├── contracts/            invariants, catalogue des offres, types partagés
│   └── prisma-bun-sqlite/    adaptateur Prisma pour bun:sqlite
├── scripts/
└── docs/
```

## Documentation

| Document | Contenu |
| --- | --- |
| [PRODUIT.md](docs/PRODUIT.md) | Le concept, le vocabulaire, les cinq règles |
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | Les choix techniques et leurs raisons |
| [CACHE.md](docs/CACHE.md) | « Trois profils en cache uniquement », en détail |
| [MONETISATION.md](docs/MONETISATION.md) | Quatre abonnements, et tout à l'unité |
| [ORIGINALITE.md](docs/ORIGINALITE.md) | Ce qui éloigne Weave des mécaniques existantes |
| [CONFORMITE.md](docs/CONFORMITE.md) | RGPD, DSA, sécurité des personnes |
| [apps/ios/README.md](apps/ios/README.md) | Générer le projet Xcode, les deux jetons ActivityKit |

## Commandes utiles

| Commande | Effet |
| --- | --- |
| `bun run dev` | API et site en parallèle |
| `bun run test` | Toute la suite TypeScript |
| `bun run typecheck` | Vérification des types de tous les paquets |
| `bun run db:sqlite` | Dérive le schéma SQLite depuis le schéma PostgreSQL |
| `bun run db:generate` | Génère les deux clients Prisma |
| `bun run db:migrate` | Nouvelle migration (développement) |
| `bun run db:deploy` | Applique les migrations |
| `bun run db:seed` | Jeu de données de développement |
| `bun run format` | Formatage Prettier |

## Reste à faire avant un lancement

Le socle est fonctionnel et testé ; ces points demandent des décisions ou des
comptes tiers, pas du code d'architecture :

- [ ] Envoi réel des codes de connexion (fournisseur d'e-mail transactionnel)
- [ ] Stockage objet des médias et application effective du flou côté stockage
- [ ] Vérification cryptographique complète des transactions StoreKit
      (clé App Store Connect)
- [ ] Tâche planifiée : heure de tissage, purges RGPD, expiration des Live Activities
- [ ] Export des données personnelles au format lisible par machine
- [ ] Revue de marque et de brevets (voir [ORIGINALITE.md](docs/ORIGINALITE.md))
