# « Les profils du jour, en cache uniquement »

C'est l'exigence structurante de Weave. Ce document explique comment elle est
tenue, et où sont les limites.

## Ce que la règle veut dire, précisément

1. Un utilisateur détient **au plus douze fils actifs** (`MAX_ACTIVE_THREADS`), à tout instant.
2. Le **contenu** de ces fils — prénom, âge, ville, motif, fragments, photo —
   n'existe que dans Redis, avec une durée de vie.
3. La base relationnelle ne contient **aucune copie** de ce contenu.
4. Aucun palier d'abonnement ne relève ce plafond.

## Les clés

| Clé | Type | Contenu | TTL |
| --- | --- | --- | --- |
| `weave:v1:loom:<compte>` | ZSET | Identifiants des fils, score = expiration en ms | 48 h |
| `weave:v1:thread:<fil>` | String | La carte complète, en JSON | 24 h (48 h max) |
| `weave:v1:pool:<compte>` | String | Vivier de candidats pré-calculé | 30 min |
| `weave:v1:refill:<compte>` | String | Date de regarnissage de la prochaine place | 24 h |
| `weave:v1:me:<compte>` | String | Résumé d'identité, pour éviter un aller-retour base | 15 min |
| `weave:v1:la:<compte>` | String | Dernier état poussé vers la Live Activity | 48 h |
| `weave:v1:watch:<compte>` | String | Résumé compact pour watchOS | 48 h |
| `weave:v1:rl:<seau>:<sujet>` | String | Compteur de limitation de débit | fenêtre |
| `weave:v1:lock:weave:<compte>` | String | Verrou de composition | 10 s |

**Toute entrée porte un TTL.** `setJson` refuse un TTL nul ou négatif : une clé
sans expiration serait un stockage déguisé, et c'est précisément ce que la règle
interdit.

## Le plafond, appliqué de façon atomique

Deux ouvertures simultanées de l'application ne doivent pas pouvoir produire un
quatrième fil. La vérification et l'écriture sont donc faites en un seul script
Lua, exécuté par Redis :

```lua
redis.call('ZREMRANGEBYSCORE', KEYS[1], '-inf', now)   -- purge des fils échus
if redis.call('ZCARD', KEYS[1]) >= maximum then
  return -1                                            -- métier plein
end
redis.call('ZADD', KEYS[1], expiresAt, threadId)
redis.call('EXPIRE', KEYS[1], loomTtl)
redis.call('SET', KEYS[2], payload, 'EX', ttl)
return redis.call('ZCARD', KEYS[1])
```

Un test d'intégration lance six lectures concurrentes du métier sur un compte
vide et vérifie que `ZCARD` ne dépasse jamais le plafond.

## Ce qui est écrit en base, et pourquoi

Une seule table concerne les propositions : `thread_ledger`.

```
id · viewerId · candidateId · cacheKey · outcome · score · servedAt · resolvedAt · expiresAt
```

Rien d'autre. Pas de prénom, pas de photo, pas de fragment, pas de motif.

Elle existe pour trois raisons, et aucune ne pourrait être satisfaite par le
cache seul :

- **Ne jamais reproposer la même personne.** Une contrainte d'unicité
  `(viewerId, candidateId)` le garantit, y compris après expiration du cache.
- **Mesurer la qualité de la composition.** Le score et l'issue permettent de
  savoir si le moteur propose bien, sans jamais avoir à relire un profil.
- **Réconcilier après incident.** Si Redis redémarre, les fils sont perdus — et
  c'est acceptable — mais on sait qui avait déjà été proposé.

Un test d'intégration énumère les colonnes réellement présentes sur les lignes
de registre et échoue si une colonne s'y ajoute : c'est le garde-fou contre la
dérive, plus fiable qu'une note dans un document.

## Ce qu'un redémarrage de Redis emporte

Les fils en cours, les viviers, les compteurs de débit. C'est assumé : à la
prochaine ouverture, le métier se regarnit à partir de personnes qui n'avaient
pas encore été proposées.

Ce qui survit : les comptes, les profils, les motifs, les conversations déjà
tissées, et le registre.

## Le passage du cache à la base

Un fil ne devient persistant qu'au **deuxième échange** — quand chacun a
répondu. À ce moment seulement, une ligne `woven_threads` et les messages sont
écrits. Avant cela, une proposition qui n'aboutit pas ne laisse qu'une ligne de
registre.

C'est la frontière du produit : ce qui est resté sans réponse ne s'archive pas.

## Dimensionner le cache

Le plafond de fils se paie directement en mémoire Redis. Mesure relevée sur le
jeu de données de développement, cartes réelles en cache :

| | |
| --- | --- |
| Taille d'une carte de fil | ~1,4 Ko |
| Un utilisateur au métier complet (12 fils) | ~16 Ko |
| Plan Heroku « mini » (25 Mo) | ~1 500 utilisateurs au métier complet |

Ce sont des utilisateurs **simultanément actifs**, pas des inscrits : une carte
expire au bout de 24 h. Mais le seuil arrive vite, et il arrive quatre fois
plus vite qu'avec un plafond de trois fils. Surveillez `used_memory` dès les
premières centaines d'utilisateurs, et prévoyez de quitter le plan « mini »
bien avant de l'atteindre — une éviction fait disparaître des fils en cours.

## Points de vigilance en exploitation

- **Éviction.** Configurer Redis en `maxmemory-policy volatile-ttl` et
  surveiller la mémoire : une éviction fait disparaître des fils en cours. Le
  produit y survit, mais l'expérience se dégrade silencieusement.
- **Persistance.** Un instantané RDB toutes les cinq minutes suffit ; l'AOF
  n'apporte pas grand-chose pour une donnée dont la durée de vie est de 24 h.
- **Sonde.** `/health` renvoie 503 si Redis ne répond pas. Un service qui
  accepterait des requêtes sans cache mentirait sur ce qu'il peut faire.
