#!/usr/bin/env bash
# Phase de publication Heroku : applique les migrations avant que la nouvelle
# version ne reçoive du trafic.
#
# En cas d'échec, Heroku interrompt le déploiement et la version précédente
# reste en ligne — c'est exactement le comportement voulu pour une migration
# de base.
set -euo pipefail

echo "Weave — phase de publication"

if [ -z "${DATABASE_URL:-}" ]; then
  echo "  DATABASE_URL absent : ajoutez l'add-on Heroku Postgres." >&2
  exit 1
fi

echo "  • application des migrations"
cd apps/api

# Le CLI Prisma est une dépendance de PRODUCTION : la phase de publication
# s'exécute après l'élagage des dépendances de développement, et il aurait
# disparu au moment précis où l'on en a besoin.
WEAVE_DB=postgres npx --no-install prisma migrate deploy

echo "  • terminé"
