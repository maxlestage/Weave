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
| Accès et portabilité | `GET /v1/me/export` — un document JSON complet ; le cache n'a rien à exporter, il est vide de données durables |
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
l'intervalle, et part au passage suivant une fois le dossier clos — par
`weave-api console clore`, voir plus bas — ou au bout de quatre-vingt-dix
jours si le dossier n'a jamais été instruit. Sans cette borne, une exception à
l'article 17 serait devenue une exemption permanente, et un seul signalement
aurait suffi à empêcher définitivement la suppression du compte d'autrui.

**Elle tourne d'elle-même**, depuis le service : une tentative par heure, un
verrou dans le cache tenu vingt-trois heures. Au plus un passage par jour, quel
que soit le nombre de dynos. Rien à planifier, rien à installer — c'est ce qui
séparait encore le code écrit du droit exercé.

La commande `weave-api purge` reste disponible pour un appel externe ; le
verrou vaut pour elle aussi.

Côté iOS, les réglages portent désormais « Obtenir mes données » — qui appelle
la route et propose le fichier au partage — et « Supprimer mon compte », qu'Apple
exige de toute application permettant d'en créer un. Ce code Swift **n'a jamais
été compilé** : ni macOS ni compte Apple Developer n'étaient disponibles. Il
suit les idiomes du client existant, et devra être repris à la première
ouverture du projet dans Xcode.

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
  même un prénom. L'alerte qui signale un message arrivé ne dit que cela :
  « Nouveau message », sans auteur ni extrait. Une notification se lit
  par-dessus une épaule.
- **Les messages échangés ne sont pas supprimés au blocage** : ils restent
  lisibles par la personne qui a bloqué jusqu'à la purge. Une suppression
  immédiate effacerait aussi les preuves d'un comportement qu'on vient de
  signaler.

## Modération

`reports` porte un motif normalisé et un état de traitement. Les motifs
prévoient explicitement `mineur`, qui doit déclencher un traitement prioritaire
et, le cas échéant, un signalement aux autorités compétentes.

**`weave-api console`** est l'outil qui pose ces décisions. Trois colonnes
existaient sans que rien ne les écrive : `reports.handledAt` — la clôture d'un
dossier, lue par la purge et écrite nulle part ; `accounts.status = 'suspended'`
— posé d'office sur un signalement de minorité, avec en commentaire « la
suspension se lève à la main », qu'aucune main ne pouvait lever ;
`profiles.photoReviewedAt` — remis à `NULL` à chaque envoi de photo et jamais
relu.

```sh
heroku run -a weave ./bin-release/weave-api console signalements
heroku run -a weave ./bin-release/weave-api console clore <dossier> "motif"
heroku run -a weave ./bin-release/weave-api console suspendre <compte> "motif"
heroku run -a weave ./bin-release/weave-api console retablir <compte>
heroku run -a weave ./bin-release/weave-api console verifier <compte> "motif"
heroku run -a weave ./bin-release/weave-api console deverifier <compte> "motif"
heroku run -a weave ./bin-release/weave-api console photos
heroku run -a weave ./bin-release/weave-api console photo-ok <compte>
heroku run -a weave ./bin-release/weave-api console photo-retirer <compte>
```

**Pourquoi une commande et pas une route.** Une surface d'administration en
HTTP demanderait une authentification d'administrateur — un rôle, un second
chemin de connexion, et une porte de plus sur l'internet public : celle-là même
qui donne le pouvoir de suspendre un compte et de lire les détails d'un
signalement. La commande ne pose aucune porte ; l'autorisation, c'est l'accès
au dyno, donc le compte Heroku et son second facteur.

**Clore et sanctionner sont deux gestes.** Clore un dossier ne suspend
personne, et suspendre ne clôt aucun dossier. Les confondre ferait d'un
classement sans suite une sanction silencieuse, ou l'inverse. Chaque décision
s'inscrit à `audit_events` avec son motif : le règlement européen sur les
services numériques (DSA) impose une voie de recours pour les décisions de
modération, et une décision sans trace ne se conteste pas. `retablir` est cette
voie.

**Le badge « vérifié »** relevait du même défaut : `accounts.verified` est
affiché par le fil et la liste des demandes, écrit `false` à l'inscription, et
remis à `true` par rien. « Vérification de profil accélérée », vendue avec le
palier Grand Tour, portait donc sur une procédure qui n'existait à aucune
vitesse. Le badge dit désormais ce qu'il peut dire : qu'une personne a examiné
une pièce et consigné sa décision avec son motif. Il ne dit pas qu'un contrôle
automatique a eu lieu — il n'y en a aucun, et **aucune page publique ne définit
encore ce que le badge affirme**. Cette page-là reste à écrire.

**La vérification se demande** depuis Réglages › Vérification, et la demande
entre dans une file que `console verifications` relève. Le badge se posait
depuis la console, et personne ne pouvait le demander — le Grand Tour vendait
par ailleurs une priorité dans une file qui n'existait pas.

