#!/usr/bin/env bash
#
# Étape de construction appelée par le buildpack, après l'installation des
# dépendances et AVANT le script `build`.
#
# Trois voies mènent Weave en production, et elles n'appellent pas les mêmes
# choses :
#
#   heroku-community/inline → `bin/compile` fait tout lui-même
#   un buildpack Bun tiers  → `bun install`, puis ce script, puis `build`
#   conteneur               → les étapes sont écrites dans le Dockerfile
#
# Rien dans `build` n'engendre les clients Prisma, et `bun build` de l'API en a
# besoin : sans eux cette étape échoue, et le dyno s'arrête au démarrage sur
# « Cannot find module ../generated/prisma/client.ts » — un message qui ne
# désigne pas l'étape manquante.
#
# D'où le choix de `heroku-prebuild` plutôt que `heroku-postbuild` : les
# clients doivent exister AVANT que `build` ne s'exécute.
#
# Il est idempotent : la voie inline le rejoue sans dommage.

set -euo pipefail

if ! command -v bun >/dev/null 2>&1; then
  # Le buildpack Node.js officiel appelle aussi ce script, sans que Bun existe.
  # Échouer ici masquerait le vrai problème — c'est un buildpack Bun, ou
  # l'inline, qu'il faut configurer — et casserait une construction que le
  # buildpack suivant sauverait peut-être.
  echo "Bun absent : étape ignorée. Voir docs/DEPLOIEMENT.md pour le buildpack à configurer."
  exit 0
fi

echo "Clients Prisma"
bun run db:sqlite
bun run db:generate
