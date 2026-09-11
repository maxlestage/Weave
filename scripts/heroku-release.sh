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

# Le binaire local est préféré à `bunx`, qui peut tenter d'aller chercher le
# paquet sur le réseau — au moment précis où l'on veut le moins en dépendre.
if [ -x ./node_modules/.bin/prisma ]; then
  prisma_bin=./node_modules/.bin/prisma
elif [ -x ../../node_modules/.bin/prisma ]; then
  prisma_bin=../../node_modules/.bin/prisma
else
  prisma_bin="bunx prisma"
fi

WEAVE_DB=postgres $prisma_bin migrate deploy

echo "  • terminé"
