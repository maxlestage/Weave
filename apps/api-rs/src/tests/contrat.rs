//! Le contrat partagé, tenu des deux côtés.
//!
//! `packages/contracts/src/invariants.ts` est la source unique : le site en
//! lit les nombres pour écrire les pages juridiques, et l'application iOS les
//! affiche. L'API, elle, les réécrit en Rust — il n'existe pas de moyen de
//! lire un module TypeScript à la compilation.
//!
//! Deux écritures d'un même nombre finissent par diverger, et ici la
//! divergence est silencieuse et publique : la page « Supprimer votre compte »
//! promettrait trente jours pendant que la purge en attendrait soixante.
//! Personne ne s'en apercevrait, sinon la personne à qui on l'a promis.
//!
//! Ce test relit donc le fichier TypeScript et compare. Il ne vérifie pas que
//! les valeurs sont bonnes — seulement que les deux côtés disent la même
//! chose, ce qui est précisément ce qu'on ne peut pas voir en relisant un seul
//! des deux.

/// Lit `export const NOM = <nombre>` dans le contrat.
///
/// Une expression — `5 * 60` — n'est pas évaluée : le test la signale plutôt
/// que de deviner. Un contrat dont une valeur devient calculée mérite qu'on
/// revienne ici décider quoi en faire.
fn valeur_du_contrat(source: &str, nom: &str) -> i64 {
    let prefixe = format!("export const {nom} = ");
    let ligne = source
        .lines()
        .find(|l| l.starts_with(&prefixe))
        .unwrap_or_else(|| panic!("« {nom} » a disparu du contrat partagé"));

    let reste = ligne[prefixe.len()..].trim();
    let brut: String = reste.chars().take_while(|c| c.is_ascii_digit()).collect();
    assert!(
        !brut.is_empty(),
        "« {nom} » ne commence pas par un nombre : « {reste} »"
    );

    // Ce qui suit doit être la fin de la déclaration, pas un calcul.
    let suite = reste[brut.len()..].trim();
    assert!(
        suite.is_empty() || suite.starts_with(';') || suite.starts_with("as const"),
        "« {nom} » vaut une expression — « {reste} » — que ce test ne sait pas évaluer"
    );

    brut.parse().expect("un entier")
}

fn contrat() -> String {
    let chemin = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../packages/contracts/src/invariants.ts");
    std::fs::read_to_string(&chemin)
        .unwrap_or_else(|e| panic!("contrat illisible en {} : {e}", chemin.display()))
}

#[test]
fn les_nombres_de_l_api_sont_ceux_du_contrat_partage() {
    let source = contrat();

    // Chaque ligne : le nom dans le contrat, la constante Rust qui doit lui
    // répondre. La liste ne couvre que ce que l'API réécrit de son côté.
    let accords: [(&str, i64); 12] = [
        ("MAX_OPEN_PLANS", crate::routes::plans::MAX_PLANS_OUVERTS as i64),
        ("ACCOUNT_PURGE_DAYS", crate::purge::PURGE_COMPTE_JOURS),
        (
            "MESSAGE_RETENTION_DAYS",
            crate::routes::conversations::RETENTION_MESSAGES_JOURS,
        ),
        ("PLAN_TITLE_MIN_CHARS", crate::routes::plans::TITRE_MIN as i64),
        ("PLAN_TITLE_MAX_CHARS", crate::routes::plans::TITRE_MAX as i64),
        ("PLAN_NOTE_MAX_CHARS", crate::routes::plans::NOTE_MAX as i64),
        ("PLAN_MIN_LEAD_MINUTES", crate::routes::plans::DELAI_MINIMUM_MINUTES),
        ("PLAN_CAPACITY_SOLO", crate::routes::plans::CAPACITE_SOLO as i64),
        (
            "PLAN_CAPACITY_GROUP_MAX",
            crate::routes::plans::CAPACITE_GROUPE_MAX as i64,
        ),
        ("REQUEST_MIN_CHARS", crate::routes::requests::MESSAGE_MIN as i64),
        ("REQUEST_MAX_CHARS", crate::routes::requests::MESSAGE_MAX as i64),
        ("BIO_MAX_CHARS", crate::routes::me::BIO_MAX_CARACTERES as i64),
    ];

    let mut ecarts = Vec::new();
    for (nom, cote_api) in accords {
        let cote_contrat = valeur_du_contrat(&source, nom);
        if cote_contrat != cote_api {
            ecarts.push(format!(
                "  {nom} : le contrat dit {cote_contrat}, l'API applique {cote_api}"
            ));
        }
    }

    assert!(
        ecarts.is_empty(),
        "l'API et le contrat partagé ne disent plus la même chose :\n{}\n\
         Le site et l'application iOS affichent les valeurs du contrat ; \
         l'API applique les siennes.",
        ecarts.join("\n")
    );
}

