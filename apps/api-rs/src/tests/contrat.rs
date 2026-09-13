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
    let accords: [(&str, i64); 16] = [
        // L'âge minimum d'abord : c'est la seule de ces valeurs qui décide
        // qui a le droit d'être là. Les CGU l'annoncent en lisant le contrat,
        // l'API le refuse en lisant sa propre constante — et rien ne les
        // rapprochait. Une divergence ferait annoncer un âge et en appliquer
        // un autre, sur la règle qui compte le plus pour ce produit.
        ("MIN_AGE", crate::routes::auth::AGE_MINIMUM as i64),
        ("MAX_OPEN_PLANS", crate::routes::plans::MAX_PLANS_OUVERTS as i64),
        ("RENFORT_GRANT", crate::droits::RENFORT_GRANT),
        ("DEFAULT_RADIUS_KM", crate::routes::fil::RAYON_DEFAUT_KM as i64),
        ("MAX_RADIUS_KM", crate::routes::me::RAYON_MAXIMUM_KM as i64),
        // `FEED_TTL_SECONDS` reste hors de cette liste : le contrat l'écrit
        // « 5 * 60 », que ce test refuse d'évaluer — et à raison. C'est par
        // ailleurs un réglage de cache, pas une promesse : une divergence y
        // coûterait quelques secondes de fraîcheur, rien qu'on ait annoncé.
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

/// Les objets de consentement, et la version des textes.
///
/// La version est ce qui décide qu'un consentement cesse de valoir. Écrite
/// deux fois — une pour la page, une pour l'API — elle finirait par désigner
/// deux textes différents : l'API tiendrait pour périmé un consentement que la
/// page affiche comme en vigueur, ou l'inverse. La seconde est la pire : le
/// traitement de données sensibles continuerait sur un consentement que le
/// texte affiché ne couvre plus.
#[test]
fn les_consentements_sont_les_memes_des_deux_cotes() {
    let source = contrat();

    assert_eq!(
        liste_du_contrat(&source, "CONSENT_KINDS"),
        trier(&crate::routes::consentements::OBJETS),
        "les objets de consentement divergent"
    );
    assert_eq!(
        chaine_du_contrat(&source, "POLICY_VERSION"),
        crate::routes::consentements::VERSION_POLITIQUE,
        "la version des textes diverge : un consentement vaudrait d'un côté et \
         pas de l'autre"
    );
}

/// La date affichée au bas des pages juridiques est celle de la version qui
/// gouverne les consentements.
///
/// Deux dates écrites séparément auraient fini par diverger : la page aurait
/// attesté d'un texte, la base d'un autre — et c'est cette date qui établit
/// quelle version a été acceptée.
#[test]
fn la_date_affichee_est_celle_de_la_version_en_vigueur() {
    let source = contrat();
    let version = chaine_du_contrat(&source, "POLICY_VERSION");
    let affichee = chaine_du_contrat(&source, "POLICY_UPDATED_LABEL");

    let mois = [
        "janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre",
        "octobre", "novembre", "décembre",
    ];
    let (annee, reste) = version.split_once('-').expect("version ISO : AAAA-MM-JJ");
    let (mois_iso, jour) = reste.split_once('-').expect("version ISO : AAAA-MM-JJ");
    let rang: usize = mois_iso.parse().expect("mois numérique");
    let attendue = format!(
        "{} {} {annee}",
        jour.trim_start_matches('0'),
        mois[rang - 1]
    );

    assert_eq!(
        affichee, attendue,
        "la date affichée ne correspond pas à la version en vigueur"
    );
}

/// Lit `export const NOM = "valeur"` du contrat.
fn chaine_du_contrat(source: &str, nom: &str) -> String {
    let prefixe = format!("export const {nom} = ");
    let ligne = source
        .lines()
        .find(|l| l.starts_with(&prefixe))
        .unwrap_or_else(|| panic!("« {nom} » a disparu du contrat partagé"));
    ligne[prefixe.len()..]
        .trim()
        .trim_end_matches(';')
        .trim()
        .trim_matches('"')
        .to_string()
}

/// Les crans de rayon, des deux côtés.
///
/// Le catalogue vend « distance fine » à partir de l'Escapade. Les crans sont
/// ce que les autres paliers obtiennent : une liste qui diverge ferait que le
/// site annonce un rayon que le fil n'applique pas.
#[test]
fn les_crans_de_rayon_sont_ceux_du_contrat_partage() {
    let source = contrat();
    let attendus: Vec<String> = crate::droits::CRANS_RAYON_KM
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        liste_du_contrat(&source, "RADIUS_STEPS_KM"),
        {
            let mut triee = attendus;
            triee.sort();
            triee
        },
        "les crans de rayon divergent entre le contrat et l'API"
    );
}

