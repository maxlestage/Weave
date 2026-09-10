# Originalité et prudence juridique

> **Ce document n'est pas un avis juridique.** Il consigne les choix de
> conception faits pour éloigner Weave des mécaniques identifiables des
> applications existantes, et les points à faire valider par un conseil en
> propriété intellectuelle avant le lancement. Une revue de brevets (freedom to
> operate) et un dépôt de marque restent nécessaires — ce document ne les
> remplace pas.

## Le principe retenu

Plutôt que de reprendre une mécanique connue en la renommant — ce qui protège
peu, puisqu'un brevet porte sur une fonction et non sur un nom — Weave part
d'une **contrainte différente** : trois profils simultanés, en cache, engagés
par une réponse écrite. Les mécaniques usuelles n'y ont pas leur place, non par
prudence, mais parce qu'elles n'ont plus de sens dans ce cadre.

C'est la position la plus solide : une fonction qu'on n'a pas ne peut pas
contrefaire.

## Mécaniques écartées, et ce que Weave fait à la place

| Mécanique répandue | Pourquoi elle est écartée | Ce que fait Weave |
| --- | --- | --- |
| Balayage gauche/droite sur une pile de cartes pour exprimer un intérêt | Fonction revendiquée par des brevets détenus par Match Group, notamment US 9 733 811 ; c'est le point le plus exposé du secteur | Aucune pile, aucun geste d'appréciation. On répond à un fragment, par écrit |
| Double consentement silencieux, puis écran d'annonce de correspondance | Mécanique et présentation fortement associées à un acteur identifié | Il n'y a pas d'instant « correspondance ». Le fil se tisse quand chacun a répondu, et la conversation était déjà commencée |
| Délai de 24 h imposé à un genre pour écrire en premier | Mécanique associée à Bumble, et discutable en soi | Le compte à rebours de 24 h porte sur **le fil**, identiquement pour les deux personnes, sans distinction de genre |
| File des « personnes qui vous ont aimé », débloquée par abonnement | Le levier commercial le plus copié — et celui qui produit la dynamique que Weave refuse | N'existe pas. Il n'y a pas de like, donc pas de file |
| Mise en avant payante dans le vivier | Vend de la visibilité, pas de la rencontre | N'existe pas. Aucun terme du score de composition n'est achetable |
| Marques verbales et signes du secteur (flamme, cœur, « super » quelque chose, « rembobiner », « passeport ») | Risque de marque et de présentation trompeuse | Lexique textile intégralement distinct : fil, trame, motif, métier, écho, prolonge, relais, escale |

## Ce qui appartient en propre à Weave

Ces éléments constituent l'identité du produit et sont ceux à protéger :

1. **Le plafond de trois fils simultanés**, identique pour tous les paliers, y
   compris payants.
2. **La proposition en cache uniquement**, sans archivage des profils vus.
3. **L'engagement par réponse écrite à un fragment**, à l'exclusion de tout
   geste binaire.
4. **La révélation progressive de la photo** indexée sur le nombre d'échanges
   aboutis (0 / 33 / 66 / 100 %).
5. **L'heure de tissage** : un rendez-vous quotidien choisi, seul moment de
   sollicitation.
6. **Le motif** : cinq mots comme première présentation d'une personne, avant
   son visage.
7. **Le lexique textile**, cohérent du code jusqu'à l'interface.

## Points à faire valider avant le lancement

- [ ] Recherche d'antériorité de marque sur « Weave » dans les classes 9, 42 et
      45, dans les territoires visés — le mot est courant en anglais, ce qui
      affaiblit son caractère distinctif : envisager un signe combiné
      (logotype des trois fils) plutôt que le mot seul.
- [ ] Revue de brevets (freedom to operate) sur la composition de fils, la
      révélation progressive d'image et le démarrage de Live Activity à
      distance.
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
Apache 2.0. L'adaptateur `@weave/prisma-bun-sqlite` a été écrit pour ce projet ;
il implémente une interface publique de Prisma et en reproduit la sémantique de
conversion documentée, ce qui est l'usage prévu de cette interface.
