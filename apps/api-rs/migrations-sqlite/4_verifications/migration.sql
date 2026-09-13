-- La file des demandes de vérification.
--
-- `accounts.verified` s'écrit désormais depuis la console de modération, mais
-- RIEN NE PERMETTAIT DE DEMANDER À L'ÊTRE. Le badge ne pouvait donc se poser
-- que sur quelqu'un dont on aurait su, par un autre chemin, qu'il le voulait.
--
-- Et le palier Grand Tour vend « Vérification de profil accélérée » : une
-- priorité suppose une file, et il n'y en avait aucune.
--
-- Ce que cette table NE contient PAS : aucune pièce d'identité, aucun document,
-- aucune photo. La vérification se fait par échange avec l'assistance ; faire
-- transiter des papiers d'identité par l'application créerait une réserve de
-- données dont la perte serait irréparable, pour un service qui n'en a pas
-- besoin. La table ne porte qu'une demande, sa date, et la décision.
CREATE TABLE "verification_requests" (
    "id" TEXT NOT NULL,
    "accountId" TEXT NOT NULL,
    -- « en_attente », « acceptee », « refusee ».
    "state" TEXT NOT NULL DEFAULT 'en_attente',
    -- Ce que la personne dit d'elle-même, librement. Jamais obligatoire.
    "note" TEXT NOT NULL DEFAULT '',
    -- La raison de la décision, consignée : les mentions légales promettent
    -- qu'une décision de modération se conteste, et un refus sans motif ne se
    -- conteste pas.
    "decision" TEXT NOT NULL DEFAULT '',
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "handledAt" DATETIME,

    CONSTRAINT "verification_requests_pkey" PRIMARY KEY ("id"),
    CONSTRAINT "verification_requests_accountId_fkey"
        FOREIGN KEY ("accountId") REFERENCES "accounts"("id")
        ON DELETE CASCADE ON UPDATE CASCADE
);

-- Une seule demande en cours par compte : l'index ne porte que sur les lignes
-- en attente, si bien qu'un refus n'interdit pas de redemander plus tard.
CREATE UNIQUE INDEX "verification_requests_account_en_attente"
    ON "verification_requests"("accountId")
    WHERE "state" = 'en_attente';

CREATE INDEX "verification_requests_state_createdAt_idx"
    ON "verification_requests"("state", "createdAt");
