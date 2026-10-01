#!/usr/bin/env bash
# Tout ce que la CI vérifie, dans le même ordre, sur cette machine.
#
# Il manquait. Les commandes existaient une à une — `bun run typecheck`,
# `cargo test`, `bun run lint` — mais rien ne les enchaînait, et il fallait se
# souvenir de la liste. J'en ai oublié deux : un type TypeScript réécrit sur
# plusieurs lignes est passé en local et tombé sur la CI, sur du formatage
# seul, alors que les trois cent cinquante-deux tests étaient au vert. Trois
# minutes de coureur pour une ligne trop longue.
#
# L'ordre est celui de la CI, et il est délibéré : le plus rapide d'abord. Une
# faute de frappe TypeScript tombe en dix secondes, pas après les vingt de la
# suite Rust.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

etape() {
  printf '\n\033[1m— %s\033[0m\n' "$1"
  shift
  "$@"
}

etape "Types" bun run typecheck
etape "Site" bun run --filter @weave/web build
# Le portage en Yew se vérifie en comparant les deux rendus, section par
# section : c'est la seule façon de savoir qu'aucune phrase n'a disparu en
# route. Le rendu est en débogage et non en production — on compare du HTML,
# pas des temps de compilation.
# L'en-tête de `contrat.rs` annonçait qu'un test vérifiait qu'il est à jour.
# Rien ne le vérifiait.
etape "Site — le contrat engendré est-il à jour" \
  bun run apps/site/outils/engendrer-contrat.ts --verifier

etape "Site — le rendu Yew" \
  cargo run --quiet --manifest-path apps/site/Cargo.toml --features ssr --bin rendre
etape "Site — React et Yew rendent-ils la même chose" \
  bun run apps/site/outils/comparer.ts

etape "API" cargo test --manifest-path apps/api-rs/Cargo.toml
etape "API — compilation de production" \
  cargo build --release --manifest-path apps/api-rs/Cargo.toml

# Swift n'est pas installé partout, et ce n'est pas une raison de tout arrêter.
# Le dire plutôt que de le taire : une vérification qu'on croit complète et qui
# saute une étape en silence vaut moins que pas de vérification du tout.
if command -v swift >/dev/null 2>&1; then
  etape "WeaveKit" ./apps/ios/verification-linux/verifier.sh
else
  printf '\n\033[33m— WeaveKit : ignoré, Swift absent de cette machine\033[0m\n'
  printf '  La CI le vérifie ; ici, non. Installation : https://swift.org/install\n'
fi

etape "Formatage" bunx --bun prettier --check .

printf '\n\033[32mTout est vert.\033[0m\n'