/// Les prix, des deux côtés.
///
/// L'API les réécrit — elle ne peut pas lire le TypeScript — et elle ne les
/// affiche pas seulement : `unit_purchases.price_cents` REÇOIT ce nombre à
/// chaque achat. Une divergence ferait donc que le site annonce un prix,
/// qu'Apple en encaisse un autre, et que le registre en consigne un troisième.
#[test]
fn les_prix_de_l_api_sont_ceux_du_contrat_partage() {
    let catalogue = catalogue();

    for (sku, attendu) in crate::routes::billing::unites_pour_test() {
        assert_eq!(
            prix_du_catalogue(&catalogue, "sku", sku),
            attendu,
            "le prix de « {sku} » diverge entre le contrat et l'API"
        );
    }
    for (palier, attendu) in crate::routes::billing::offres_pour_test() {
        assert_eq!(
            prix_du_catalogue(&catalogue, "tier", palier),
            attendu,
            "le prix du palier « {palier} » diverge entre le contrat et l'API"
        );
    }
}

/// L'achat à l'unité ne doit jamais revenir moins cher que l'abonnement.
///
/// Un catalogue où l'on s'en sort mieux au détail vend des unités à des gens
/// qui auraient pris un abonnement. Ce n'était pas théorique : quatre escales
/// coûtaient 14,99 + 2 × 3,99 = 22,97 € par-dessus l'Expédition, contre 24,99 €
/// pour le Grand Tour qui les comprend. On s'abonnait moins pour en avoir plus.
///
/// Le test ne cherche pas à rendre l'unité perdante dans tous les cas : en
/// acheter UNE pour un besoin ponctuel reste moins cher que de s'abonner, et
/// c'est le propre de l'achat à l'unité.
#[test]
fn aucun_achat_a_l_unite_ne_revient_moins_cher_que_l_abonnement() {
    let catalogue = catalogue();
    let unite = |sku: &str| prix_du_catalogue(&catalogue, "sku", sku);
    let palier = |tier: &str| prix_du_catalogue(&catalogue, "tier", tier);

    let somme: i64 = ["renfort", "horizon", "tablee", "escale", "bilan"]
        .iter()
        .map(|sku| unite(sku))
        .sum();
    assert!(
        somme > palier("expedition"),
        "une fois chaque unité ({somme} c) coûte moins qu'un mois d'Expédition ({} c), \
         qui les accorde toutes et les redonne le mois suivant",
        palier("expedition")
    );

    // La Virée accorde sans compter ce que ces trois-là vendent à l'unité.
    for sku in ["renfort", "horizon", "tablee"] {
        assert!(
            2 * unite(sku) > palier("viree"),
            "deux « {sku} » ({} c) coûtent moins que la Virée ({} c)",
            2 * unite(sku),
            palier("viree")
        );
    }

    // L'Escapade comprend une escale ; le Grand Tour en comprend quatre, soit
    // deux de plus que l'Expédition.
    assert!(
        palier("viree") + unite("escale") > palier("escapade"),
        "la Virée plus une escale revient moins cher que l'Escapade, qui en comprend une"
    );
    assert!(
        palier("expedition") + 2 * unite("escale") > palier("grandtour"),
        "l'Expédition plus deux escales revient moins cher que le Grand Tour, \
         qui comprend les quatre"
    );
}

