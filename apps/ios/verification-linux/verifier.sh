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

# ----------------------------------------------------------------------------
# Le reste : l'application, l'extension et la complication
#
# Trois mille quatre cents lignes de SwiftUI, qu'aucun Linux ne saura typer —
# il n'y a ni SwiftUI ni WidgetKit ici, et il n'y en aura pas. Reste
# l'analyse syntaxique, qui, elle, ne dépend d'aucun cadre : `-parse` lit la
# grammaire sans résoudre un seul `import`.
#
# C'est peu : une faute de frappe dans un nom, un argument mal étiqueté, un
# membre inexistant passent tous les trois. Mais une accolade en trop, une
# expression tronquée ou une déclaration incomplète ne passent pas, et ce code
# n'avait jusqu'ici absolument rien pour l'arrêter.
# ----------------------------------------------------------------------------
echo "Analyse syntaxique de l'application iOS"
refuses=0
while IFS= read -r fichier; do
    if ! swift-frontend -parse "$fichier" 2>&1; then
        echo "  refusé : ${fichier#"$racine/"}"
        refuses=$((refuses + 1))
    fi
done < <(find "$racine/apps/ios" -name '*.swift' -not -path '*/WeaveKit/*' | sort)

if [ "$refuses" -gt 0 ]; then
    echo "$refuses fichier(s) ne s'analysent pas" >&2
    exit 1
fi
echo "  tout s'analyse"
