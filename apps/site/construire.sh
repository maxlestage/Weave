#!/usr/bin/env bash
# Construit le site : le HTML au serveur, le wasm pour l'hydratation.
#
# Deux compilations, deux cibles, un seul jeu de composants. Les traits `ssr`
# et `hydration` sont exclusifs — `yew/ssr` et `yew/hydration` demandent des
# arbres de dépendances différents, et les activer ensemble ferait entrer le
# DOM dans le binaire natif.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

etape() { printf '\n\033[1m— %s\033[0m\n' "$1"; }

etape "Le contrat, engendré depuis packages/contracts"
bun run outils/engendrer-contrat.ts

etape "Le wasm d'hydratation"
cargo build --release --features hydration --lib --target wasm32-unknown-unknown
# `--target web` : un module ES, chargé par `<script type="module">`. Pas de
# paquet npm, pas d'empaqueteur — le navigateur sait importer un module.
wasm-bindgen --target web --no-typescript --out-dir dist-wasm \
  target/wasm32-unknown-unknown/release/weave_site.wasm

# `wasm-opt` n'est PAS lancé, et c'est mesuré, pas supposé.
#
# `-Oz`, `-Os` et `-O2` réduisent tous le fichier brut — 273 Kio tombent à
# 240 — mais ils AUGMENTENT le compressé : 98 730 octets gzip deviennent
# 100 386 avec `-Oz`, 100 554 avec `-Os`, 101 378 avec `-O2`. Ils réorganisent
# le code d'une façon qui compresse moins bien.
#
# Ce que le visiteur télécharge est le compressé. L'optimiseur le dégrade donc
# ici, et on s'en passe. C'est contre-intuitif, et c'est pour cela que ce
# commentaire existe : quelqu'un va vouloir l'ajouter.

etape "Le HTML, rendu au serveur"
cargo run --release --features ssr --bin rendre

# Le wasm rejoint le HTML.
#
# `dist-rs/demarrer.js` importe « /weave_site.js » : une adresse servie depuis
# la racine du site. Laisser le wasm dans `dist-wasm` donnerait un site qui
# s'affiche parfaitement et ne s'hydrate jamais — une panne qu'on ne voit pas
# en regardant la page.
etape "Le wasm rejoint le HTML"
cp dist-wasm/weave_site.js dist-wasm/weave_site_bg.wasm dist-rs/
ls -l dist-rs/weave_site.js dist-rs/weave_site_bg.wasm dist-rs/demarrer.js
