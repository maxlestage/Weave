-- Le rappel avant le rendez-vous.
--
-- Un plan se publie des jours à l'avance, et rien ne le remettait en mémoire.
-- L'absence n'est presque jamais un renoncement : c'est un oubli, et c'est ce
-- qui coûte le plus cher à un produit qui fait se rencontrer des gens — la
-- personne qui attend seule ne revient pas.
--
-- `remindedAt` plutôt qu'une fenêtre de temps étroite. Un dyno endormi une
-- heure manquerait une fenêtre, et le rappel ne partirait JAMAIS : la marque
-- laisse au contraire le rattrapage possible, et interdit seulement le doublon.
-- C'est aussi ce qui rend deux dynos inoffensifs l'un pour l'autre.
ALTER TABLE "plans" ADD COLUMN "remindedAt" TIMESTAMP(3);

-- Le rappel se coupe, et le réglage vit avec les autres critères.
--
-- Actif par défaut : l'autorisation de notifier a déjà été demandée à part, et
-- quelqu'un qui l'a accordée n'a pas à la redemander réglage par réglage. Le
-- couper reste à un geste.
ALTER TABLE "preferences" ADD COLUMN "remindersOn" BOOLEAN NOT NULL DEFAULT true;

-- Le balayage cherche les plans à venir non encore rappelés : sans cet index,
-- il lit toute la table à chaque passage.
CREATE INDEX "plans_remindedAt_startsAt_idx" ON "plans"("remindedAt", "startsAt");