/// Les genres, eux, décident de qui voit qui.
///
/// Le fil compare le genre de l'auteur d'un plan à ceux que le lecteur
/// cherche, caractère par caractère. Un vocabulaire qui diverge d'un côté ne
/// rend pas une erreur : il rend un fil vide, sans rien dire.
#[test]
fn les_genres_sont_les_memes_des_deux_cotes() {
    assert_eq!(
        liste_du_contrat(&contrat(), "GENDERS"),
        trier(&crate::routes::me::GENRES),
        "le vocabulaire des genres diverge : la correspondance par genre ne rendrait plus rien"
    );
}

/// Les catégories de plan aussi : une catégorie acceptée par l'API mais
/// inconnue du contrat n'est affichable nulle part, et l'inverse fait rejeter
/// un plan que l'application proposait.
#[test]
fn les_categories_de_plan_sont_les_memes_des_deux_cotes() {
    let source = contrat();
    let debut = source
        .find("export const PLAN_CATEGORIES")
        .expect("les catégories ont disparu du contrat");
    let ouvrante = source[debut..].find('[').expect("une liste") + debut;
    let fermante = source[ouvrante..].find(']').expect("une liste fermée") + ouvrante;

    let mut du_contrat: Vec<String> = source[ouvrante + 1..fermante]
        .split(',')
        .map(|m| m.trim().trim_matches(['"', '\'']).to_string())
        .filter(|m| !m.is_empty())
        .collect();
    du_contrat.sort();

    let mut de_l_api: Vec<String> = crate::routes::plans::CATEGORIES
        .iter()
        .map(|c| (*c).to_string())
        .collect();
    de_l_api.sort();

    assert_eq!(
        du_contrat, de_l_api,
        "les catégories divergent : le contrat en déclare {} et l'API en accepte {}",
        du_contrat.len(),
        de_l_api.len()
    );
}

/// Les états de plan, eux aussi, doivent être les mêmes des deux côtés.
///
/// L'API écrivait « suspendu » — l'état d'un plan dont l'auteur est en pause —
/// sans que le contrat ni le modèle iOS le déclarent. Or `PlanState` y est une
/// énumération `Codable` non facultative : le décodage de « Mes plans »
/// échouait d'un bloc, et l'écran entier tombait, pour un état qu'aucun des
/// deux autres côtés ne connaissait.
#[test]
fn les_etats_de_plan_ecrits_par_l_api_sont_declares_au_contrat() {
    let chemin = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../packages/contracts/src/domain.ts");
    let source = std::fs::read_to_string(&chemin)
        .unwrap_or_else(|e| panic!("contrat illisible en {} : {e}", chemin.display()));

    let debut = source
        .find("export type PlanState =")
        .expect("les états de plan ont disparu du contrat");
    let fin = source[debut..].find(';').expect("déclaration fermée") + debut;
    let declaration = &source[debut..fin];

    // Ce que l'API écrit réellement en base, relu dans ses propres sources
    // plutôt que recopié ici : une liste tenue à la main dériverait comme le
    // reste, et c'est précisément ce que ce fichier existe pour empêcher.
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut ecrits = std::collections::BTreeSet::new();
    for etat in ETATS_CONNUS {
        if sources_contiennent(&racine, &format!("\"{etat}\"")) {
            ecrits.insert(*etat);
        }
    }

    let absents: Vec<&str> = ecrits
        .iter()
        .copied()
        .filter(|etat| !declaration.contains(&format!("\"{etat}\"")))
        .collect();

    assert!(
        absents.is_empty(),
        "l'API écrit des états que le contrat ne déclare pas : {absents:?}\n\
         Le modèle iOS décode `PlanState` sans repli : un état inconnu fait \
         échouer l'écran entier."
    );
}

/// Les états qu'un plan peut prendre, tous côtés confondus.
///
/// Écrite ici parce qu'il faut bien un point de départ pour chercher. Le test
/// ne vérifie pas cette liste : il vérifie que ceux qu'il retrouve dans les
/// sources de l'API figurent au contrat.
const ETATS_CONNUS: &[&str] = &["ouvert", "complet", "passe", "annule", "suspendu"];

fn sources_contiennent(racine: &std::path::Path, aiguille: &str) -> bool {
    let Ok(entrees) = std::fs::read_dir(racine) else {
        return false;
    };
    for entree in entrees.flatten() {
        let chemin = entree.path();
        if chemin.is_dir() {
            // Les tests écrivent des états pour les éprouver : ils ne disent
            // pas ce que l'API rend à ses clients.
            if chemin.file_name().is_some_and(|n| n == "tests") {
                continue;
            }
            if sources_contiennent(&chemin, aiguille) {
                return true;
            }
        } else if chemin.extension().is_some_and(|e| e == "rs")
            && std::fs::read_to_string(&chemin)
                .is_ok_and(|contenu| contenu.contains(aiguille))
        {
            return true;
        }
    }
    false
}

