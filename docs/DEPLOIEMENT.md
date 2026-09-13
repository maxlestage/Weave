# Déployer Weave depuis un téléphone

Ce guide suppose que vous n'avez **pas d'ordinateur**. Tout ce qui suit se fait
depuis un navigateur mobile, sur github.com et heroku.com.

Il y a deux chaînes indépendantes :

| | Ce qui est déployé | Où | Compte nécessaire |
| --- | --- | --- | --- |
| **A** | L'API et le site vitrine | Heroku | Heroku (carte bancaire requise) |
| **B** | L'application iPhone et montre | TestFlight | Apple Developer (99 €/an) |

La chaîne A suffit pour avoir un service en ligne. La chaîne B est nécessaire
pour mettre l'application entre les mains de testeurs.

---

## A. L'API et le site, sur Heroku

L'API et le site vitrine tournent dans **un seul processus** : le site est
statique et peu visité, lui dédier un second dyno doublerait la facture sans
rien apporter.

Il y a **deux étapes**, et rien d'autre.

### A1. Déposer votre clé Heroku dans GitHub

1. Ouvrez **dashboard.heroku.com/account**, section **API Key** → **Reveal**,
   copiez la valeur.
2. Sur le dépôt GitHub : **Settings → Secrets and variables → Actions**, onglet
   **Secrets** → **New repository secret**.
   - Nom : `HEROKU_API_KEY`
   - Valeur : la clé copiée

### A2. Lancer la mise en ligne

1. Onglet **Actions** du dépôt
2. Workflow **« Heroku — mettre en ligne »** → **Run workflow**
3. Indiquez un nom d'application (minuscules, chiffres et tirets — il doit être
   unique sur tout Heroku, par exemple `weave-api-2026`) et la région `eu`
4. **Run workflow**

Le workflow crée l'application, la met sur la bonne pile, y branche PostgreSQL
et le magasin clé-valeur, génère les secrets de signature, construit l'image et
déploie. Il interroge enfin `/health` et n'annonce « en ligne » que si le
service répond vraiment.

**Si vous avez déjà une application Heroku**, indiquez son nom : le workflow
détecte qu'elle existe, corrige ce qui doit l'être, et déploie. Il est
relançable sans risque.

Comptez cinq à dix minutes pour la première construction.

### A3. Vérifier

Le récapitulatif du workflow affiche les trois adresses. Vous pouvez aussi les
ouvrir directement :

- `https://VOTRE-APP.herokuapp.com` → le site vitrine
- `https://VOTRE-APP.herokuapp.com/health` → doit afficher `"status":"ok"`

### A4. Déployer automatiquement ensuite (facultatif)

Pour que chaque fusion sur `master` parte en ligne toute seule, ajoutez une
variable de dépôt — **Settings → Secrets and variables → Actions**, onglet
**Variables** :

- Nom : `HEROKU_APP_NAME`
- Valeur : le nom choisi en A2

Sans elle, le workflow de déploiement automatique s'arrête proprement sans rien
faire ; relancez simplement « Heroku — mettre en ligne » quand vous voulez
publier.

### Variante : déployer depuis le tableau de bord Heroku

Si vous préférez le tableau de bord et ses déploiements automatiques depuis
GitHub, c'est possible — mais pas avec le buildpack Node.js, qui appelle `npm`
et bute aussitôt :

```
-----> Using buildpack: heroku/nodejs
       npm error Unsupported URL Type "workspace:": workspace:*
```

Heroku ne fournit de buildpack ni pour Bun ni pour Rust, et Weave a besoin des
deux : le site est bâti par Bun, l'API compilée par Cargo. Le dépôt embarque
donc **son propre buildpack**, dans `bin/detect`, `bin/compile` et
`bin/release`, exécuté par le buildpack officiel `heroku-community/inline` —
dont le rôle est précisément de lancer un buildpack contenu dans le dépôt de
l'application. Aucun code tiers n'intervient.

**Une seule action :** tableau de bord → **Settings → Buildpacks** → **Add
buildpack** → `heroku-community/inline`.

Retirer `heroku/nodejs` est préférable — il ne sert à rien ici — mais le laisser
ne casse plus rien. Il installera des paquets pour rien, puis le buildpack
inline fera le vrai travail :

