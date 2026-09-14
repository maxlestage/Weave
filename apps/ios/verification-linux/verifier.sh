#!/usr/bin/env bash
# Compile WeaveKit et joue ses tests sous Linux.
#
# Six mille lignes de Swift n'avaient jamais rencontré de compilateur : le
# dépôt n'a pas de Mac, et Xcode est le seul endroit où ce code se construisait.
# Deux erreurs de compilation y dormaient — un `deinit` non isolé dans une
# classe `@MainActor`, et deux formateurs de date partagés que le mode Swift 6
# refuse. Aucune n'avait de rapport avec iOS : ce sont des règles du langage.
#
# Ce que ce script NE vérifie PAS : tout ce qui touche UIKit, SwiftUI,
# ActivityKit et StoreKit. Ces fichiers portent déjà leur propre garde et se
# vident hors d'iOS. Reste le cœur — modèles, client d'API, magasins d'état —
# soit la part que le serveur doit satisfaire, et celle qui se teste.
set -euo pipefail

racine="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
atelier="$(mktemp -d)"
trap 'rm -rf "$atelier"' EXIT

cp -r "$racine/apps/ios/WeaveKit/." "$atelier/"
# Les greffons vivent hors de la bibliothèque : ils n'entrent que dans cette
# copie de travail, jamais dans ce que l'application embarque.
cp "$racine/apps/ios/verification-linux/greffons/"*.swift "$atelier/Sources/WeaveKit/"

cd "$atelier"
swift build --build-tests
swift test
