#!/usr/bin/env bash
# Prépare une session de travail : dépendances et cache.
# Idempotent — il peut être relancé sans effet de bord.
set -uo pipefail

cd "$(dirname "$0")/../.." || exit 0

echo "Weave — préparation de la session"

if ! command -v bun >/dev/null 2>&1; then
  echo "  bun introuvable : installez-le depuis https://bun.sh"
  exit 0
fi

echo "  • dépendances"
bun install --silent || echo "    (échec de l'installation, poursuite)"

# Redis est une dépendance dure du produit : sans lui, il n'y a pas de fils.
if ! redis-cli ping >/dev/null 2>&1; then
  if command -v redis-server >/dev/null 2>&1; then
    echo "  • démarrage de Redis"
    redis-server --daemonize yes --save '' --appendonly no >/dev/null 2>&1
  else
    echo "  • Redis absent : les tests d'intégration échoueront (bun run infra:up)"
  fi
fi

# L'API crée son schéma elle-même : `weave-api migrate` applique les migrations
# de `apps/api-rs/migrations`. Rien à préparer ici.
echo "  prêt — bun run dev:web pour le site, cargo run pour l'API"
