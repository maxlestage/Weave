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
#
# Il est lancé PAR BUN, jamais directement : le script d'entrée de Prisma porte
# un en-tête `#!/usr/bin/env node`, que le noyau résout en cherchant un binaire
# `node`. Or il n'y en a pas dans le slug, et c'est voulu — Bun remplace
# Node.js de bout en bout. Bun, lui, sait exécuter un tel fichier.
if [ -x ./node_modules/.bin/prisma ]; then
  prisma_bin="bun ./node_modules/.bin/prisma"
elif [ -x ../../node_modules/.bin/prisma ]; then
  prisma_bin="bun ../../node_modules/.bin/prisma"
else
  prisma_bin="bunx prisma"
fi

WEAVE_DB=postgres $prisma_bin migrate deploy

echo "  • terminé"
