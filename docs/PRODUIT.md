# Weave — le produit

> Trois fils par jour. Rien de plus.

## Le problème que Weave prend au sérieux

Les applications de rencontre grand public optimisent le temps passé. Elles
produisent donc du volume : des piles de profils, des files d'attente qui
grossissent, des compteurs à vider. Le résultat est connu — on parcourt
beaucoup, on lit peu, on écrit encore moins.

Weave prend le contre-pied par une contrainte : **trois profils à la fois,
jamais plus, pour personne.** Tout le reste du produit découle de là.

## Le vocabulaire

Le produit emprunte son lexique au tissage. Ce n'est pas de l'ornement : chaque
terme désigne une mécanique précise, et le même mot est utilisé dans le code,
dans l'API et dans l'interface.

| Terme | Ce que c'est |
| --- | --- |
| **Métier** | L'écran principal, qui porte au plus trois fils |
| **Fil** | Une personne proposée, pendant 24 h |
| **Trame** | Les trois fragments + le motif qui présentent un fil |
| **Fragment** | Une question et la réponse d'une personne |
| **Motif** | Les cinq mots-clés qui situent quelqu'un |
| **Tisser** | Composer les fils du jour |
| **Engager** | Répondre à un fragment |
| **Dénouer** | Laisser un fil expirer, ou le relâcher |
| **Heure de tissage** | Le rendez-vous quotidien choisi par la personne |
| **Regarnir** | Remplacer un fil dénoué |

## Les cinq règles

### 1. Trois fils, jamais plus

Le plafond est un invariant technique (`MAX_ACTIVE_THREADS`), appliqué de façon
atomique côté serveur par un script Lua dans Redis, et réappliqué à la réception
côté client. **Aucun palier d'abonnement ne le relève.** Ce qui se vend, c'est
la vitesse de remplacement d'un fil dénoué et la finesse des critères — jamais
le volume.

C'est une décision commerciale autant que produit : le jour où l'on vend « plus
de profils », on a construit exactement ce que l'on cherchait à éviter.

### 2. Les profils proposés n'existent qu'en cache

Les fils vivent dans Redis, avec un TTL. La base relationnelle ne contient
**aucune copie** d'un profil proposé : ni photo, ni fragment, ni motif. Elle ne
garde qu'un registre minimal — deux identifiants, un horodatage, une issue —
pour ne jamais reproposer deux fois la même personne.

Voir [CACHE.md](./CACHE.md) pour le détail.

### 3. On engage en écrivant

Il n'y a pas de geste pour dire oui ou non. Pour engager un fil, il faut répondre
à un fragment, au minimum douze caractères. L'autre personne reçoit ce que vous
avez écrit, pas un signal.

Conséquence directe : il n'existe pas de « likes reçus », donc pas de file
d'attente à monétiser, donc pas de compteur à faire grossir.

### 4. Le temps fait le tri

Un fil non engagé se dénoue après 24 h et disparaît. Il n'y a pas de retour en
arrière gratuit ni de pile qui s'accumule. Ne rien faire est une réponse
valable, et elle ne coûte rien à personne : l'autre ne reçoit pas de refus, le
fil s'éteint.

Le plafond absolu de vie d'un fil est de 48 h, même avec des achats répétés.

### 5. La photo se dévoile, elle n'ouvre pas

Un fil se présente d'abord par sa trame. La photo est là, mais floutée **côté
serveur** — l'image nette ne quitte jamais le serveur tant que la révélation ne
l'autorise pas. À chaque échange abouti, elle gagne un cran : 0 %, 33 %, 66 %,
100 %.

## Le déroulé

1. **Inscription** — adresse e-mail, code à six chiffres. Pas de mot de passe.
2. **Fiche** — ville, position arrondie au kilomètre, genre, intention.
3. **Fragments** — réponses à trois questions de la bibliothèque.
4. **Motif** — cinq mots.
5. **Heure de tissage** — 8 h, 12 h, 18 h ou 21 h.
6. **Le métier se garnit** — à l'heure dite, la Live Activity apparaît d'elle-même.
7. **Répondre, ou laisser faire.**
8. **Le fil se tisse** — au deuxième échange, la conversation s'ouvre et devient
   persistante ; c'est le seul moment où quelque chose est écrit en base.

## Ce que Weave n'a pas, et n'aura pas

- Pile de cartes à balayer
- Liste des personnes qui vous ont aimé
- Mise en avant payante dans le vivier de composition
- Compteur de vues de votre profil
- Publicité, revente ou courtage de données
- Relances automatiques pour vous faire revenir

Ces absences ne sont pas des manques de la version 1 : elles sont le produit.
