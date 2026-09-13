-- Les médias, stockés en base.
--
-- `photos.photoKey` existait depuis le début, et n'a jamais été écrit
-- autrement qu'à NULL : aucune route ne permettait d'envoyer une photo, et
-- `/media/{key}` rendait un objet JSON expliquant que le relais vers le
-- stockage objet « est branché au déploiement ». Il ne l'a jamais été.
--
-- Le stockage objet reste préférable — une base n'est pas faite pour les
-- octets d'images, et ils alourdissent chaque sauvegarde. Mais il demande un
-- fournisseur, un bucket et des identifiants ; ici, il n'y a rien à
-- provisionner et rien à configurer. L'URL signée sert d'interface : le jour
-- où un bucket existe, on change ce qui lit ces octets sans toucher au reste.
CREATE TABLE "media_objects" (
    "key" TEXT NOT NULL,
    "accountId" TEXT NOT NULL,
    "contentType" TEXT NOT NULL,
    "bytes" BYTEA NOT NULL,
    "byteSize" INTEGER NOT NULL,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "media_objects_pkey" PRIMARY KEY ("key")
);

-- CreateIndex
CREATE INDEX "media_objects_accountId_idx" ON "media_objects"("accountId");

-- AddForeignKey
-- La cascade fait le travail de la purge : effacer un compte emporte ses
-- médias, sans qu'aucune ligne de Rust n'ait à y penser.
ALTER TABLE "media_objects" ADD CONSTRAINT "media_objects_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "accounts"("id") ON DELETE CASCADE ON UPDATE CASCADE;
