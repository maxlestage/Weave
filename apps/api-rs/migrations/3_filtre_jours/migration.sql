-- Le filtre par jour de la semaine.
--
-- Le catalogue vend « Critères précis : catégorie, jour, distance fine » au
-- palier Escapade. Le filtre par catégorie existait ; celui par jour n'existait
-- nulle part. On vendait un critère qui n'était pas écrit.
--
-- Les jours sont stockés en JSON dans une colonne texte, comme les autres
-- listes : le schéma est partagé avec SQLite, qui n'a pas de type tableau.
ALTER TABLE "preferences" ADD COLUMN "daysJson" TEXT NOT NULL DEFAULT '[]';
