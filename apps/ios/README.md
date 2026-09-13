# Weave — iOS et watchOS

Application native Swift : iPhone, Live Activity « Prochain plan », application
Apple Watch et complication de cadran. Tout le code partagé vit dans le paquet
`WeaveKit`, consommé par les quatre cibles.

## Cibles

| Cible | Type | Rôle |
| --- | --- | --- |
| `Weave` | Application iOS | L'application : le fil, les plans, les demandes |
| `WeaveActivity` | Extension WidgetKit | Live Activity sur l'écran verrouillé et l'île dynamique |
| `WeaveWatch` | Application watchOS | Le prochain plan et ce qui attend une réponse |
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

## Langues

L'application est écrite en français, et `Weave/Localizable.xcstrings` en porte
les traductions anglaise et espagnole.

Le catalogue de chaînes n'a pas demandé de changer une ligne de Swift : en
SwiftUI, `Text("Publier")` passe déjà par `LocalizedStringKey`, et le texte
français EST la clé. Les phrases françaises restent donc lisibles dans le code,
et une chaîne sans traduction retombe sur elles.

Le service, lui, traduit ses propres messages d'erreur : l'application les rend
tels quels, et chacune de ses requêtes annonce la langue du téléphone par
`Accept-Language` (voir `WeaveKit/.../Networking/WeaveAPI.swift`).

### Ce qui reste à faire dans Xcode

Deux choses demandent l'outil, et n'ont donc pas pu être faites depuis un
éditeur de texte :

1. **Vérifier que `en` et `es` figurent dans les localisations du projet**
   (Project › Info › Localizations). XcodeGen ne les déduit pas d'un catalogue
   de chaînes comme il le ferait de répertoires `.lproj`, et sans elles Xcode
   pourrait ne pas compiler les traductions dans le paquet.

2. **Traduire les chaînes interpolées.** Une phrase comme
   `Text("Bloquer \(prenom)")` n'a pas la même clé que son texte : Xcode y
   substitue un spécificateur de format, qu'il faut le laisser extraire plutôt
   que le deviner. À la première compilation, il les ajoute lui-même au
   catalogue, à côté des autres, avec un état « à traduire ». Il y en a
   vingt-neuf.

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
  Live Activity à distance : c'est lui qui fait apparaître la bannière quand
  quelqu'un demande à venir, application fermée. Il est transmis à
  `PUT /v1/devices`.
- **Jeton de mise à jour** — propre à **une** activité en cours, obtenu par
  `activity.pushTokenUpdates`. Il permet d'en modifier le contenu. Il est
  transmis à `POST /v1/live-activity/sessions`.

Les deux peuvent être renouvelés par le système à tout moment : `ActivityController`
écoute leurs flux pendant toute la durée de vie de l'application, jamais une
seule fois au démarrage.

Côté serveur, le sujet APNs doit porter le suffixe `.push-type.liveactivity` et
l'en-tête `apns-push-type` valoir `liveactivity` — c'est fait dans
`apps/api-rs/src/apns.rs`.

Le nom du type `WeaveActivityAttributes` est repris tel quel par le serveur dans
le champ `attributes-type` de la charge utile. **Le renommer casse le démarrage
à distance.**

## Ce que l'on ne met jamais dans une Live Activity

Une bannière d'écran verrouillé est lisible par quiconque regarde le téléphone
posé sur une table. L'état poussé ne contient donc qu'un titre de plan, une
heure et deux compteurs. Jamais de nom, jamais de photo, jamais un message —
et un titre de plan est justement ce qu'on peut lire par-dessus l'épaule sans
rien apprendre de qui vous voyez.

Les décomptes utilisent `Text(timerInterval:)` : le système les anime lui-même,
sans réveiller l'application ni consommer d'envoi APNs.

## Tests

```sh
cd apps/ios/WeaveKit
swift test
```

Les tests couvrent l'ordre du fil (jamais retrié localement), le décodage des
charges utiles de l'API, les états de la Live Activity, la logique de
renouvellement de session et l'absence de tout produit vendant de la visibilité.
Ils utilisent `swift-testing`, pas XCTest.

## Trousseau et groupe d'application

La session est stockée dans le trousseau, dans le groupe partagé
`com.weave.app.shared`, avec `kSecAttrAccessibleAfterFirstUnlock` : l'extension
et la montre peuvent la lire appareil verrouillé, mais seulement après un premier
déverrouillage depuis le démarrage.

Le résumé destiné à la complication transite par les préférences du groupe
`group.com.weave.app` : la complication ne fait aucun appel réseau, elle serait
réveillée bien trop souvent pour une donnée qui change une fois par jour.
