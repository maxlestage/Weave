# Types de processus pour un déploiement par buildpack (pile heroku-24).
#
# Sur la pile `container`, c'est `heroku.yml` qui fait autorité et ce fichier
# est ignoré. Les deux voies de déploiement restent donc utilisables.

# Appliquée avant que la nouvelle version ne reçoive du trafic. Si elle échoue,
# Heroku interrompt la publication et l'ancienne version reste en ligne.
release: bash scripts/heroku-release.sh

web: bun apps/api/src/index.ts