/// Un crédit « Horizon » ne donne pas plus que l'abonnement le plus cher.
///
/// Il ne donnait AUCUNE borne : au-delà de l'horizon du palier, il passait. Un
/// compte gratuit muni d'un crédit publiait un plan pour 2050 — plus loin que
/// le Grand Tour, et plus loin que les soixante jours que le catalogue annonce
/// en le vendant.
#[test]
fn le_credit_horizon_ne_depasse_pas_ce_qu_il_annonce() {
    let catalogue = catalogue();
    let annonce = catalogue
        .split("horizon: {")
        .nth(1)
        .and_then(|bloc| bloc.split("description:").nth(1))
        .and_then(|reste| reste.split('"').nth(1))
        .expect("la description du crédit « Horizon »");
    assert!(
        annonce.contains("soixante jours"),
        "le catalogue annonce autre chose que soixante jours : « {annonce} »"
    );
    assert_eq!(
        crate::droits::HORIZON_CREDIT_JOURS, 60,
        "la borne du crédit ne correspond plus à ce qui est vendu"
    );
    assert!(
        crate::droits::HORIZON_CREDIT_JOURS <= crate::droits::droits_pour("grandtour").jours_a_l_avance,
        "un crédit à l'unité donnerait plus que l'abonnement le plus cher"
    );
}

/// La date affichée sur la politique de confidentialité est celle que portent
/// les consentements.
///
/// Chaque document juridique porte désormais SA date — corriger un tarif touche
/// les conditions de vente, pas la politique de confidentialité. Mais celle de
/// la politique n'est pas libre : c'est la version qu'enregistrent les
/// consentements, et une page qui afficherait une autre date prétendrait qu'on
/// a accepté un texte qui n'est pas celui-là.
///
/// Le test vérifie que l'entrée reprend la constante partagée plutôt qu'une
/// date recopiée — une copie se désynchroniserait sans que rien ne le dise.
#[test]
fn la_politique_affiche_la_date_de_la_version_consentie() {
    let chemin = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../web/src/pages/documents.ts");
    let Ok(source) = std::fs::read_to_string(&chemin) else {
        eprintln!("site absent en {} — accord non vérifié", chemin.display());
        return;
    };

    let debut = source
        .find(r#"slug: "confidentialite""#)
        .expect("la politique a disparu de la liste des documents");
    let bloc = &source[debut..debut + source[debut..].find("},").expect("bloc fermé")];
    assert!(
        bloc.contains("miseAJour: POLICY_UPDATED_LABEL"),
        "la politique de confidentialité porte une date recopiée plutôt que la \
         version consentie : « {bloc} »"
    );
}

fn catalogue() -> String {
    let chemin = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../packages/contracts/src/catalog.ts");
    std::fs::read_to_string(&chemin)
        .unwrap_or_else(|e| panic!("catalogue illisible en {} : {e}", chemin.display()))
}

/// Lit le prix du bloc dont `champ: "valeur"` ouvre la déclaration.
///
/// `sku` pour une unité — elle porte `priceCents` — et `tier` pour un palier,
/// qui porte `monthlyPriceCents`.
fn prix_du_catalogue(source: &str, champ: &str, valeur: &str) -> i64 {
    let marque = format!("{champ}: \"{valeur}\",");
    let debut = source
        .find(&marque)
        .unwrap_or_else(|| panic!("« {valeur} » a disparu du catalogue"));
    let cle = if champ == "sku" { "priceCents: " } else { "monthlyPriceCents: " };
    let apres = source[debut..]
        .find(cle)
        .unwrap_or_else(|| panic!("« {valeur} » n'a pas de prix"))
        + debut
        + cle.len();
    source[apres..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap_or_else(|_| panic!("le prix de « {valeur} » n'est pas un nombre"))
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
        // Un objet de consentement que l'application nommerait autrement se
        // ferait refuser par la route, et l'accord ne pourrait pas être donné
        // — le traitement resterait bloqué sans que rien ne dise pourquoi.
        ("Consentement.swift", "ConsentKind", liste_du_contrat(&contrat, "CONSENT_KINDS")),
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