- les dépendances internes sont déclarées avec une syntaxe que npm comprend, et
  `heroku-postbuild` neutralise son étape de construction ;
- le Node.js qu'il dépose dans le slug — **208 Mo** — est retiré par
  `bin/compile` avant la mesure finale. Sans cela, le slug passait de 422 à
  622 Mo et Heroku rejetait la publication. Rien dans Weave n'exécute Node : le
  dyno ne lance qu'un binaire, pour la phase de publication comme pour le web.

Les déploiements automatiques depuis GitHub fonctionnent ensuite normalement.

#### Un buildpack Rust en plus fait échouer toute la construction

Ajouter `emk/heroku-buildpack-rust` à côté de l'inline paraît raisonnable —
l'API est en Rust, après tout. **C'est ce qui casse le déploiement**, et la
manière dont il casse mérite d'être connue.

Ce buildpack cherche un `Cargo.toml` **à la racine du dépôt**. Celui de Weave
est dans `apps/api-rs/`. Sa détection échoue donc, et une détection qui échoue
sur un buildpack de la liste annule la publication entière :

```
-----> Weave est prêt
       présent : bin-release/weave-api
...
-----> App not compatible with buildpack: https://github.com/emk/heroku-buildpack-rust
 !     Push failed
```

Le piège est là : **la construction a réussi**. `bin/compile` a installé la
chaîne Rust, compilé le binaire et vérifié sa présence. Tout le travail est
fait, et il est jeté à la dernière ligne par un buildpack qui n'avait rien à
faire là. L'application continue de servir la version précédente, sans qu'aucun
message ne relie ce qu'on voit en ligne à ce qu'on vient de fusionner.

`bin/compile` construit déjà l'API — c'est tout son objet. Aucun buildpack Rust
n'est nécessaire :

```sh
heroku buildpacks -a weave                 # voir la liste
heroku buildpacks:remove https://github.com/emk/heroku-buildpack-rust -a weave
```

`heroku/nodejs` peut rester sans danger, mais il reconstruit le site une
seconde fois pour rien : le retirer aussi raccourcit chaque déploiement.

#### Les buildpacks Bun tiers ne conviennent plus

Il en existe plusieurs — `jakeg/heroku-buildpack-bun` et les autres — et ils
ont servi tant que l'API était écrite en TypeScript. **Ils ne peuvent plus
construire Weave.**

La raison tient en une phrase : ces buildpacks installent Bun, puis appellent
`heroku-prebuild`, `build` et `heroku-postbuild`. Aucun d'eux n'installe la
chaîne Rust, et sans elle `cargo build` n'existe pas. La construction se
termine donc sans erreur visible — le site sort — mais `bin-release/weave-api`
n'est jamais produit, et le dyno s'arrête au démarrage sur un `No such file or
directory` qui ne désigne pas l'étape manquante.

Si le tableau de bord affiche encore un buildpack Bun tiers dans **Settings →
Buildpacks**, il faut le remplacer par `heroku-community/inline`. C'est la
seule voie par buildpack ; l'autre est le conteneur, décrit plus bas.

#### Poser la configuration

Une application créée depuis le tableau de bord n'a **aucune** des variables que
Weave exige : `app.json` ne s'applique qu'aux déploiements par bouton Heroku.
Sans elles, la construction réussit, puis le dyno démarre et s'arrête aussitôt —
Heroku affiche `H10 App crashed`.

Le plus simple, depuis un téléphone : Actions → **« Heroku — configurer les
variables »** → **Run workflow**, en donnant le nom de l'application. Le
workflow pose tout ce qui manque, **sans toucher à la pile ni aux buildpacks**,
et conserve les secrets déjà posés — les régénérer déconnecterait tout le monde
et rendrait illisibles les URL de médias déjà signées.

À la main, c'est **Settings → Config Vars** :

| Variable | Valeur |
| --- | --- |
| `JWT_SECRET` | une chaîne aléatoire d'au moins 32 caractères |
| `MEDIA_SIGNING_SECRET` | une autre chaîne aléatoire |
| `NODE_ENV` | `production` |
| `WEAVE_DB` | `postgres` |
| `DATABASE_SSL_INSECURE` | `true` |
| `REDIS_TLS_INSECURE` | `true` |
| `PUBLIC_WEB_ORIGIN` | `https://<application>.herokuapp.com` |
| `MEDIA_BASE_URL` | `https://<application>.herokuapp.com/media` |

