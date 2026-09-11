# Weave — le produit

> Des plans, pas des profils.

## Le problème que Weave prend au sérieux

Les applications de rencontre grand public optimisent le temps passé. Elles
produisent donc du volume : des piles de profils, des files d'attente qui
grossissent, des compteurs à vider. Le résultat est connu — on parcourt
beaucoup, on lit peu, on écrit encore moins, et on ne sort pas.

Weave prend le problème par l'autre bout. **On ne publie pas un profil : on
publie un plan pour les jours qui viennent.** Un mur d'escalade jeudi à 19 h, un
concert vendredi, un marché puis un brunch samedi matin. Les autres demandent à
venir — en écrivant pourquoi.

C'est un déplacement, pas une nuance. Une fiche dit qui l'on prétend être ; un
plan dit ce qu'on fait jeudi. Le second est vérifiable, il a une date, et il
finit tout seul.

## À qui Weave s'adresse

Aux **jeunes adultes**. Le vocabulaire, les prix et le rythme sont écrits pour
la tranche 18-30 : le moment où l'on change de ville, de travail et de cercle
d'amis, et où l'on se retrouve à ne connaître personne un vendredi soir.

Les critères par défaut reflètent ce choix — 18 à 32 ans, 25 km — et chacun peut
les élargir.

L'accès est **réservé aux personnes majeures**, sans exception. `MIN_AGE` vaut
18, la date de naissance est vérifiée côté serveur à l'inscription, et le motif
de signalement « mineur » déclenche un traitement prioritaire (voir
[CONFORMITE.md](./CONFORMITE.md)). Une application de rencontre n'a rien à faire
entre les mains de mineurs : c'est un vecteur connu de mise en relation
d'adultes avec des enfants, et l'App Store l'interdit par ailleurs.

## Le vocabulaire

Quatre mots suffisent, et ce sont les mêmes dans le code, dans l'API et dans
l'interface. Aucun n'est inventé : ils veulent déjà dire ce qu'ils désignent.

| Terme | Ce que c'est |
| --- | --- |
| **Plan** | Ce que quelqu'un compte faire, à une date, avec des places |
| **Fil** | L'écran principal : les plans à venir autour de soi |
| **Demande** | Un message écrit pour se joindre à un plan |
| **Conversation** | Ce qui s'ouvre quand une demande est acceptée |

## Les deux invariants

Tout le produit tient à deux règles. Elles sont énoncées dans
`packages/contracts/src/invariants.ts`, appliquées côté serveur, et rappelées
dans l'interface.

### 1. On ne peut pas arroser

Le nombre de demandes envoyables dans une journée est **borné à tous les
paliers, socle gratuit compris** — au minimum `REQUESTS_PER_DAY_FLOOR`, soit
cinq. Le compteur vit dans Redis, expire à minuit dans le fuseau de la personne,
et se décrémente par un script Lua atomique : deux requêtes simultanées ne
peuvent pas dépenser la même demande.

C'est ce qui donne du poids à une demande. Sans limite, envoyer le même message
à trente personnes ne coûte rien, et recevoir une demande ne veut plus rien
dire.

Deux conséquences assumées :

- une demande **retirée avant d'avoir été lue** est rendue — se raviser vite ne
  doit pas coûter la journée ;
- une demande **refusée** ne l'est pas : elle a été lue, elle a occupé
  l'attention de quelqu'un.

Et un plafond complémentaire : au plus `MAX_OPEN_PLANS` plans ouverts à la fois.
Au-delà, ce ne sont plus des plans, c'est une annonce permanente.

### 2. On ne peut pas acheter de visibilité

`PAID_VISIBILITY` vaut littéralement `false` dans les contrats partagés. Le fil
est trié par **imminence puis par proximité**, et par rien d'autre : ni palier,
ni achat, ni ancienneté de compte, ni popularité. Deux termes, tous deux
explicables à qui les lit.

Ce qui se vend, c'est l'horizon de publication, la finesse des critères, les
plans de groupe et les à-côtés (Escale, Bilan) — voir
[MONETISATION.md](./MONETISATION.md). Il n'existe volontairement **aucun produit
de remontée**, et les tests iOS vérifient qu'il n'en apparaît pas par
inadvertance.

Le jour où l'on vend une place dans la file, on a construit exactement ce qu'on
cherchait à éviter.

## Les trois règles qui en découlent

### On demande en écrivant

Il n'existe aucun geste pour dire « je viens ». Une demande porte un message
d'au moins `REQUEST_MIN_CHARS` caractères. Le seuil n'est pas décoratif : c'est
ce qui distingue une demande d'un réflexe, et c'est la même règle des deux côtés
du réseau.

Conséquence directe : il n'y a pas de « likes reçus », donc pas de file
d'attente à monétiser, donc pas de compteur à faire grossir.

### Un plan a une date, donc une fin

Passé le rendez-vous (plus une tolérance de `PLAN_GRACE_MINUTES`), le plan sort
du fil. Rien ne s'accumule, rien ne traîne, il n'y a pas de retour en arrière
payant. Et si personne ne demande à venir, on fait ce qu'on avait prévu — c'était
l'idée de départ. Aucun compteur ne vient le rappeler.

### Une conversation naît d'un oui, et de rien d'autre

Elle est toujours **à deux**, même sur un plan de groupe : on parle à quelqu'un,
pas à une salle. Pas d'accusé de lecture visible par l'autre, pas d'indicateur
de frappe, pas de « vu à » — ces mécaniques servent à retenir, pas à se parler.

## Le déroulé

1. **Inscription** — adresse e-mail, code à six chiffres. Pas de mot de passe.
2. **Fiche** — une ville, une position arrondie au kilomètre, un genre, une
   phrase si l'on veut. Pas de questionnaire : ce n'est pas ce qu'on va lire.
3. **Publier un plan** — ce qu'on fait, quand, combien de places. Au moins
   `PLAN_MIN_LEAD_MINUTES` minutes à l'avance.
4. **Lire le fil** — les plans autour de soi, du plus imminent au plus lointain.
5. **Demander à venir** — quelques lignes qui disent pourquoi ce plan-là.
6. **Accepter, ou non** — un refus ne se commente pas et ne notifie rien
   d'accusateur. Quand la dernière place part, les demandes restantes sont
   closes.
7. **La conversation s'ouvre** — et l'endroit exact se dit là, à qui l'on a
   accepté. Jamais dans le fil.

## Ce que Weave n'a pas, et n'aura pas

- Pile de cartes à balayer
- Liste des personnes qui vous ont remarqué
- Mise en avant payante dans le fil
- Compteur de vues de votre profil
- Publicité, revente ou courtage de données
- Notification inventée pour vous faire revenir

Ces absences ne sont pas des manques de la version 1 : elles sont le produit.
