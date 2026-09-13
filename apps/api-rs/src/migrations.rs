//! Application des migrations.
//!
//! Le CLI Prisma disparaît avec l'API TypeScript, mais l'état qu'il a laissé
//! en base, lui, reste : la table `_prisma_migrations` dit ce qui est déjà
//! appliqué. On l'honore plutôt que de la remplacer — une base de production
//! qui rejouerait `0_init` échouerait sur des tables déjà créées, et un retour
//! en arrière vers Prisma resterait possible tant qu'on parle sa langue.

use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement};
use sha2::{Digest, Sha256};

/// Les migrations, incluses dans le binaire : un slug sans elles laisserait la
/// phase de publication échouer là où on ne peut plus rien corriger.
///
/// Deux jeux, parce que les deux dialectes divergent dès la première table —
/// `TIMESTAMP(3)` contre `DATETIME`, `TEXT[]` contre `JSONB`. Servir le SQL de
/// PostgreSQL à SQLite ne dégrade pas la migration : elle échoue à la première
/// parenthèse.
const MIGRATIONS_POSTGRES: &[(&str, &str)] = &[
    ("0_init", include_str!("../migrations/0_init/migration.sql")),
    (
        "1_media_objects",
        include_str!("../migrations/1_media_objects/migration.sql"),
    ),
];

const MIGRATIONS_SQLITE: &[(&str, &str)] = &[
    ("0_init", include_str!("../migrations-sqlite/0_init/migration.sql")),
    (
        "1_media_objects",
        include_str!("../migrations-sqlite/1_media_objects/migration.sql"),
    ),
];

/// Le jeu qui correspond au dialecte de la connexion.
fn jeu(backend: DatabaseBackend) -> &'static [(&'static str, &'static str)] {
    match backend {
        DatabaseBackend::Sqlite => MIGRATIONS_SQLITE,
        _ => MIGRATIONS_POSTGRES,
    }
}

/// Applique ce qui manque, et rien d'autre. Renvoie les noms appliqués.
pub async fn appliquer(db: &DatabaseConnection) -> Result<Vec<String>, DbErr> {
    creer_journal(db).await?;
    let deja = deja_appliquees(db).await?;

    let mut posees = Vec::new();
    for (nom, sql) in jeu(db.get_database_backend()) {
        if deja.iter().any(|d| d == nom) {
            continue;
        }
        for instruction in decouper(sql) {
            db.execute_unprepared(&instruction).await?;
        }
        inscrire(db, nom, sql).await?;
        posees.push((*nom).to_string());
    }
    Ok(posees)
}

/// La table du journal, au format de Prisma. `IF NOT EXISTS` la laisse
/// intacte là où elle existe déjà, avec tout son historique.
async fn creer_journal(db: &DatabaseConnection) -> Result<(), DbErr> {
    // `now()` et `TIMESTAMPTZ` sont du PostgreSQL ; SQLite veut
    // `CURRENT_TIMESTAMP` et `DATETIME`. Le reste des colonnes est identique,
    // pour que Prisma relise sans broncher la base qu'on aura migrée.
    let horodatage = match db.get_database_backend() {
        DatabaseBackend::Sqlite => ("DATETIME", "CURRENT_TIMESTAMP"),
        _ => ("TIMESTAMPTZ", "now()"),
    };
    db.execute_unprepared(&format!(
        r#"CREATE TABLE IF NOT EXISTS "_prisma_migrations" (
            "id" VARCHAR(36) NOT NULL PRIMARY KEY,
            "checksum" VARCHAR(64) NOT NULL,
            "finished_at" {0},
            "migration_name" VARCHAR(255) NOT NULL,
            "logs" TEXT,
            "rolled_back_at" {0},
            "started_at" {0} NOT NULL DEFAULT {1},
            "applied_steps_count" INTEGER NOT NULL DEFAULT 0
        )"#,
        horodatage.0, horodatage.1
    ))
    .await?;
    Ok(())
}

async fn deja_appliquees(db: &DatabaseConnection) -> Result<Vec<String>, DbErr> {
    let lignes = db
        .query_all_raw(Statement::from_string(
            db.get_database_backend(),
            // Une migration commencée mais non terminée n'est pas appliquée :
            // la rejouer est le bon comportement.
            r#"SELECT "migration_name" FROM "_prisma_migrations" WHERE "finished_at" IS NOT NULL"#,
        ))
        .await?;

    lignes
        .into_iter()
        .map(|ligne| ligne.try_get::<String>("", "migration_name"))
        .collect()
}