`DATABASE_URL` et `REDIS_URL` sont posées par les add-ons **Heroku Postgres** et
**Heroku Key-Value Store**, à attacher dans **Resources**. Sans eux, rien ne
démarre.

Si une variable manque, Weave ne démarre pas — mais il dit lesquelles, **toutes
d'un coup** et avec le remède à côté de chaque ligne :

```
  Weave ne peut pas démarrer : la configuration est incomplète.

  • JWT_SECRET — absente
      une chaîne aléatoire d'au moins 32 caractères — `openssl rand -base64 32`
  • MEDIA_SIGNING_SECRET — absente
      une autre chaîne aléatoire — `openssl rand -base64 32`
```

Apprendre les variables manquantes une par une coûterait un déploiement par
variable.

#### Ce que cette variante coûte

Le slug Heroku est plafonné à **500 Mo**. C'est ce plafond qui a fait échouer
les premières publications, quand l'API était en TypeScript : Bun, ses
dépendances et un Node.js déposé pour rien pesaient ensemble plus de 600 Mo.

Depuis le passage à Rust, le dyno ne transporte plus qu'un binaire de **20 Mo**
et le site construit. La chaîne Rust, qui pèse près d'un gigaoctet, reste dans
le cache de construction et n'entre jamais dans le slug ; les `node_modules`
ayant servi à bâtir le site en sortent aussi, puisque plus rien ne les ouvre.

`bin/compile` mesure le slug à chaque construction, prévient au-delà de 450 Mo
et **interrompt la construction au-delà de 500 Mo**, plutôt que de laisser
Heroku la rejeter avec un message obscur. Il vérifie aussi que le binaire et
`apps/web/dist/index.html` sont bien là : un slug auquel il manque l'un des
deux se publie sans erreur, et c'est au démarrage que le dyno s'arrête.

### Ce que ça coûte

| Poste | Plan | Prix indicatif |
| --- | --- | --- |
| Dyno web | Basic | ~7 $/mois |
| PostgreSQL | Essential-0 | ~5 $/mois |
| Magasin clé-valeur | Mini | ~3 $/mois |

Soit environ **15 $ par mois**. Vérifiez les tarifs en vigueur : ils changent.

### Deux réglages à connaître

`DATABASE_SSL_INSECURE` et `REDIS_TLS_INSECURE` sont positionnés à `true`.

Heroku présente des certificats **auto-signés** sur son réseau interne : sans
ces réglages, la connexion échoue. Le trafic reste chiffré, mais l'identité du
serveur n'est pas vérifiée. C'est acceptable parce que la base et le cache sont
joints par le réseau privé de l'hébergeur — **ce ne le serait pas** pour
atteindre une base à travers l'internet public. Si vous changez d'hébergeur,
repassez ces deux valeurs à vide.

### Pourquoi la pile de construction est décisive

Heroku ne fournit de buildpack ni pour Bun ni pour Rust, et Weave a besoin des
deux. Cette voie passe donc par le `Dockerfile` décrit dans `heroku.yml` — et
Heroku ne lit `heroku.yml` **que si l'application est sur la pile
`container`**.

Une application créée depuis le tableau de bord est sur la pile `heroku-24` par
défaut. Dans ce cas, Heroku ignore le `Dockerfile`, croit à une application
Node.js, et le build échoue ainsi :

```
-----> Node.js app detected                  (buildpack déduit)
-----> Using buildpack: heroku/nodejs        (buildpack imposé)
       npm error code EUNSUPPORTEDPROTOCOL
       npm error Unsupported URL Type "workspace:": workspace:*
```

Les deux premières lignes se valent : dans un cas Heroku a deviné un buildpack,
dans l'autre il en a un de configuré. Le résultat est le même, et la cause
aussi — l'application n'est pas sur la pile `container`.

