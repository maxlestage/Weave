# Types de processus Heroku.
#
# Le buildpack officiel `heroku/nodejs` fait tout le travail : il lit `engines`
# dans package.json, installe les dépendances, exécute `heroku-postbuild`, puis
# élague les dépendances de développement. Weave n'a plus de buildpack maison.

# Appliquée avant que la nouvelle version ne reçoive du trafic. Si elle échoue,
# Heroku interrompt la publication et l'ancienne version reste en ligne.
release: bash scripts/heroku-release.sh

web: node apps/api/src/index.ts
