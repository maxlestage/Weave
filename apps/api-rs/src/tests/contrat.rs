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
    let accords: [(&str, i64); 11] = [
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