Ce message ne parle pas de la vraie cause. `workspace:*` désigne les paquets
internes du dépôt ; c'est une syntaxe que Bun comprend et que npm ne
comprendra jamais. Le problème n'est pas cette ligne : c'est que npm n'aurait
jamais dû être appelé.

**Correction** : lancez le workflow **« Heroku — mettre en ligne »** (étape A2)
avec le nom de votre application. Il bascule la pile en `container`, retire les
buildpacks, affiche un avant/après, et déploie dans la foulée.

## B. L'application iOS, sans posséder de Mac

Construire une application iOS exige macOS. Vous n'avez pas de Mac — mais
**GitHub en loue à la minute**, et le workflow `ios-testflight.yml` s'en sert.
La construction, la signature et l'envoi à TestFlight se déclenchent depuis le
navigateur de votre téléphone.

Il faut en revanche un **compte Apple Developer** (99 €/an) : c'est
incontournable, aucune application ne peut être distribuée sans.

### B1. La clé d'API App Store Connect

Sur **appstoreconnect.apple.com** → **Users and Access** → **Integrations** →
**App Store Connect API** :

1. **+** pour créer une clé, rôle **App Manager**
2. Notez le **Key ID** et l'**Issuer ID**
3. Téléchargez le fichier `.p8` — **il n'est téléchargeable qu'une fois**

Le fichier `.p8` doit être converti en base64 pour être déposé dans GitHub.
Depuis un téléphone, le plus simple est d'ouvrir le fichier dans une
application de notes, de copier son contenu, puis d'utiliser un encodeur
base64 hors ligne. **N'utilisez pas un service d'encodage en ligne** : cette
clé donne accès à votre compte développeur.

### B2. Le dépôt des certificats

`fastlane match` conserve certificats et profils, **chiffrés**, dans un dépôt
privé. C'est ce qui permet à une machine neuve de signer sans Mac de référence.

1. Créez un dépôt GitHub **privé** et **vide**, par exemple `weave-certificats`
2. Créez un jeton d'accès personnel ayant le droit `repo` sur ce dépôt
   (**Settings → Developer settings → Personal access tokens**)
3. Encodez `votre-identifiant:le-jeton` en base64

### B3. Déposer les valeurs dans GitHub

**Settings → Secrets and variables → Actions**, onglet **Secrets** :

| Nom | Valeur |
| --- | --- |
| `ASC_KEY_ID` | Le Key ID de l'étape B1 |
| `ASC_ISSUER_ID` | L'Issuer ID de l'étape B1 |
| `ASC_KEY_CONTENT` | Le fichier `.p8` encodé en base64 |
| `ASC_TEAM_ID` | Votre identifiant d'équipe Apple |
| `MATCH_PASSWORD` | Une phrase secrète que vous choisissez — elle chiffre les certificats, **notez-la** |
| `MATCH_GIT_TOKEN` | `identifiant:jeton` en base64, de l'étape B2 |

Onglet **Variables** :

| Nom | Valeur |
| --- | --- |
| `MATCH_GIT_URL` | L'adresse du dépôt privé, par exemple `https://github.com/vous/weave-certificats.git` |

### B4. Déclarer l'application sur App Store Connect

Créez sur App Store Connect une application avec l'identifiant
`com.weave.app`, ainsi que les trois identifiants d'extension :

- `com.weave.app.activity` — la Live Activity
- `com.weave.app.watchkitapp` — l'application montre
- `com.weave.app.watchkitapp.widgets` — la complication

### B5. Première construction

1. **Actions** → **« iOS — envoyer à TestFlight »** → **Run workflow**
2. Cochez **« Première exécution : autoriser la création des certificats »**
3. **Run workflow**

Cette première exécution crée les certificats et les dépose, chiffrés, dans le
dépôt privé. **Ne cochez plus cette case ensuite** : les exécutions suivantes
réutilisent ce qui a été créé, au lieu d'accumuler des certificats — Apple en
limite le nombre par compte.

Comptez vingt à quarante minutes. Ensuite, la version apparaît dans TestFlight.

### B6. Les fois suivantes

Deux façons de publier :

- **Actions → « iOS — envoyer à TestFlight » → Run workflow**
- ou en poussant une étiquette commençant par `ios-v` (par exemple `ios-v0.2.0`)

