//! L'interface en ligne de commande de la console de modération.
//!
//! Séparée de `console`, qui contient les gestes : celui-ci n'y ajoute que
//! l'analyse des arguments et la mise en forme. Les tests éprouvent les
//! gestes, pas l'affichage.

use crate::{cache, console};
use sea_orm::DatabaseConnection;

use console::Issue;
use redis::aio::ConnectionManager;

const AIDE: &str = "\
weave-api console — modération

  signalements                 les dossiers ouverts, du plus ancien au plus récent
  clore <dossier> [note]       clôt un dossier : il cesse de retenir la purge du compte visé
  suspendre <compte> <motif>   met un compte hors circulation ; ses sessions tombent
  retablir <compte>            lève une suspension

  verifications                les demandes de vérification, prioritaires d'abord
  verifier <compte> <motif>    pose le badge « vérifié » et clôt la demande
  refuser-verif <compte> <m.>  clôt la demande sans poser le badge
  deverifier <compte> <motif>  retire un badge déjà posé

  photos                       les photos envoyées et jamais examinées
  photo-ok <compte>            garde la photo et la marque examinée
  photo-retirer <compte>       retire la photo et efface ses octets

Clore un dossier ne sanctionne personne, et suspendre ne clôt aucun dossier :
ce sont deux décisions, et il faut les poser toutes les deux.

Les gestes qui demandent un motif l'inscrivent au journal d'audit : une décision
de modération sans trace ne se conteste pas.";

pub async fn executer(
    db: &DatabaseConnection,
    cache: &ConnectionManager,
    arguments: &[String],
) -> anyhow::Result<()> {
    let commande = arguments.first().map(String::as_str).unwrap_or("");
    let un = arguments.get(1).map(String::as_str);
    let deux = arguments.get(2..).map(|reste| reste.join(" "));

    // Vrai tant qu'aucun geste n'a visé un identifiant inexistant.
    let mut trouve = true;

    match commande {
        "signalements" => signalements(db).await?,
        "photos" => photos(db).await?,
        "verifications" => verifications(db).await?,

        "clore" => {
            let dossier = exiger(un, "clore <dossier> [note]")?;
            trouve &= dire(
                console::clore(db, dossier, deux.as_deref()).await?,
                "dossier clos",
                "ce dossier était déjà clos",
                "aucun dossier ne porte cet identifiant",
            );
        }

        "suspendre" => {
            let compte = exiger(un, "suspendre <compte> <motif>")?;
            let motif = exiger_motif(deux, "suspendre <compte> <motif>")?;
            trouve &= dire(
                console::suspendre(db, compte, &motif).await?,
                "compte suspendu",
                "ce compte n'est pas dans un état où la suspension s'applique",
                "aucun compte ne porte cet identifiant",
            );
            oublier(cache, compte).await;
        }

        "retablir" => {
            let compte = exiger(un, "retablir <compte>")?;
            trouve &= dire(
                console::retablir(db, compte).await?,
                "suspension levée",
                "ce compte n'était pas suspendu",
                "aucun compte ne porte cet identifiant",
            );
            oublier(cache, compte).await;
        }

        "verifier" => {
            let compte = exiger(un, "verifier <compte> <motif>")?;
            let motif = exiger_motif(deux, "verifier <compte> <motif>")?;
            trouve &= dire(
                console::verifier(db, compte, &motif).await?,
                "badge posé",
                "ce profil portait déjà le badge",
                "aucun compte ne porte cet identifiant",
            );
            oublier(cache, compte).await;
        }

        "refuser-verif" => {
            let compte = exiger(un, "refuser-verif <compte> <motif>")?;
            let motif = exiger_motif(deux, "refuser-verif <compte> <motif>")?;
            trouve &= dire(
                console::refuser_verification(db, compte, &motif).await?,
                "demande refusée, motif consigné",
                "ce compte n'a pas de demande en attente",
                "ce compte n'a pas de demande en attente",
            );
        }

        "deverifier" => {
            let compte = exiger(un, "deverifier <compte> <motif>")?;
            let motif = exiger_motif(deux, "deverifier <compte> <motif>")?;
            trouve &= dire(
                console::deverifier(db, compte, &motif).await?,
                "badge retiré",
                "ce profil ne portait pas le badge",
                "aucun compte ne porte cet identifiant",
            );
            oublier(cache, compte).await;
        }

        "photo-ok" => {
            let compte = exiger(un, "photo-ok <compte>")?;
            trouve &= dire(
                console::photo_valider(db, compte).await?,
                "photo gardée et marquée examinée",
                "ce compte n'a pas de photo en attente d'examen",
                "aucune fiche pour cet identifiant",
            );
        }

        "photo-retirer" => {
            let compte = exiger(un, "photo-retirer <compte>")?;
            trouve &= dire(
                console::photo_retirer(db, compte).await?,
                "photo retirée et octets effacés",
                "ce compte n'a pas de photo",
                "aucune fiche pour cet identifiant",
            );
            oublier(cache, compte).await;
        }

        // `console` tout court demande l'aide : ce n'est pas une erreur.
        "" => println!("{AIDE}"),

        // Une commande inconnue en est une. Elle affichait l'aide et sortait
        // avec succès : une faute de frappe dans un script passait pour un
        // geste accompli.
        autre => {
            eprintln!("{AIDE}");
            anyhow::bail!("commande inconnue : « {autre} »");
        }
    }

    if !trouve {
        anyhow::bail!("le geste n'a rien visé : aucun compte ou dossier ne porte cet identifiant");
    }
    Ok(())
}