/// L'application iOS réécrit elle aussi certains nombres du contrat.
///
/// `accountPurgeDays` est affiché à qui demande la suppression de son compte :
/// « tout est effacé sous N jours ». Annoncer autre chose que ce que la purge
/// applique serait mentir sur un délai que la politique de confidentialité
/// engage — et c'est un troisième endroit où le même nombre est écrit, sans
/// que rien ne relie les trois.
#[test]
fn les_nombres_de_l_application_ios_sont_ceux_du_contrat_partage() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ios/WeaveKit/Sources/WeaveKit/Models");
    if !racine.is_dir() {
        // Le dépôt iOS peut être absent d'une copie partielle. Le dire plutôt
        // que d'échouer : ce test garde un accord, il ne réclame pas un
        // fichier.
        eprintln!("modèles iOS absents en {} — accord non vérifié", racine.display());
        return;
    }
    // Les constantes sont réparties entre les fichiers de modèle : les
    // chercher toutes plutôt que d'en nommer un seul, sinon déplacer une
    // constante suffirait à désarmer le test sans que personne ne le voie.
    let mut source = String::new();
    for entree in std::fs::read_dir(&racine).expect("modèles lisibles").flatten() {
        if entree.path().extension().is_some_and(|e| e == "swift") {
            source.push_str(&std::fs::read_to_string(entree.path()).expect("modèle lisible"));
            source.push('\n');
        }
    }

    let contrat = contrat();
    for (cote_swift, cote_contrat) in [
        ("accountPurgeDays", "ACCOUNT_PURGE_DAYS"),
        ("bioMaxChars", "BIO_MAX_CHARS"),
    ] {
        let prefixe = format!("public let {cote_swift} = ");
        let ligne = source
            .lines()
            .find(|l| l.trim_start().starts_with(&prefixe))
            .unwrap_or_else(|| panic!("« {cote_swift} » a disparu du modèle iOS"));
        let brut: String = ligne.trim_start()[prefixe.len()..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        let valeur: i64 = brut
            .parse()
            .unwrap_or_else(|_| panic!("« {cote_swift} » ne vaut pas un entier : « {ligne} »"));

        assert_eq!(
            valeur,
            valeur_du_contrat(&contrat, cote_contrat),
            "l'application iOS affiche {valeur} pour {cote_swift}, le contrat dit autre chose"
        );
    }
}

/// Lit une liste `export const NOM = ["a", "b"]` du contrat, triée.
fn liste_du_contrat(source: &str, nom: &str) -> Vec<String> {
    let debut = source
        .find(&format!("export const {nom}"))
        .unwrap_or_else(|| panic!("« {nom} » a disparu du contrat partagé"));
    let ouvrante = source[debut..].find('[').expect("une liste") + debut;
    let fermante = source[ouvrante..].find(']').expect("une liste fermée") + ouvrante;

    let mut valeurs: Vec<String> = source[ouvrante + 1..fermante]
        .split(',')
        .map(|m| m.trim().trim_matches(['"', '\'']).to_string())
        .filter(|m| !m.is_empty())
        .collect();
    valeurs.sort();
    valeurs
}

fn trier(valeurs: &[&str]) -> Vec<String> {
    let mut triees: Vec<String> = valeurs.iter().map(|v| (*v).to_string()).collect();
    triees.sort();
    triees
}

/// Les vocabulaires écrits en Swift doivent être ceux du contrat.
///
/// C'est le côté qu'aucun test ne tenait, et cela s'est vu : « suspendu »
/// manquait à `PlanState` sans que rien ne le signale, et l'écran « Mes plans »
/// serait tombé entier au premier plan d'un compte en pause. Une énumération
/// `Codable` non facultative ne pardonne pas une valeur qu'elle ignore.
#[test]
fn les_vocabulaires_de_l_application_ios_sont_ceux_du_contrat() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ios/WeaveKit/Sources/WeaveKit/Models");
    if !racine.is_dir() {
        eprintln!("modèles iOS absents en {} — accord non vérifié", racine.display());
        return;
    }

    let contrat = contrat();
    let domaine = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../packages/contracts/src/domain.ts"),
    )
    .expect("contrat de domaine lisible");

    // Chaque ligne : le fichier Swift, l'énumération, et la liste de référence.
    for (fichier, enumeration, reference) in [
        ("Gender.swift", "Gender", liste_du_contrat(&contrat, "GENDERS")),
        ("Plan.swift", "PlanState", liste_du_type(&domaine, "PlanState")),
        // Les motifs de signalement ne passent pas par le contrat partagé :
        // le site n'en a pas l'usage. L'accord se tient donc directement
        // entre l'application et la route qui les refuse.
        ("Moderation.swift", "ReportReason", trier(&crate::routes::moderation::MOTIFS)),
    ] {
        let source = std::fs::read_to_string(racine.join(fichier))
            .unwrap_or_else(|e| panic!("{fichier} illisible : {e}"));
        let cotes_swift = cas_de_l_enumeration(&source, enumeration);
        assert_eq!(
            cotes_swift, reference,
            "« {enumeration} » ({fichier}) ne dit pas la même chose que le contrat"
        );
    }
}

