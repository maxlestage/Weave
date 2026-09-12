# Conformité et protection des données

> Comme [ORIGINALITE.md](./ORIGINALITE.md), ce document consigne des choix
> techniques, pas un avis juridique. Une revue par un conseil et la désignation
> d'un délégué à la protection des données sont nécessaires avant tout
> lancement.

## Pourquoi c'est structurant ici

Une application de rencontre traite des données qui relèvent de l'article 9 du
RGPD — l'orientation sexuelle se déduit des critères de recherche, et la
localisation trace les déplacements. Weave en ajoute une couche : **un plan dit
où l'on sera, et quand.** C'est une donnée de déplacement future, plus sensible
qu'une position passée. Ce n'est pas un domaine où l'on ajoute la conformité
après coup.

## Ce que l'architecture apporte d'elle-même

| Principe RGPD | Ce qui le sert, concrètement |
| --- | --- |
| Minimisation | Le profil est réduit à une ville, un genre et une phrase ; le fil consulté n'est jamais archivé ; un plan passé sort du fil |
| Limitation de la conservation | Toute entrée de cache porte un TTL ; les messages sont purgés 90 jours après clôture ; un compte supprimé est purgé sous 30 jours |
| Exactitude | Les coordonnées sont arrondies au dépôt : on ne stocke jamais mieux que ~1 km |
| Intégrité et confidentialité | Codes hachés en Argon2id, jetons de rafraîchissement hachés et rotatifs, médias sous URL signée à durée limitée |
| Responsabilité | `consent_records` horodate chaque consentement avec la version du texte accepté ; `audit_events` journalise les actions sensibles |

## Base légale et consentement

`consent_records` distingue quatre objets : conditions générales, politique de
confidentialité, **données sensibles**, mesure d'audience. Chacun porte sa
version, sa date d'octroi et sa date de révocation. Un consentement retiré reste
tracé — c'est ce qui permet de prouver la période pendant laquelle le traitement
était licite.

Le consentement aux données sensibles doit être recueilli **séparément**, pas
par une case unique valant acceptation de tout.

## Droits des personnes

| Droit | Où c'est traité |
| --- | --- |
| Accès et portabilité | Export à construire à partir de `Account`, `Profile`, `Preference`, `Plan`, `JoinRequest`, `Conversation`, `Message` — le cache n'a rien à exporter, il est vide de données durables |
| Rectification | `PATCH /v1/me`, `PUT /v1/me/profile`, `PATCH /v1/me/preferences` |
| Effacement | `DELETE /v1/auth/account` : les plans ouverts sont retirés du fil immédiatement, purge sous 30 jours |
| Opposition | `POST /v1/me/pause` : les plans ouverts sont annulés, le compte n'apparaît plus dans le fil |
| Limitation | La mise en pause retire du fil sans supprimer le compte |

## L'export et la purge

Les deux existent désormais, et ce n'est pas un détail : jusqu'ici
`deletionRequestedAt` et `purgeAfter` décrivaient une intention que rien
n'exécutait. Les lignes restaient en base indéfiniment.

**`GET /v1/me/export`** rend en un seul document JSON tout ce que le service
détient d'un compte. Trois choses en sont volontairement absentes, et chacune
pour une raison qui tient : les empreintes (adresse, jetons, codes) — ce sont
nos données sur la personne, pas les siennes, et les rendre affaiblirait le
compte ; les messages écrits par d'autres — ils appartiennent aussi à leur
auteur ; et les signalements reçus — les rendre livrerait qui a signalé.

**`weave-api purge`** est un processus séparé, comme `migrate`. Il efface les
messages dont la date de purge est passée, puis les comptes dont le délai de
trente jours est écoulé. La suppression d'un compte suffit à emporter tout ce
qui s'y rattache : les vingt-trois clés étrangères sont en `ON DELETE CASCADE`,
et `audit_events` en `SET NULL` — la trace de l'action survit, son auteur
devient anonyme. Un test vérifie que la cascade s'applique réellement, y
compris sous SQLite, où les clés étrangères ne sont pas actives par défaut.

**L'exception à retenir** : un compte visé par un signalement non traité n'est
pas purgé. Sinon, supprimer son compte suffirait à effacer les preuves d'un
comportement qu'on vient de signaler. Le compte reste hors circulation dans
l'intervalle, et part au passage suivant une fois le dossier clos.

À planifier sur l'hébergeur — Heroku Scheduler, ou l'équivalent — une fois par
jour :

```sh
weave-api purge
```

Sans cette planification, la purge n'a toujours pas lieu : le code existe, son
déclenchement est une question d'exploitation.

**Reste à construire** : l'écran « Mes données » dans l'application iOS, qui
appellera la route d'export. La route, elle, est en place et testée.

## Sécurité des personnes

- Le **blocage** est immédiat et coupe tout des deux côtés : demandes en attente
  closes, conversation fermée, plans retirés du fil de l'autre — sans
  notification à la personne bloquée.
- Un **signalement entraîne toujours un blocage** : personne n'a à revoir les
  plans de qui il vient de signaler pendant l'examen du dossier.
- La **position n'est jamais exposée** : un plan affiche une ville et une
  distance arrondie, jamais une adresse. **L'endroit exact se dit dans la
  conversation**, à qui l'on a accepté — c'est la règle la plus importante de
  cette page, parce qu'un lieu et une heure publiés largement sont ce qu'une
  application comme celle-ci peut faire de plus dangereux.
- Aucun **contenu de conversation** ne transite par APNs ni ne s'affiche sur
  l'écran verrouillé — un titre de plan et deux compteurs, rien de plus. Pas
  même un prénom.
- **Les messages échangés ne sont pas supprimés au blocage** : ils restent
  lisibles par la personne qui a bloqué jusqu'à la purge. Une suppression
  immédiate effacerait aussi les preuves d'un comportement qu'on vient de
  signaler.

## Modération

`reports` porte un motif normalisé et un état de traitement. Les motifs
prévoient explicitement `mineur`, qui doit déclencher un traitement prioritaire
et, le cas échéant, un signalement aux autorités compétentes.

Le règlement européen sur les services numériques (DSA) impose par ailleurs un
point de contact, des délais de traitement des signalements et une voie de
recours pour les décisions de modération. `reports.state` et `reports.handledAt`
en posent la base technique ; le processus humain reste à définir.

## Sous-traitants

Le cœur de Weave ne dépend d'aucun service tiers de traitement de données
personnelles. Les intégrations à prévoir — envoi d'e-mails transactionnels,
stockage objet des médias, hébergement — devront figurer au registre des
traitements, avec les accords de sous-traitance correspondants et une
vérification des transferts hors Union européenne.

Apple traite les données de paiement : aucun moyen de paiement ne transite par
nos serveurs.

## Journalisation

La journalisation est structurée et volontairement pauvre en données
personnelles : identifiants de compte, jamais d'adresse e-mail en clair (les
demandes de code sont journalisées par empreinte). `audit_events` conserve les
actions sensibles avec leur auteur et leur sujet.