fn exiger<'a>(valeur: Option<&'a str>, usage: &str) -> anyhow::Result<&'a str> {
    valeur.ok_or_else(|| anyhow::anyhow!("Usage : weave-api console {usage}"))
}

/// Un motif obligatoire n'est pas une formalité : c'est ce qui rend la décision
/// contestable, comme les mentions légales le promettent.
fn exiger_motif(valeur: Option<String>, usage: &str) -> anyhow::Result<String> {
    valeur
        .filter(|m| !m.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("Il faut un motif : weave-api console {usage}"))
}

/// Annonce l'issue d'un geste, et dit si la cible existait.
///
/// Le retour n'est pas décoratif : il devient le code de sortie. Un geste posé
/// sur un identifiant qui n'existe pas ne fait rien, et le dire seulement à
/// l'écran laissait la commande réussir — une console de modération se scripte
/// et s'appelle depuis un téléphone, et « rien ne porte cet identifiant » doit
/// se distinguer de « c'est fait » autrement que par la lecture.
///
/// `Deja` n'est pas un échec : reposer le même geste est sans effet et c'est
/// voulu. Seule une cible absente l'est.
#[must_use]
fn dire(issue: Issue, fait: &str, deja: &str, introuvable: &str) -> bool {
    match issue {
        Issue::Fait => println!("  • {fait}"),
        Issue::Deja => println!("  • rien à faire : {deja}"),
        Issue::Introuvable => println!("  • {introuvable}"),
    }
    !matches!(issue, Issue::Introuvable)
}

/// Oublie le résumé d'identité et le fil d'un compte.
///
/// L'échec est dit fort : sans cet oubli, une suspension ne prend effet qu'au
/// bout d'un quart d'heure, et quelqu'un qui vient de suspendre un compte doit
/// le savoir plutôt que de croire le geste posé.
async fn oublier(cache: &ConnectionManager, compte: &str) {
    for cle in [cache::cles::identite(compte), cache::cles::fil(compte)] {
        if let Err(erreur) = cache::oublier(cache, &cle).await {
            eprintln!(
                "  ⚠ cache non invalidé ({cle}) : {erreur}\n    \
                 Le changement ne sera visible qu'au bout d'un quart d'heure."
            );
        }
    }
}

async fn signalements(db: &DatabaseConnection) -> anyhow::Result<()> {
    let dossiers = console::dossiers_ouverts(db).await?;
    if dossiers.is_empty() {
        println!("  • aucun dossier ouvert");
        return Ok(());
    }
    println!("  {} dossier(s) ouvert(s)\n", dossiers.len());
    for dossier in dossiers {
        println!("  {}", dossier.id);
        println!("    motif   : {}", dossier.motif);
        println!(
            "    cible   : {} ({}, statut « {} »)",
            dossier.cible_nom, dossier.cible, dossier.cible_statut
        );
        println!("    auteur  : {}", dossier.auteur);
        println!("    déposé  : {}", dossier.cree_le.format("%d/%m/%Y %H:%M"));
        if !dossier.details.is_empty() {
            println!("    dit     : {}", dossier.details);
        }
        println!();
    }
    Ok(())
}

async fn verifications(db: &DatabaseConnection) -> anyhow::Result<()> {
    let file = console::verifications_en_attente(db).await?;
    if file.is_empty() {
        println!("  • aucune demande de vérification en attente");
        return Ok(());
    }
    println!("  {} demande(s) en attente\n", file.len());
    for demande in file {
        let priorite = if demande.palier == "grandtour" {
            " ★ prioritaire"
        } else {
            ""
        };
        println!("  {} ({}){priorite}", demande.nom, demande.compte);
        println!(
            "    palier {} — depuis le {}",
            demande.palier,
            demande.depuis.format("%d/%m/%Y")
        );
        if !demande.note.is_empty() {
            println!("    dit : {}", demande.note);
        }
        println!();
    }
    println!("  Aucune pièce d'identité ne transite par Weave : la vérification");
    println!("  se poursuit par courrier, à l'adresse d'assistance.");
    Ok(())
}

