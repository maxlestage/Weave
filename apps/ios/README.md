# Weave — iOS et watchOS

Application native Swift : iPhone, Live Activity « Métier », application Apple
Watch et complication de cadran. Tout le code partagé vit dans le paquet
`WeaveKit`, consommé par les quatre cibles.

## Cibles

| Cible | Type | Rôle |
| --- | --- | --- |
| `Weave` | Application iOS | L'application : le métier, les fils, les réponses |
| `WeaveActivity` | Extension WidgetKit | Live Activity sur l'écran verrouillé et l'île dynamique |
| `WeaveWatch` | Application watchOS | Liste des fils et échéances au poignet |
| `WeaveWatchWidgets` | Extension WidgetKit | Complication de cadran |
| `WeaveKit` | Paquet Swift | Modèles, client d'API, magasins d'état |

## Générer le projet

Le fichier `.xcodeproj` n'est pas versionné : il se régénère à partir de
`project.yml`. Cela évite les conflits de fusion sur le format `pbxproj` et rend
la configuration du projet lisible.

```sh
brew install xcodegen        # une fois
cd apps/ios
xcodegen generate
open Weave.xcodeproj
```

Renseignez votre identifiant d'équipe avant de compiler :

```sh
echo 'WEAVE_TEAM_ID = VOTREEQUIPE' > Secrets.xcconfig   # non versionné
```

## Adresse de l'API

Elle vient du `Info.plist` (`WEAVE_API_URL`), alimenté par la configuration de
build : `http://localhost:3000` en Debug, `https://api.weave.app` en Release.
Rien n'est codé en dur dans le binaire.

Pour tester contre l'API locale depuis un iPhone réel, remplacez `localhost` par
l'adresse de votre machine sur le réseau local, dans le `xcconfig` de Debug.

## Live Activity : les deux jetons

C'est le point le plus subtil de l'intégration, et le plus facile à confondre.

- **Jeton « push-to-start »** — propre à l'appareil, obtenu par
  `Activity.pushToStartTokenUpdates`. Il permet au serveur de **démarrer** une
  Live Activity à distance : c'est lui qui fait apparaître la bannière à l'heure
  de tissage, application fermée. Il est transmis à `PUT /v1/devices`.
- **Jeton de mise à jour** — propre à **une** activité en cours, obtenu par
  `activity.pushTokenUpdates`. Il permet d'en modifier le contenu. Il est
  transmis à `POST /v1/live-activity/sessions`.

Les deux peuvent être renouvelés par le système à tout moment : `ActivityController`
écoute leurs flux pendant toute la durée de vie de l'application, jamais une
seule fois au démarrage.

Côté serveur, le sujet APNs doit porter le suffixe `.push-type.liveactivity` et
l'en-tête `apns-push-type` valoir `liveactivity` — c'est fait dans
`apps/api/src/lib/apns.ts`.

Le nom du type `WeaveActivityAttributes` est repris tel quel par le serveur dans
le champ `attributes-type` de la charge utile. **Le renommer casse le démarrage
à distance.**

## Ce que l'on ne met jamais dans une Live Activity

Une bannière d'écran verrouillé est lisible par quiconque regarde le téléphone
posé sur une table. L'état poussé ne contient donc que des compteurs, un prénom
et des échéances. Jamais de photo, jamais un message, jamais un nom complet.

Les décomptes utilisent `Text(timerInterval:)` : le système les anime lui-même,
sans réveiller l'application ni consommer d'envoi APNs.

## Tests

```sh
cd apps/ios/WeaveKit
swift test
```

Les tests couvrent le plafond des trois fils, le décodage des charges utiles de
l'API, les états de la Live Activity et la logique de renouvellement de session.
Ils utilisent `swift-testing`, pas XCTest.

## Trousseau et groupe d'application

La session est stockée dans le trousseau, dans le groupe partagé
`com.weave.app.shared`, avec `kSecAttrAccessibleAfterFirstUnlock` : l'extension
et la montre peuvent la lire appareil verrouillé, mais seulement après un premier
déverrouillage depuis le démarrage.

Le résumé destiné à la complication transite par les préférences du groupe
`group.com.weave.app` : la complication ne fait aucun appel réseau, elle serait
réveillée bien trop souvent pour une donnée qui change une fois par jour.
