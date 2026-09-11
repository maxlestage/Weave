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

> ### À lire avant tout : n'utilisez pas les déploiements du tableau de bord
>
> Dans l'onglet *Deploy* du tableau de bord Heroku, **« Connect to GitHub » et
> « Enable Automatic Deploys » ne fonctionnent pas pour ce projet.** Ce chemin
> construit avec des *buildpacks* : il ignore le `Dockerfile`, croit à une
> application Node.js, et échoue ainsi —
>
> ```
> -----> Using buildpack: heroku/nodejs
>        npm error Unsupported URL Type "workspace:": workspace:*
> ```
>
> Weave tourne sous Bun, qui n'a pas de buildpack Heroku. Le déploiement doit
> passer par le workflow **« Heroku — déployer »** décrit plus bas, qui pousse
> vers le dépôt git d'Heroku et déclenche bien une construction du
> `Dockerfile`.
>
> **Si vous avez déjà activé les déploiements automatiques, désactivez-les** :
> ils échoueront à chaque poussée sur `master` et brouilleront les journaux.

L'API et le site vitrine tournent dans **un seul processus** : le site est
statique et peu visité, lui dédier un second dyno doublerait la facture sans
rien apporter.

### A1. Créer la clé d'API Heroku

1. Ouvrez **dashboard.heroku.com/account**
2. Section **API Key** → **Reveal** → copiez la valeur

### A2. Déposer la clé dans GitHub

1. Sur le dépôt : **Settings → Secrets and variables → Actions**
2. Onglet **Secrets** → **New repository secret**
   - Nom : `HEROKU_API_KEY`
   - Valeur : la clé copiée

### A3. Créer l'application

> **Si vous avez déjà créé l'application depuis le tableau de bord Heroku**,
> faites quand même cette étape en indiquant **le nom existant**. Le workflow
> détectera que l'application est déjà là et se contentera de corriger la pile
> de construction — ce qui est justement le réglage que le tableau de bord ne
> pose pas. Voir « Pourquoi la pile compte » plus bas.

1. Onglet **Actions** du dépôt
2. Workflow **« Heroku — créer l'application »** → **Run workflow**
3. Renseignez un nom (minuscules, chiffres et tirets — il doit être unique sur
   tout Heroku, par exemple `weave-api-2026`) et la région `eu`
4. **Run workflow**

En deux minutes, le workflow crée l'application, y branche PostgreSQL et le
magasin clé-valeur, et **génère les secrets** de signature. Ceux-ci ne passent
par aucun écran : ils sont écrits directement dans la configuration Heroku.

Le récapitulatif affiché à la fin rappelle le nom et l'adresse de
l'application.

### A4. Déclarer le nom de l'application

1. **Settings → Secrets and variables → Actions**, onglet **Variables**
2. **New repository variable**
   - Nom : `HEROKU_APP_NAME`
   - Valeur : le nom choisi à l'étape A3

### A5. Déployer

1. **Actions** → **« Heroku — déployer »** → **Run workflow**

Le workflow pousse le code vers Heroku, qui construit l'image, applique les
migrations de base, puis met la nouvelle version en ligne. Il interroge enfin
`/health` et n'affiche « Déployé » que si le service répond vraiment.

**À partir de là, tout ce qui est fusionné sur `master` est déployé
automatiquement.** Vous n'avez plus rien à lancer à la main.

### Pourquoi la pile de construction est décisive

Weave tourne sous **Bun**, pour lequel il n'existe aucun buildpack Heroku. Le
déploiement passe donc par le `Dockerfile` décrit dans `heroku.yml` — et Heroku
ne lit `heroku.yml` **que si l'application est sur la pile `container`**.

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

**Correction**, dans cet ordre :

1. Lancez le workflow de l'étape A3 avec le nom de votre application. Il bascule
   la pile en `container`, **retire les buildpacks** et affiche un avant/après.
2. Désactivez les déploiements automatiques dans l'onglet *Deploy* du tableau de
   bord — sans quoi ils continueront d'échouer à chaque poussée.
3. Lancez le workflow **« Heroku — déployer »**.

Le workflow de déploiement vérifie désormais la pile avant de pousser quoi que
ce soit, et s'arrête en nommant la cause si elle n'est pas la bonne.

### Déployer par le workflow, pas par le tableau de bord

Si vous avez activé les **déploiements automatiques depuis GitHub** dans
l'onglet *Deploy* du tableau de bord Heroku, désactivez-les : utilisez le
workflow **« Heroku — déployer »**, qui pousse vers le dépôt git d'Heroku et
déclenche bien une construction du `Dockerfile`.

Deux mécanismes de déploiement concurrents sur la même application, c'est la
garantie de ne plus savoir lequel a produit la version en ligne.

### A6. Vérifier

Ouvrez dans votre navigateur :

- `https://VOTRE-APP.herokuapp.com` → le site vitrine
- `https://VOTRE-APP.herokuapp.com/health` → doit afficher `"status":"ok"`
- `https://VOTRE-APP.herokuapp.com/openapi` → la documentation de l'API

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

---

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
  bord et corrigez `.github/workflows/heroku-provision.yml`.

---

## En cas de problème

| Symptôme | Piste |
| --- | --- |
| `Node.js app detected` ou `Using buildpack: heroku/nodejs`, puis `EUNSUPPORTEDPROTOCOL` / `workspace:*` | Le build n'utilise pas le `Dockerfile`. Deux causes possibles, souvent simultanées : la pile n'est pas `container` (étape A3), et/ou le déploiement vient du tableau de bord au lieu du workflow (encadré en tête de section A) |
| « Pile « heroku-24 » au lieu de « container » » | Même cause, détectée cette fois avant la poussée. Même correction |
| « Déploiement ignoré : configuration Heroku absente » | Le secret `HEROKU_API_KEY` ou la variable `HEROKU_APP_NAME` manque — étapes A2 et A4 |
| Le déploiement réussit mais `/health` reste muet | Journaux dans le tableau de bord Heroku, onglet **More → View logs** |
| `"cache":{"ok":false}` | Le magasin clé-valeur n'est pas branché. Sans lui, il n'y a pas de fils : c'est une dépendance dure, pas un confort |
| « Publication ignorée : configuration Apple absente » | Un des secrets de l'étape B3 manque |
| Échec de signature iOS | Relancez avec la case « créer les certificats » cochée, **une seule fois** |