async fn inscrire(db: &DatabaseConnection, nom: &str, sql: &str) -> Result<(), DbErr> {
    // Le checksum est celui que Prisma calcule : l'empreinte SHA-256 du
    // fichier, en hexadécimal.
    let checksum = hex::encode(Sha256::digest(sql.as_bytes()));
    let id = cuid2::create_id();

    db.execute_unprepared(&format!(
        r#"INSERT INTO "_prisma_migrations"
           ("id","checksum","migration_name","started_at","finished_at","applied_steps_count")
           VALUES ('{id}','{checksum}','{nom}',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,1)"#
    ))
    .await?;
    Ok(())
}

/// Découpe un fichier de migration en instructions.
///
/// Chaque instruction est précédée d'un commentaire « -- CreateTable » :
/// écarter une instruction parce qu'elle commence par un commentaire les
/// écarterait toutes, en silence. Les commentaires se retirent ligne à ligne.
fn decouper(sql: &str) -> Vec<String> {
    // Les commentaires partent AVANT le découpage, et l'ordre est tout.
    //
    // Découper d'abord coupait un commentaire en deux dès qu'il contenait un
    // point-virgule — ce que la ponctuation française fait volontiers — et la
    // seconde moitié de la phrase devenait une instruction SQL. L'erreur
    // tombait au démarrage, sur « near "ici": syntax error », sans rien qui la
    // relie au commentaire qui l'a causée. En production, c'est la phase de
    // publication qui échoue, là où l'on ne peut plus rien corriger.
    let sans_commentaires: String = sql
        .lines()
        .filter(|ligne| !ligne.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");

    sans_commentaires
        .split(';')
        .map(|bloc| bloc.trim().to_string())
        .filter(|bloc| !bloc.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un point-virgule dans un commentaire ne coupe rien.
    ///
    /// La ponctuation française en met volontiers, et le découpage se faisait
    /// avant le retrait des commentaires : la moitié d'une phrase devenait une
    /// instruction. L'erreur tombait au démarrage, sans rien qui la relie au
    /// commentaire fautif.
    #[test]
    fn un_point_virgule_dans_un_commentaire_ne_coupe_rien() {
        let sql = "-- Il faut un bucket ; ici, il n'y en a pas.\nCREATE TABLE t (a TEXT);";
        let instructions = decouper(sql);
        assert_eq!(
            instructions,
            vec!["CREATE TABLE t (a TEXT)"],
            "le commentaire a été pris pour du SQL"
        );
    }

    #[test]
    fn le_decoupage_garde_toutes_les_instructions() {
        for (dialecte, jeu) in [("postgres", MIGRATIONS_POSTGRES), ("sqlite", MIGRATIONS_SQLITE)] {
            let instructions = decouper(jeu[0].1);
            // Le schéma compte dix-huit tables, plus ses index.
            assert!(
                instructions.len() >= 18,
                "{dialecte} : seulement {} instructions découpées",
                instructions.len()
            );
            assert!(
                instructions.iter().all(|i| !i.starts_with("--")),
                "{dialecte} : une instruction commence encore par un commentaire"
            );
            assert!(
                instructions
                    .iter()
                    .any(|i| i.contains("CREATE TABLE") && i.contains("accounts")),
                "{dialecte} : la table accounts est absente du découpage"
            );
        }
    }

    /// Le défaut qui a fait échouer la phase de publication en développement :
    /// le SQL de PostgreSQL servi à SQLite, jusqu'à `near "(": syntax error`.
    #[test]
    fn chaque_dialecte_recoit_son_propre_jeu() {
        assert_eq!(jeu(DatabaseBackend::Sqlite)[0].1, MIGRATIONS_SQLITE[0].1);
        assert_eq!(jeu(DatabaseBackend::Postgres)[0].1, MIGRATIONS_POSTGRES[0].1);
        assert_ne!(
            MIGRATIONS_SQLITE[0].1, MIGRATIONS_POSTGRES[0].1,
            "les deux jeux sont identiques : l'un des deux fichiers n'est pas le bon"
        );
    }
}
