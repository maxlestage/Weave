# Weave

> Des plans, pas des profils.

Weave est une application de rencontre où l'on ne publie pas un profil : on
publie **un plan pour les jours qui viennent** — un mur d'escalade jeudi à 19 h,
un concert vendredi — et les autres demandent à venir, en écrivant pourquoi.

Ce dépôt contient l'API, l'application iOS et watchOS, et le site de présentation.

## Ce qu'il faut savoir en trois points

1. **On ne peut pas arroser.** Le nombre de demandes envoyables par jour est
   borné à tous les paliers, socle gratuit compris. Le compteur vit dans Redis,
   expire à minuit dans le fuseau de la personne, et se décrémente par un script
   Lua atomique.
2. **On ne peut pas acheter de visibilité.** `PAID_VISIBILITY` vaut littéralement
   `false` : le fil est trié par imminence puis par proximité, et par rien
   d'autre. Aucun produit du catalogue ne vend de remontée.
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
bun test                      # API + adaptateur Prisma
cd apps/ios/WeaveKit && swift test
```

Les tests d'intégration de l'API passent par les vraies routes HTTP, le vrai
cache Redis et la vraie base de développement. Ils couvrent les deux invariants :
le quota journalier s'épuise et se rembourse correctement, le plafond de plans
ouverts tient, et un « Renfort » acheté reste lui-même borné par jour.

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
| [PRODUIT.md](docs/PRODUIT.md) | Le concept, le vocabulaire, les deux invariants |
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | Les choix techniques et leurs raisons |
| [CACHE.md](docs/CACHE.md) | Ce que Redis porte, et pourquoi le quota y vit seul |
| [MONETISATION.md](docs/MONETISATION.md) | Quatre abonnements, tout à l'unité, et rien qui vende de la visibilité |
| [ORIGINALITE.md](docs/ORIGINALITE.md) | Ce qui éloigne Weave des mécaniques existantes |
| [CONFORMITE.md](docs/CONFORMITE.md) | RGPD, DSA, sécurité des personnes |
| [DEPLOIEMENT.md](docs/DEPLOIEMENT.md) | Mettre en ligne depuis un téléphone, sans ordinateur |
| [apps/ios/README.md](apps/ios/README.md) | Générer le projet Xcode, les deux jetons ActivityKit |

## Mettre en ligne

Tout se pilote depuis un navigateur, y compris sur téléphone — aucun ordinateur
n'est nécessaire.

- **L'API et le site** partent sur Heroku. Deux étapes suffisent : déposer sa
  clé d'API Heroku dans les secrets du dépôt, puis lancer le workflow
  **« Heroku — mettre en ligne »**, qui crée l'application, la configure et
  déploie d'un seul tenant, dans un conteneur bâti sur l'image officielle Bun.
- **Variante** : pour déployer depuis le tableau de bord Heroku, le dépôt
  embarque son propre buildpack Bun (`bin/compile`), exécuté par le buildpack
  officiel `heroku-community/inline`. Heroku n'en fournit pas pour Bun.
- **L'application iOS** est construite sur un exécuteur macOS loué à la minute
  par GitHub, signée par `fastlane match`, puis envoyée à TestFlight. Posséder
  un Mac n'est donc pas nécessaire ; un compte Apple Developer l'est.

La marche à suivre, étape par étape : **[docs/DEPLOIEMENT.md](docs/DEPLOIEMENT.md)**.

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
- [ ] Stockage objet des médias, avec modération des photos avant publication
- [ ] Vérification cryptographique complète des transactions StoreKit
      (clé App Store Connect)
- [ ] Tâche planifiée : clôture des plans passés, purges RGPD, expiration des
      Live Activities
- [ ] Export des données personnelles au format lisible par machine
- [ ] Revue de marque et de brevets (voir [ORIGINALITE.md](docs/ORIGINALITE.md))
- [ ] Première exécution de la chaîne iOS, qui n'a pas pu être testée ici
      (ni macOS, ni compte Apple Developer disponibles)
