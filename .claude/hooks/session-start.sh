#!/usr/bin/env bash
# Prépare une session de travail : dépendances, cache, schéma, base locale.
# Idempotent — il peut être relancé sans effet de bord.
set -uo pipefail

cd "$(dirname "$0")/../.." || exit 0

echo "Weave — préparation de la session"

if ! command -v node >/dev/null 2>&1; then
  echo "  node introuvable : installez Node.js 22.18 ou plus récent"
  exit 0
fi

echo "  • dépendances"
npm install --silent || echo "    (échec de l'installation, poursuite)"

# Redis est une dépendance dure du produit : le quota de demandes ne vit que là.
if ! redis-cli ping >/dev/null 2>&1; then
  if command -v redis-server >/dev/null 2>&1; then
    echo "  • démarrage de Redis"
    redis-server --daemonize yes --save '' --appendonly no >/dev/null 2>&1
  else
    echo "  • Redis absent : les tests d'intégration échoueront (npm run infra:up)"
  fi
fi

echo "  • schéma SQLite"
npm run db:sqlite >/dev/null 2>&1

echo "  • clients Prisma"
npm run db:generate >/dev/null 2>&1 || echo "    (échec de la génération)"

if [ ! -f apps/api/prisma/dev.db ]; then
  echo "  • base de développement et jeu de données"
  (cd apps/api && WEAVE_DB=sqlite npm run db:deploy >/dev/null 2>&1 && WEAVE_DB=sqlite npm run db:seed >/dev/null 2>&1)
fi

echo "  prêt — npm run dev"
