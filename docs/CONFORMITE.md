# Conformité et protection des données

> Comme [ORIGINALITE.md](./ORIGINALITE.md), ce document consigne des choix
> techniques, pas un avis juridique. Une revue par un conseil et la désignation
> d'un délégué à la protection des données sont nécessaires avant tout
> lancement.

## Pourquoi c'est structurant ici

Une application de rencontre traite des données qui relèvent de l'article 9 du
RGPD — l'orientation sexuelle se déduit des critères de recherche, et la
localisation trace les déplacements. Ce n'est pas un domaine où l'on ajoute la
conformité après coup.

## Ce que l'architecture apporte d'elle-même

| Principe RGPD | Ce qui le sert, concrètement |
| --- | --- |
| Minimisation | Les profils proposés ne sont jamais archivés : seul un registre d'identifiants subsiste |
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
| Accès et portabilité | Export à construire à partir de `Account`, `Profile`, `ProfileFragment`, `MotifTag`, `WovenThread`, `Message` — le cache n'a rien à exporter, il est vide de données durables |
| Rectification | `PATCH /v1/me`, `PUT /v1/me/profile`, `PUT /v1/me/motif` |
| Effacement | `DELETE /v1/auth/account` : sortie immédiate de la composition, purge sous 30 jours |
| Opposition | `POST /v1/me/pause` : le compte cesse d'être proposé et de recevoir des fils |
| Limitation | La mise en pause vide le métier sans supprimer le compte |

**À construire avant le lancement** : la route d'export au format lisible par
machine, et la tâche planifiée qui exécute réellement les purges (la structure
est en place — `deletionRequestedAt`, `purgeAfter` — l'exécution périodique
reste à brancher).

## Sécurité des personnes

- Le **blocage** est immédiat et retire le fil des deux côtés, sans notification
  à l'autre partie.
- Un **signalement entraîne toujours un blocage** : personne n'a à revoir un
  profil qu'il vient de signaler pendant l'examen du dossier.
- La **position n'est jamais exposée** : seule une distance arrondie l'est.
- Aucun **contenu de conversation** ne transite par APNs ni ne s'affiche sur
  l'écran verrouillé — un prénom et un décompte, rien de plus.

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
