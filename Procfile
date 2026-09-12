# Types de processus Heroku.
#
# L'API est un binaire Rust autonome, construit par `bin/compile`. Le site
# vitrine est servi par le même processus : il est statique et peu visité, lui
# dédier un second dyno doublerait la facture sans rien apporter.

# Appliquée avant que la nouvelle version ne reçoive du trafic. Si elle échoue,
# Heroku interrompt la publication et l'ancienne version reste en ligne.
release: ./bin-release/weave-api migrate

web: ./bin-release/weave-api
