# Originalité et prudence juridique

> **Ce document n'est pas un avis juridique.** Il consigne les choix de
> conception faits pour éloigner Weave des mécaniques identifiables des
> applications existantes, et les points à faire valider par un conseil en
> propriété intellectuelle avant le lancement. Une revue de brevets (freedom to
> operate) et un dépôt de marque restent nécessaires — ce document ne les
> remplace pas.

## Le principe retenu

Plutôt que de reprendre une mécanique connue en la renommant — ce qui protège
peu, puisqu'un brevet porte sur une fonction et non sur un nom — Weave part d'un
**objet différent** : on ne publie pas un profil, on publie un plan daté auquel
d'autres demandent à se joindre, par écrit, dans la limite d'un quota
journalier. Les mécaniques usuelles n'y ont pas leur place, non par prudence,
mais parce qu'elles n'ont plus de sens dans ce cadre : il n'y a rien à balayer,
et un plan ne se « superlike » pas.

C'est la position la plus solide : une fonction qu'on n'a pas ne peut pas
contrefaire.

## Mécaniques écartées, et ce que Weave fait à la place

| Mécanique répandue | Pourquoi elle est écartée | Ce que fait Weave |
| --- | --- | --- |
| Balayage gauche/droite sur une pile de cartes pour exprimer un intérêt | Fonction revendiquée par des brevets détenus par Match Group, notamment US 9 733 811 ; c'est le point le plus exposé du secteur | Aucune pile, aucun geste d'appréciation. On écrit une demande, ou l'on ne fait rien |
| Double consentement silencieux, puis écran d'annonce de correspondance | Mécanique et présentation fortement associées à un acteur identifié | Il n'y a pas d'instant « correspondance ». Une personne accepte une demande qu'elle a lue ; la conversation commence sur ce qui était déjà écrit |
| Délai imposé à un genre pour écrire en premier | Mécanique associée à Bumble, et discutable en soi | Le seul compte à rebours est celui du rendez-vous lui-même, identique pour tout le monde, sans distinction de genre |
| File des « personnes qui vous ont aimé », débloquée par abonnement | Le levier commercial le plus copié — et celui qui produit la dynamique que Weave refuse | N'existe pas. Il n'y a pas de like, donc pas de file. Les demandes reçues sont lisibles gratuitement par l'auteur du plan |
| Mise en avant payante dans le fil | Vend de la visibilité, pas de la rencontre | N'existe pas. `PAID_VISIBILITY` vaut littéralement `false`, et le tri n'a que deux termes : imminence, proximité |
| Marques verbales et signes du secteur (flamme, cœur, « super » quelque chose, « rembobiner », « passeport ») | Risque de marque et de présentation trompeuse | Lexique du départ et du trajet, distinct du secteur : plan, fil, demande, Départ, Virée, Escapade, Expédition, Grand Tour, Renfort, Horizon, Tablée, Escale, Bilan |

## Ce qui appartient en propre à Weave

Ces éléments constituent l'identité du produit et sont ceux à protéger :

1. **L'objet publié est un plan daté**, pas un profil : un rendez-vous à venir,
   avec un nombre de places, qui disparaît une fois passé.
2. **Le quota de demandes journalier**, borné à tous les paliers — y compris au
   plus cher et y compris après achat d'un « Renfort », dont le nombre est
   lui-même plafonné par jour.
3. **La demande écrite obligatoire**, avec un plancher de caractères, à
   l'exclusion de tout geste binaire.
4. **Le tri du fil à deux termes seulement** — imminence puis proximité — et
   l'engagement explicite qu'aucun troisième terme ne sera achetable.
5. **La conversation à deux même sur un plan de groupe**, ouverte par une
   acceptation et par rien d'autre.
6. **La Live Activity « prochain plan »**, qui n'expose ni nom, ni photo, ni
   message sur un écran verrouillé.
7. **Le lexique du départ**, cohérent du code jusqu'à l'interface.

## Points à faire valider avant le lancement

- [ ] Recherche d'antériorité de marque sur « Weave » dans les classes 9, 42 et
      45, dans les territoires visés — le mot est courant en anglais, ce qui
      affaiblit son caractère distinctif : envisager un signe combiné
      (logotype des trois fils) plutôt que le mot seul.
- [ ] Revue de brevets (freedom to operate) sur la publication d'événements
      géolocalisés avec demande de participation, et sur le démarrage de Live
      Activity à distance. Le domaine des « plans entre inconnus » est plus
      proche des applications d'événements que de la rencontre classique :
      élargir la recherche à ce secteur.
- [ ] Vérification que le vocabulaire retenu ne heurte aucune marque déposée du
      secteur dans les territoires visés.
- [ ] Conditions générales et politique de confidentialité rédigées par un
      conseil, pas dérivées de celles d'un concurrent — la reprise d'un texte
      contractuel est un risque de contrefaçon d'œuvre littéraire, souvent
      sous-estimé.
- [ ] Mentions Apple : « Apple », « iPhone », « Apple Watch » sont des marques
      déposées, à citer selon les règles d'usage d'Apple.

## Sur le code

Le code de Weave n'incorpore aucune bibliothèque tierce dont la licence
imposerait la réciprocité (aucune dépendance sous GPL ou AGPL). Les dépendances
utilisées — Elysia, Prisma, React, Vite, Tailwind — sont sous licence MIT ou
Apache 2.0.
