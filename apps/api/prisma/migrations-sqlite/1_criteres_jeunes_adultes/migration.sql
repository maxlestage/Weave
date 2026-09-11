-- RedefineTables
PRAGMA defer_foreign_keys=ON;
PRAGMA foreign_keys=OFF;
CREATE TABLE "new_preferences" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "accountId" TEXT NOT NULL,
    "minAge" INTEGER NOT NULL DEFAULT 18,
    "maxAge" INTEGER NOT NULL DEFAULT 32,
    "maxDistanceKm" INTEGER NOT NULL DEFAULT 50,
    "seekingJson" TEXT NOT NULL DEFAULT '[]',
    "intentsJson" TEXT NOT NULL DEFAULT '[]',
    "refinedJson" TEXT NOT NULL DEFAULT '{}',
    "escaleCity" TEXT,
    "escaleUntil" DATETIME,
    "updatedAt" DATETIME NOT NULL,
    CONSTRAINT "preferences_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "accounts" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);
INSERT INTO "new_preferences" ("accountId", "escaleCity", "escaleUntil", "id", "intentsJson", "maxAge", "maxDistanceKm", "minAge", "refinedJson", "seekingJson", "updatedAt") SELECT "accountId", "escaleCity", "escaleUntil", "id", "intentsJson", "maxAge", "maxDistanceKm", "minAge", "refinedJson", "seekingJson", "updatedAt" FROM "preferences";
DROP TABLE "preferences";
ALTER TABLE "new_preferences" RENAME TO "preferences";
CREATE UNIQUE INDEX "preferences_accountId_key" ON "preferences"("accountId");
PRAGMA foreign_keys=ON;
PRAGMA defer_foreign_keys=OFF;

