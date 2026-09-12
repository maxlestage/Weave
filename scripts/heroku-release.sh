#!/usr/bin/env bash
#
# Phase de publication.
#
# Depuis le passage à Rust, les migrations sont appliquées par le binaire
# lui-même — `weave-api migrate` —, et le `Procfile` l'appelle directement.
# Ce script reste pour les déploiements par conteneur, que `heroku.yml` décrit.

set -euo pipefail

echo "Weave — phase de publication"

if [ -z "${DATABASE_URL:-}" ]; then
  echo "  DATABASE_URL absent : ajoutez l'add-on Heroku Postgres." >&2
  exit 1
fi

echo "  • application des migrations"
./bin-release/weave-api migrate
echo "  • terminé"