---

## Ce que je n'ai pas pu vérifier

Par honnêteté sur ce qui est testé et ce qui ne l'est pas :

- **L'image Docker est construite et démarrée pour de bon** ici, avec PostgreSQL
  et Redis : la chaîne A repose sur du vérifié.
- **La chaîne B ne l'est pas.** Elle demande macOS et un compte Apple Developer,
  dont je ne dispose pas. Les fichiers fastlane et le workflow sont écrits selon
  les pratiques établies, mais la première exécution demandera probablement des
  ajustements — typiquement sur les identifiants d'extension ou la version de
  Xcode. Les journaux d'échec sont conservés en pièce jointe du workflow
  précisément pour cela.
- **Les noms de plans Heroku** (`heroku-postgresql:essential-0`,
  `heroku-redis:mini`) étaient exacts en septembre 2026. Heroku les renomme de
  temps à autre ; en cas d'erreur à l'étape A3, vérifiez-les sur le tableau de
  bord et corrigez `.github/workflows/heroku-setup.yml`.

---

## En cas de problème

| Symptôme | Piste |
| --- | --- |
| `Using buildpack: heroku/nodejs`, puis `EUNSUPPORTEDPROTOCOL` / `workspace:*` | Le buildpack Node.js ne sait pas lire `workspace:*`. Deux issues : lancer « Heroku — mettre en ligne » (A2, voie conteneur), ou remplacer le buildpack par `heroku-community/inline` (variante ci-dessus) |
| `Slug de … Mo : au-delà de la limite de 500 Mo` | La variante buildpack a dépassé le plafond. Passez à la voie conteneur (A2), qui n'a pas cette contrainte |
| « Pile « heroku-24 » au lieu de « container » » | Même cause, détectée avant la poussée. Même correction |
| « Déploiement ignoré : configuration Heroku absente » | C'est le workflow de déploiement *automatique*, qui exige la variable `HEROKU_APP_NAME` (étape A4). Pour publier tout de suite, lancez « Heroku — mettre en ligne » |
| `H10 App crashed` juste après un déploiement réussi | La configuration est incomplète. Le journal du dyno (**More → View logs**, lignes `app[web.1]`) liste les variables manquantes. Lancez « Heroku — configurer les variables » |
| Le déploiement réussit mais `/health` reste muet | Journaux dans le tableau de bord Heroku, onglet **More → View logs** |
| `"cache":{"ok":false}` | Le magasin clé-valeur n'est pas branché. Sans lui, le quota de demandes ne peut pas être compté : c'est une dépendance dure, pas un confort |
| « Publication ignorée : configuration Apple absente » | Un des secrets de l'étape B3 manque |
| Échec de signature iOS | Relancez avec la case « créer les certificats » cochée, **une seule fois** |

## La purge : rien à planifier

Elle tourne d'elle-même, depuis le service. Aucun module à ajouter, aucune
tâche à créer — c'était la dernière chose qui demandait une intervention
d'exploitation, et sans elle la purge n'avait tout simplement pas lieu.

Le fonctionnement tient en deux règles :

- **une tentative par heure**, pas une purge par heure ;
- **un verrou dans le cache**, pris en une seule commande `SET NX EX` et tenu
  vingt-trois heures.

Au plus un passage par jour, donc, quel que soit le nombre de dynos et quelle
que soit l'heure de leur réveil. Un dyno qui dort ne manque aucun horaire : il
n'y a pas d'horaire, seulement un verrou à prendre dès qu'on est éveillé.

Le journal dit ce qui a été fait :

```json
{"message":"purge effectuée","messages":0,"activites":0,"comptes":0,"differes":0}
```

`differes` compte les comptes échus mais retenus par un signalement encore
ouvert — ils partiront au passage suivant, une fois le dossier clos.

### Si vous préférez un planificateur externe

Une tâche d'entretien n'a rien à faire dans le processus qui sert les requêtes,
et la commande reste disponible pour qui veut l'appeler de l'extérieur :

```sh
heroku addons:create scheduler:standard -a weave
# tâche quotidienne : ./bin-release/weave-api purge
```

Les deux cohabitent sans risque : le verrou vaut aussi pour l'appel externe.