async fn photos(db: &DatabaseConnection) -> anyhow::Result<()> {
    let attente = console::photos_a_examiner(db).await?;
    if attente.is_empty() {
        println!("  • aucune photo en attente d'examen");
        return Ok(());
    }
    println!("  {} photo(s) en attente\n", attente.len());
    for photo in attente {
        println!("  {} ({})", photo.nom, photo.compte);
        println!(
            "    {} — {} ko — {}",
            photo.type_mime,
            photo.octets / 1024,
            photo.envoyee_le.format("%d/%m/%Y %H:%M")
        );
        println!();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::Cache;
    use crate::tests::base_de_test;
    use sea_orm::{ActiveModelTrait, Set};

    /// Ce module n'avait aucun test — son propre en-tête le disait : « Les
    /// tests éprouvent les gestes, pas l'affichage. » C'est vrai de la mise en
    /// forme ; ça ne l'est pas du CODE DE SORTIE, qui n'est pas de l'affichage
    /// mais ce qu'un script lit.
    ///
    /// Une console de modération s'appelle depuis `heroku run`, souvent depuis
    /// un téléphone, parfois depuis un script. Une commande qui ne fait rien et
    /// sort avec succès est le pire des retours : on croit avoir suspendu un
    /// compte.
    async fn cache_de_test() -> ConnectionManager {
        crate::cache::connecter(&Cache {
            url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            tls_insecure: false,
        })
        .await
        .expect("Redis local requis")
    }

    async fn compte(db: &DatabaseConnection, id: &str) {
        crate::entities::accounts::ActiveModel {
            id: Set(id.to_string()),
            email: Set(format!("{id}@exemple.test")),
            email_hash: Set(format!("h-{id}")),
            handle: Set(id.to_string()),
            display_name: Set(format!("{id} le prénom")),
            birth_date: Set(chrono::NaiveDate::from_ymd_opt(1995, 1, 1)
                .expect("date valide")
                .and_hms_opt(0, 0, 0)
                .expect("heure valide")),
            status: Set("active".to_string()),
            timezone: Set("Europe/Paris".to_string()),
            locale: Set("fr".to_string()),
            verified: Set(false),
            last_seen_at: Set(None),
            last_bilan_at: Set(None),
            deletion_requested_at: Set(None),
            created_at: Set(chrono::Utc::now().naive_utc()),
            updated_at: Set(chrono::Utc::now().naive_utc()),
        }
        .insert(db)
        .await
        .expect("compte inséré");
    }

    fn args(mots: &[&str]) -> Vec<String> {
        mots.iter().map(|m| (*m).to_string()).collect()
    }

    /// Une commande inconnue échoue.
    ///
    /// Elle affichait l'aide et sortait avec succès : une faute de frappe dans
    /// un script passait pour un geste accompli.
    #[tokio::test]
    async fn une_commande_inconnue_echoue() {
        let db = base_de_test().await;
        let cache = cache_de_test().await;

        assert!(
            executer(&db, &cache, &args(&["suspendr"])).await.is_err(),
            "« suspendr » ressemble à « suspendre » : c'est exactement la faute \
             qu'un code de sortie doit attraper"
        );
    }

    /// `console` sans argument demande l'aide : ce n'est pas une erreur.
    #[tokio::test]
    async fn l_aide_seule_reussit() {
        let db = base_de_test().await;
        let cache = cache_de_test().await;
        assert!(executer(&db, &cache, &[]).await.is_ok());
    }

    /// Un geste posé sur un identifiant qui n'existe pas échoue.
    ///
    /// Il n'écrit rien — c'est correct — mais il sortait avec succès. Un
    /// modérateur qui suspend le mauvais identifiant croyait avoir agi.
    #[tokio::test]
    async fn un_geste_sur_un_compte_inexistant_echoue() {
        let db = base_de_test().await;
        let cache = cache_de_test().await;

        for commande in [
            args(&["suspendre", "personne", "un", "motif"]),
            args(&["retablir", "personne"]),
            args(&["verifier", "personne", "un", "motif"]),
            args(&["photo-ok", "personne"]),
        ] {
            assert!(
                executer(&db, &cache, &commande).await.is_err(),
                "« {} » a réussi alors qu'elle n'a rien visé",
                commande.join(" ")
            );
        }
    }

    /// Et un geste sur un compte qui existe réussit.
    ///
    /// Sans ce test, faire échouer tout le reste passerait aussi.
    #[tokio::test]
    async fn un_geste_sur_un_compte_reel_reussit() {
        let db = base_de_test().await;
        let cache = cache_de_test().await;
        compte(&db, "c_console").await;

        executer(
            &db,
            &cache,
            &args(&["suspendre", "c_console", "un", "motif"]),
        )
        .await
        .expect("la suspension d'un compte existant doit réussir");

        // Reposer le même geste ne change rien, et n'est pas une erreur.
        executer(
            &db,
            &cache,
            &args(&["suspendre", "c_console", "un", "motif"]),
        )
        .await
        .expect("reposer un geste sans effet n'est pas un échec");

        // Et les listes, qui ne visent personne, réussissent toujours.
        for liste in ["signalements", "verifications", "photos"] {
            executer(&db, &cache, &args(&[liste]))
                .await
                .unwrap_or_else(|e| panic!("« {liste} » a échoué : {e}"));
        }
    }
}