`verification_requests` ne porte **aucune pièce d'identité** : une demande, un
mot libre facultatif, et la décision avec son motif. La suite se fait par
courrier. Conserver des papiers d'identité créerait une réserve de données
sensibles dont la perte serait irréparable, pour un service qui n'en a pas
besoin — et l'article 9 s'appliquerait à cette réserve comme au reste.

Le motif d'un refus est **rendu à la personne** : les mentions légales
promettent qu'une décision de modération se conteste, et un refus dont on ignore
la raison ne se conteste pas. Les CGU disent désormais ce que le badge affirme,
et surtout ce qu'il n'affirme pas — aucun contrôle automatique, aucune
vérification d'identité officielle, et rien qui garantisse l'âge ou les photos.

**Ce qui reste à définir** est le processus humain, pas l'outil : qui relève la
file, sous quel délai, et selon quelle grille. Les photos sont publiées avant
examen — la file est donc a posteriori, et `photos` la donne de la plus
ancienne à la plus récente.

## Consentement de l'article 9

Les personnes que l'on cherche, rapprochées de son propre genre, peuvent
révéler l'orientation sexuelle. Le règlement range cette information parmi les
catégories particulières de l'article 9 : elle ne peut être traitée que sur un
**consentement explicite et distinct**.

La politique de confidentialité le promet depuis le début, et nomme même
l'écran où le retirer. `consent_records` existait pour le consigner, avec son
objet, sa version, sa date d'octroi et sa date de retrait — et **rien ne
l'écrivait**. Aucune route, aucun parcours. Le critère de genre s'enregistrait
sans qu'aucun consentement n'ait jamais été demandé, l'export « vos
consentements » rendait une liste vide à tout le monde, et « Réglages ›
Confidentialité » désignait un écran qui n'existait pas.

| Promesse de la page | Ce qui la tient |
| --- | --- |
| demandé séparément, jamais par une case unique | `POST /v1/me/consents`, un objet à la fois ; les autres critères ne demandent rien |
| retirable à tout moment depuis l'application | `POST /v1/me/consents/revoke`, et l'écran Réglages › Confidentialité |
| le service continue, avec un fil non filtré sur ce critère | le retrait efface `seekingJson` ; le fil cesse d'y filtrer |
| le retrait est enregistré avec sa date | `revokedAt` posé sur tout enregistrement encore ouvert |
| la période d'activité est conservée | les enregistrements ne sont jamais supprimés, seulement datés |
| un changement substantiel fait redemander le consentement | un consentement porte `POLICY_VERSION` ; une version antérieure cesse de valoir |

**Retirer, c'est arrêter le traitement.** Un retrait qui laisserait le critère
en base ferait durer un traitement de données sensibles sans base légale. Le
critère est donc effacé ; la trace du consentement, elle, reste — c'est la
période d'activité qui permet d'établir que le traitement était licite quand il
a eu lieu.

**La péremption est le chemin sans écriture.** Le retrait efface le critère,
mais un changement de version ne repasse sur aucune ligne : le critère reste en
base alors que le consentement ne vaut plus. Le fil relit donc le consentement à
chaque composition, et c'est ce qui fait que le traitement s'arrête vraiment
plutôt qu'à la prochaine fois que la personne touche à ses critères.

**La version ne s'écrit qu'une fois.** `POLICY_VERSION` et la date affichée au
bas des pages juridiques viennent du même endroit, et un test tient leur
accord : deux dates écrites séparément auraient fini par attester de deux
textes différents.

## Ce que les paliers limitent vraiment

Chaque critère vendu par palier est relu **à la lecture du fil**, et pas
seulement contrôlé à l'écriture. Sans cela, un abonnement qui expire laisserait
en place les critères posés du temps où il courait : on continuerait de
bénéficier de ce qu'on ne paie plus, et il aurait suffi de s'abonner un mois.

| Critère | Ce que le palier change |
| --- | --- |
| genre recherché | proposé à partir de la Virée, et soumis au consentement de l'article 9 |
| catégorie, jour | proposés à partir de l'Escapade (« critères précis ») |
| distance | au kilomètre près à partir de l'Escapade ; ailleurs, rabattue sur 10, 25, 50 ou 100 km |

La distance était l'exception : elle se réglait au kilomètre près **à tous les
paliers**, y compris le gratuit, alors que « distance fine » figure trois fois
au catalogue. On vendait un critère qui existait déjà partout.

Le réglage choisi n'est jamais écrasé en base — seulement rabattu à la lecture.
Un abonnement qui s'interrompt ne fait donc pas perdre ce qu'on avait réglé, et
le rayon exact revient dès que l'offre le permet. `/v1/me/preferences` rend les
deux valeurs, pour que l'application n'affiche pas un rayon que le fil
n'applique pas.

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
