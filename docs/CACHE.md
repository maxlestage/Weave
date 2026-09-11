# Le cache

Redis n'est pas un confort dans Weave : **l'invariant central du produit y vit
tout entier.** Ce document explique ce qu'on y met, pourquoi, et où sont les
limites.

## Ce que Redis porte

| Ce qui est stocké | Pourquoi là, et pas en base |
| --- | --- |
| **Le quota de demandes du jour** | C'est un compteur à haute fréquence, remis à zéro chaque nuit, qui doit se décrémenter de façon atomique. Aucune ligne de base n'a besoin d'en garder trace le lendemain |
| **Le fil composé** | Il dépend de la position, des critères et de l'heure ; il se périme en minutes. Le recalculer à chaque ouverture coûterait une requête lourde pour un résultat identique |
| **Le résumé d'identité** | Évite un aller-retour base à chaque requête authentifiée |
| **Les états Live Activity et montre** | Une donnée d'affichage, poussée quelques fois par jour |
| **Les compteurs de limitation de débit** | Fenêtres courtes, sans valeur historique |

Les plans, eux, **vivent en base**. Ce sont des engagements datés que leurs
auteurs ont écrits : les perdre serait perdre le produit. C'est une différence
de nature avec le fil, qui n'est qu'une vue calculée.

## Les clés

| Clé | Type | Contenu | TTL |
| --- | --- | --- | --- |
| `weave:v2:feed:<compte>` | String | Fil composé, en JSON | 5 min |
| `weave:v2:req:<compte>:<jour>` | String | Demandes déjà envoyées aujourd'hui | jusqu'à minuit |
| `weave:v2:me:<compte>` | String | Résumé d'identité | 15 min |
| `weave:v2:la:<compte>` | String | Dernier état poussé vers la Live Activity | 24 h |
| `weave:v2:watch:<compte>` | String | Résumé compact pour watchOS | 24 h |
| `weave:v2:rl:<seau>:<sujet>` | String | Compteur de limitation de débit | fenêtre |

**Toute entrée porte un TTL.** `setJson` refuse un TTL nul ou négatif : une clé
sans expiration serait un stockage déguisé.

## Le quota, appliqué de façon atomique

Deux demandes envoyées simultanément ne doivent pas pouvoir dépenser la même
unité. La vérification et l'écriture sont donc faites en un seul script Lua,
exécuté par Redis :

```lua
local utilisees = redis.call('INCR', KEYS[1])
if utilisees == 1 then
  redis.call('EXPIRE', KEYS[1], expiration)
end

if utilisees > plafond then
  redis.call('DECR', KEYS[1])   -- on annule son propre incrément
  return -1
end

return plafond - utilisees
```

L'ordre compte : on incrémente **puis** on compare. Une lecture suivie d'une
écriture en deux temps laisserait passer deux demandes concurrentes sur la
dernière place.

L'expiration est posée sur la **première** demande du jour, et calculée jusqu'à
minuit dans le fuseau de la personne — pas 24 h glissantes. Un quota qui se
recharge à une heure différente chaque jour est incompréhensible.

### Le prélèvement précède l'écriture

Dans `requests.routes.ts`, le quota est consommé **avant** l'insertion de la
demande, et rendu si l'insertion échoue. L'inverse laisserait une demande écrite
gratuitement en cas d'erreur — c'est-à-dire un trou dans l'invariant, exploitable
en provoquant des erreurs.

### Le remboursement

Une demande **retirée** rend son unité, avec une précision qui compte : le jour
crédité est celui de **l'envoi**, pas celui du retrait. Sans cela, retirer une
demande après minuit créditerait une journée qu'on n'a pas entamée.

Une demande **refusée** ne rend rien : elle a été lue.

## L'invalidation du fil

Trois événements invalident un fil :

- **ses propres critères changent** — `invalidateFeed(compte)` ;
- **un blocage** — les deux fils, dans les deux sens ;
- **un plan est publié ou annulé** — `invalidateAllFeeds()`, car il doit
  apparaître ou disparaître sans attendre l'expiration des fils déjà composés.

`invalidateAllFeeds` utilise `SCAN`, pas `KEYS` : cette dernière bloque le
serveur le temps du parcours, ce qui est acceptable sur un jeu de développement
et ne l'est plus en production.

## Ce qu'un redémarrage de Redis emporte

Les fils composés, les compteurs de débit, et **les quotas du jour**. Ce dernier
point mérite d'être dit franchement : après un redémarrage, tout le monde
récupère ses demandes du jour.

C'est un compromis assumé. L'alternative — écrire chaque demande consommée en
base — ajouterait une écriture synchrone sur le chemin le plus chaud du produit
pour se prémunir d'un incident rare, dont la conséquence est qu'une poignée de
gens envoient quelques messages de plus un soir. Ce n'est pas une fraude à la
facturation : le quota n'est pas un bien qu'on vend, c'est une règle de
conception.

Ce qui survit : les comptes, les profils, **les plans**, les demandes, les
conversations et les messages.

## Dimensionner le cache

Mesure relevée sur le jeu de données de développement (fil de 22 plans,
sérialisé tel qu'il est mis en cache) :

| | |
| --- | --- |
| Un plan dans un fil | ~476 octets |
| Un fil plein (60 plans, `FEED_SIZE`) | ~28 Ko |
| Compteur de quota | quelques octets |
| Plan Heroku « mini » (25 Mo) | ~850 fils pleins simultanément en cache |

Les photos du jeu de développement sont absentes : une URL signée réelle ajoute
une centaine d'octets par plan. Comptez plutôt 35 Ko pour un fil plein en
production.

Un fil expire en cinq minutes : ce sont donc des personnes **actives dans les
cinq dernières minutes**, pas des inscrites. La marge est confortable, mais
surveillez `used_memory` : une éviction ne casse rien, elle fait seulement
recomposer un fil.

L'éviction d'un **compteur de quota**, elle, rend des demandes. C'est la raison
de la politique recommandée ci-dessous.

## Points de vigilance en exploitation

- **Éviction.** Configurer Redis en `maxmemory-policy volatile-ttl` : les fils,
  qui ont le TTL le plus court, sont évincés avant les quotas.
- **Persistance.** Un instantané RDB toutes les cinq minutes suffit ; l'AOF
  n'apporte pas grand-chose pour une donnée dont la durée de vie se compte en
  minutes.
- **Sonde.** `/health` renvoie 503 si Redis ne répond pas. Un service qui
  accepterait des demandes sans pouvoir compter les quotas mentirait sur son
  invariant principal.
