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

  photos                       les photos envoyées et jamais examinées
  photo-ok <compte>            garde la photo et la marque examinée
  photo-retirer <compte>       retire la photo et efface ses octets

Clore un dossier ne sanctionne personne, et suspendre ne clôt aucun dossier :
ce sont deux décisions, et il faut les poser toutes les deux.";

pub async fn executer(
    db: &DatabaseConnection,
    cache: &ConnectionManager,
    arguments: &[String],
) -> anyhow::Result<()> {
    let commande = arguments.first().map(String::as_str).unwrap_or("");
    let un = arguments.get(1).map(String::as_str);
    let deux = arguments.get(2..).map(|reste| reste.join(" "));

    match commande {
        "signalements" => signalements(db).await?,
        "photos" => photos(db).await?,

        "clore" => {
            let dossier = exiger(un, "clore <dossier> [note]")?;
            dire(
                console::clore(db, dossier, deux.as_deref()).await?,
                "dossier clos",
                "ce dossier était déjà clos",
                "aucun dossier ne porte cet identifiant",
            );
        }

        "suspendre" => {
            let compte = exiger(un, "suspendre <compte> <motif>")?;
            let motif = deux.filter(|m| !m.is_empty()).ok_or_else(|| {
                // Un motif obligatoire n'est pas une formalité : c'est ce qui
                // rend la décision contestable, comme les mentions légales le
                // promettent.
                anyhow::anyhow!("Il faut un motif : suspendre <compte> <motif>")
            })?;
            dire(
                console::suspendre(db, compte, &motif).await?,
                "compte suspendu",
                "ce compte n'est pas dans un état où la suspension s'applique",
                "aucun compte ne porte cet identifiant",
            );
            oublier(cache, compte).await;
        }

        "retablir" => {
            let compte = exiger(un, "retablir <compte>")?;
            dire(
                console::retablir(db, compte).await?,
                "suspension levée",
                "ce compte n'était pas suspendu",
                "aucun compte ne porte cet identifiant",
            );
            oublier(cache, compte).await;
        }

        "photo-ok" => {
            let compte = exiger(un, "photo-ok <compte>")?;
            dire(
                console::photo_valider(db, compte).await?,
                "photo gardée et marquée examinée",
                "ce compte n'a pas de photo en attente d'examen",
                "aucune fiche pour cet identifiant",
            );
        }

        "photo-retirer" => {
            let compte = exiger(un, "photo-retirer <compte>")?;
            dire(
                console::photo_retirer(db, compte).await?,
                "photo retirée et octets effacés",
                "ce compte n'a pas de photo",
                "aucune fiche pour cet identifiant",
            );
            oublier(cache, compte).await;
        }

        _ => println!("{AIDE}"),
    }
    Ok(())
}

fn exiger<'a>(valeur: Option<&'a str>, usage: &str) -> anyhow::Result<&'a str> {
    valeur.ok_or_else(|| anyhow::anyhow!("Usage : weave-api console {usage}"))
}

fn dire(issue: Issue, fait: &str, deja: &str, introuvable: &str) {
    match issue {
        Issue::Fait => println!("  • {fait}"),
        Issue::Deja => println!("  • rien à faire : {deja}"),
        Issue::Introuvable => println!("  • {introuvable}"),
    }
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