/// Lit un `export type NOM = "a" | "b";` du contrat, trié.
fn liste_du_type(source: &str, nom: &str) -> Vec<String> {
    let debut = source
        .find(&format!("export type {nom} ="))
        .unwrap_or_else(|| panic!("« {nom} » a disparu du contrat"));
    let fin = source[debut..].find(';').expect("déclaration fermée") + debut;

    let mut valeurs: Vec<String> = source[debut..fin]
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect();
    valeurs.sort();
    valeurs
}

/// Les valeurs brutes des cas d'une énumération Swift à valeur de chaîne.
///
/// Un cas sans valeur explicite — `case femme` — vaut son propre nom ; un cas
/// avec — `case nonBinaire = "non_binaire"` — vaut ce qui est écrit. C'est
/// exactement la règle de Swift, et c'est ce que le serveur recevra.
fn cas_de_l_enumeration(source: &str, nom: &str) -> Vec<String> {
    let debut = source
        .find(&format!("enum {nom}"))
        .unwrap_or_else(|| panic!("« {nom} » a disparu des modèles iOS"));
    let corps_debut = source[debut..].find('{').expect("un corps") + debut + 1;

    let mut profondeur = 1usize;
    let mut fin = corps_debut;
    for (decalage, caractere) in source[corps_debut..].char_indices() {
        match caractere {
            '{' => profondeur += 1,
            '}' => {
                profondeur -= 1;
                if profondeur == 0 {
                    fin = corps_debut + decalage;
                    break;
                }
            }
            _ => {}
        }
    }

    let mut valeurs = Vec::new();
    for ligne in source[corps_debut..fin].lines() {
        let ligne = ligne.trim();
        let Some(reste) = ligne.strip_prefix("case ") else {
            continue;
        };
        // « case .autre: "Autre" » est un motif de `switch`, pas la
        // déclaration d'un cas : le point qui l'ouvre les distingue. Sans cette
        // écarte, les libellés d'affichage entraient dans le vocabulaire.
        if reste.trim_start().starts_with('.') {
            continue;
        }

        // « case ouvert, complet » — la forme condensée compte autant.
        for cas in reste.split(',') {
            let cas = cas.trim();
            let valeur = match cas.split_once('=') {
                Some((_, brute)) => brute.trim().trim_matches('"').to_string(),
                None => cas.to_string(),
            };
            if !valeur.is_empty() {
                valeurs.push(valeur);
            }
        }
    }
    valeurs.sort();
    valeurs
}

/// Le palier à partir duquel l'application propose le filtre par jour est
/// celui que le serveur accepte.
///
/// `PlanTier.filtreParJour` existe côté iOS pour ne pas MONTRER un réglage qui
/// sera refusé — proposer puis refuser est une façon de vendre, pas de régler.
/// C'est donc une copie de la règle du serveur, et une copie dérive : ici,
/// elle dériverait en silence, l'application montrant un réglage refusé ou
/// cachant un réglage permis.
#[test]
fn le_filtre_par_jour_est_propose_aux_memes_paliers_des_deux_cotes() {
    let chemin = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ios/WeaveKit/Sources/WeaveKit/Models/Account.swift");
    let Ok(source) = std::fs::read_to_string(&chemin) else {
        eprintln!("modèle iOS absent en {} — accord non vérifié", chemin.display());
        return;
    };

    let debut = source
        .find("public var filtreParJour: Bool {")
        .expect("« filtreParJour » a disparu du modèle iOS");
    let fin = source[debut..].find("\n    }").expect("corps fermé") + debut;
    let corps = &source[debut..fin];

    for palier in ["depart", "viree", "escapade", "expedition", "grandtour"] {
        // Le cas Swift s'écrit « .depart, .viree: false ».
        let cote_ios = corps
            .lines()
            .find(|ligne| ligne.contains(&format!(".{palier}")))
            .map(|ligne| ligne.contains("true"))
            .unwrap_or_else(|| panic!("« {palier} » absent de `filtreParJour`"));

        let cote_serveur = crate::droits::filtre_autorise(palier, crate::droits::Critere::Jour);
        assert_eq!(
            cote_ios, cote_serveur,
            "« {palier} » : l'application propose {cote_ios}, le serveur accepte {cote_serveur}"
        );
    }
}
