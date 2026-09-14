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
    let accords: [(&str, i64); 17] = [
        // L'âge minimum d'abord : c'est la seule de ces valeurs qui décide
        // qui a le droit d'être là. Les CGU l'annoncent en lisant le contrat,
        // l'API le refuse en lisant sa propre constante — et rien ne les
        // rapprochait. Une divergence ferait annoncer un âge et en appliquer
        // un autre, sur la règle qui compte le plus pour ce produit.
        ("MIN_AGE", crate::routes::auth::AGE_MINIMUM as i64),
        (
            "MAX_OPEN_PLANS",
            crate::routes::plans::MAX_PLANS_OUVERTS as i64,
        ),
        ("RENFORT_GRANT", crate::droits::RENFORT_GRANT),
        (
            "DEFAULT_RADIUS_KM",
            crate::routes::fil::RAYON_DEFAUT_KM as i64,
        ),
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
        (
            "PLAN_TITLE_MIN_CHARS",
            crate::routes::plans::TITRE_MIN as i64,
        ),
        (
            "PLAN_TITLE_MAX_CHARS",
            crate::routes::plans::TITRE_MAX as i64,
        ),
        ("PLAN_NOTE_MAX_CHARS", crate::routes::plans::NOTE_MAX as i64),
        (
            "PLAN_MIN_LEAD_MINUTES",
            crate::routes::plans::DELAI_MINIMUM_MINUTES,
        ),
        (
            "PLAN_CAPACITY_SOLO",
            crate::routes::plans::CAPACITE_SOLO as i64,
        ),
        (
            "PLAN_CAPACITY_GROUP_MAX",
            crate::routes::plans::CAPACITE_GROUPE_MAX as i64,
        ),
        (
            "REQUEST_MIN_CHARS",
            crate::routes::requests::MESSAGE_MIN as i64,
        ),
        (
            "REQUEST_MAX_CHARS",
            crate::routes::requests::MESSAGE_MAX as i64,
        ),
        (
            "BIO_MAX_CHARS",
            crate::routes::me::BIO_MAX_CARACTERES as i64,
        ),
        (
            "CONVERSATION_MAX_CHARS",
            crate::routes::conversations::MESSAGE_MAX as i64,
        ),
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
        du_contrat,
        de_l_api,
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
fn les_etats_ecrits_par_l_api_sont_declares_au_contrat() {
    let chemin = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../packages/contracts/src/domain.ts");
    let source = std::fs::read_to_string(&chemin)
        .unwrap_or_else(|e| panic!("contrat illisible en {} : {e}", chemin.display()));

    // Ce que l'API écrit réellement en base, relu dans ses propres sources
    // plutôt que recopié ici : une liste tenue à la main dériverait comme le
    // reste, et c'est précisément ce que ce fichier existe pour empêcher.
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    for (type_du_contrat, connus, ecran) in [
        ("PlanState", ETATS_DE_PLAN_CONNUS, "« Mes plans »"),
        ("RequestState", ETATS_DE_DEMANDE_CONNUS, "« Mes demandes »"),
    ] {
        let debut = source
            .find(&format!("export type {type_du_contrat} ="))
            .unwrap_or_else(|| panic!("« {type_du_contrat} » a disparu du contrat"));
        let fin = source[debut..].find(';').expect("déclaration fermée") + debut;
        let declaration = &source[debut..fin];

        let mut ecrits = std::collections::BTreeSet::new();
        for etat in connus {
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
            "l'API écrit des états que le contrat ne déclare pas en \
             « {type_du_contrat} » : {absents:?}\n\
             Le modèle iOS le décode sans repli : un état inconnu fait échouer \
             {ecran} en entier."
        );
    }
}

/// Les états qu'un plan peut prendre, tous côtés confondus.
///
/// Écrite ici parce qu'il faut bien un point de départ pour chercher. Le test
/// ne vérifie pas cette liste : il vérifie que ceux qu'il retrouve dans les
/// sources de l'API figurent au contrat.
const ETATS_DE_PLAN_CONNUS: &[&str] = &["ouvert", "complet", "passe", "annule", "suspendu"];

/// Les états qu'une demande peut prendre, au même titre.
///
/// Une demande se lit dans un tableau — « Mes demandes » les décode toutes ou
/// aucune. Le risque est donc celui du fil, pas celui d'une ligne fautive.
const ETATS_DE_DEMANDE_CONNUS: &[&str] = &[
    "envoyee", "acceptee", "refusee", "expiree", "retiree", "annulee",
];

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
            && std::fs::read_to_string(&chemin).is_ok_and(|contenu| contenu.contains(aiguille))
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
        eprintln!(
            "modèles iOS absents en {} — accord non vérifié",
            racine.display()
        );
        return;
    }
    // Les constantes sont réparties entre les fichiers de modèle : les
    // chercher toutes plutôt que d'en nommer un seul, sinon déplacer une
    // constante suffirait à désarmer le test sans que personne ne le voie.
    let mut source = String::new();
    for entree in std::fs::read_dir(&racine)
        .expect("modèles lisibles")
        .flatten()
    {
        if entree.path().extension().is_some_and(|e| e == "swift") {
            source.push_str(&std::fs::read_to_string(entree.path()).expect("modèle lisible"));
            source.push('\n');
        }
    }

    let contrat = contrat();
    for (cote_swift, cote_contrat) in [
        ("accountPurgeDays", "ACCOUNT_PURGE_DAYS"),
        ("bioMaxChars", "BIO_MAX_CHARS"),
        ("conversationMaxChars", "CONVERSATION_MAX_CHARS"),
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
        "janvier",
        "février",
        "mars",
        "avril",
        "mai",
        "juin",
        "juillet",
        "août",
        "septembre",
        "octobre",
        "novembre",
        "décembre",
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
        crate::droits::HORIZON_CREDIT_JOURS,
        60,
        "la borne du crédit ne correspond plus à ce qui est vendu"
    );
    assert!(
        crate::droits::HORIZON_CREDIT_JOURS
            <= crate::droits::droits_pour("grandtour").jours_a_l_avance,
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
    let chemin =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/src/pages/documents.ts");
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

/// Chaque adresse que l'application appelle existe dans le routeur.
///
/// ## Pourquoi aucun compilateur ne l'attrape
///
/// Une adresse est une chaîne de caractères des deux côtés. « /v1/feed » au
/// lieu de « /v1/plans » compile parfaitement en Swift, part en production, et
/// rend une 404 que personne ne relie à une faute de frappe. Le code Swift de
/// ce dépôt n'a jamais été compilé, faute de macOS — mais même compilé, il
/// n'aurait rien dit de celle-là.
///
/// J'ai fait exactement cette faute en écrivant un test aujourd'hui : « /v1/feed »
/// pour le fil, qui s'appelle « /v1/plans ». Le test a rendu 404, et c'est le
/// seul endroit où cela s'est vu.
///
/// Le rapprochement se fait sur les chaînes : les paramètres de chemin sont
/// normalisés de part et d'autre — `{id}` côté axum, `\(id)` côté Swift.
#[test]
fn les_adresses_appelees_par_l_application_existent_dans_le_routeur() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ios");
    if !racine.is_dir() {
        eprintln!(
            "application iOS absente en {} — accord non vérifié",
            racine.display()
        );
        return;
    }

    let exposees = adresses_du_routeur();
    let appelees = adresses_appelees(&racine);
    assert!(
        !appelees.is_empty(),
        "aucune adresse trouvée dans l'application : le test ne vérifie plus rien"
    );

    let inconnues: Vec<&String> = appelees.iter().filter(|a| !exposees.contains(*a)).collect();
    assert!(
        inconnues.is_empty(),
        "l'application appelle des adresses que le routeur n'expose pas : {inconnues:?}"
    );
}

/// Le client réseau ne décode aucune date avec une stratégie qui refuse les
/// millisecondes.
///
/// `iso8601` de l'API met TOUJOURS des millisecondes — `SecondsFormat::Millis`.
/// Or `.iso8601` de Foundation s'appuie sur `.withInternetDateTime`, qui ne les
/// accepte pas : une date de l'API décodée par cette stratégie échoue, toujours.
///
/// Le client a un décodeur qui essaie les deux formes. Le renouvellement de
/// session, lui, s'en fabriquait un second en `.iso8601`, parce qu'il est
/// statique et que le premier était propre à l'instance. Chaque renouvellement
/// échouait donc à décoder sa réponse, et la session tombait au bout du quart
/// d'heure du jeton d'accès.
///
/// Aucun compilateur n'aurait rien dit : les deux décodeurs sont valides, ils
/// ne lisent simplement pas le même format. Et ce code Swift n'a jamais été
/// compilé de toute façon.
///
/// ## Pourquoi le contrôle s'arrête à la couche réseau
///
/// Le trousseau et la complication de la montre emploient aussi `.iso8601` —
/// et ils ont raison. Ils ENCODENT avec la même stratégie qu'ils décodent :
/// l'aller-retour est local et symétrique, personne n'y lit du JSON de l'API.
/// Élargir la règle à ces fichiers reviendrait à corriger du code qui marche.
#[test]
fn le_client_reseau_ne_decode_aucune_date_sans_millisecondes() {
    let reseau = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ios/WeaveKit/Sources/WeaveKit/Networking");
    if !reseau.is_dir() {
        eprintln!(
            "client iOS absent en {} — accord non vérifié",
            reseau.display()
        );
        return;
    }

    // L'API met bien des millisecondes : si cela changeait, ce test n'aurait
    // plus lieu d'être, et mieux vaut qu'il le dise que de garder une règle
    // devenue sans objet.
    let date = crate::temps::iso8601(chrono::Utc::now());
    assert!(
        date.contains('.'),
        "l'API n'écrit plus de millisecondes ({date}) : cette règle est à revoir"
    );

    let mut fautifs = Vec::new();
    for entree in std::fs::read_dir(&reseau)
        .expect("le répertoire réseau")
        .flatten()
    {
        let chemin = entree.path();
        if chemin.extension().is_some_and(|e| e == "swift") {
            let source = std::fs::read_to_string(&chemin).unwrap_or_default();
            if source.contains("dateDecodingStrategy = .iso8601") {
                fautifs.push(chemin.display().to_string());
            }
        }
    }

    assert!(
        fautifs.is_empty(),
        "« .iso8601 » refuse les millisecondes que l'API écrit toujours — \
         il faut le décodeur partagé, qui accepte les deux formes : {fautifs:?}"
    );
}

/// Les adresses déclarées par les routeurs, paramètres normalisés en `{}`.
fn adresses_du_routeur() -> std::collections::BTreeSet<String> {
    let mut adresses = std::collections::BTreeSet::new();
    let routes = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/routes");
    for entree in std::fs::read_dir(routes)
        .expect("le répertoire des routes")
        .flatten()
    {
        let source = std::fs::read_to_string(entree.path()).unwrap_or_default();
        let mut reste = source.as_str();
        // `.route(` peut être suivi d'un retour à la ligne : la déclaration de
        // la route des photos l'est, à cause de sa limite de corps.
        while let Some(i) = reste.find(".route(") {
            reste = &reste[i + ".route(".len()..];
            let apres = reste.trim_start();
            if let Some(fin) = apres
                .strip_prefix('"')
                .and_then(|s| s.find('"').map(|f| &s[..f]))
            {
                adresses.insert(normaliser_parametres(fin));
            }
        }
    }
    adresses
}

/// Les adresses citées par le client Swift, hors tests.
fn adresses_appelees(racine: &std::path::Path) -> std::collections::BTreeSet<String> {
    let mut adresses = std::collections::BTreeSet::new();
    let mut a_visiter = vec![racine.to_path_buf()];
    while let Some(chemin) = a_visiter.pop() {
        let Ok(entrees) = std::fs::read_dir(&chemin) else {
            continue;
        };
        for entree in entrees.flatten() {
            let chemin = entree.path();
            if chemin.is_dir() {
                // Les tests du paquet Swift citent des adresses de simulation.
                if chemin.file_name().is_some_and(|n| n == "Tests") {
                    continue;
                }
                a_visiter.push(chemin);
            } else if chemin.extension().is_some_and(|e| e == "swift") {
                let source = std::fs::read_to_string(&chemin).unwrap_or_default();
                for morceau in source.split('"').skip(1).step_by(2) {
                    if morceau.starts_with("/v1/") || morceau.starts_with("/media/") {
                        adresses.insert(normaliser_parametres(morceau));
                    }
                }
            }
        }
    }
    adresses
}

/// `{id}` d'axum et `\(id)` de Swift désignent la même chose : un paramètre.
fn normaliser_parametres(adresse: &str) -> String {
    let mut sortie = String::with_capacity(adresse.len());
    let mut reste = adresse;
    loop {
        let ouvrant = reste.find("\\(").map(|i| (i, "\\(", ')'));
        let accolade = reste.find('{').map(|i| (i, "{", '}'));
        let premier = match (ouvrant, accolade) {
            (Some(a), Some(b)) => Some(if a.0 <= b.0 { a } else { b }),
            (a, b) => a.or(b),
        };
        let Some((i, prefixe, fermant)) = premier else {
            sortie.push_str(reste);
            break;
        };
        sortie.push_str(&reste[..i]);
        sortie.push_str("{}");
        let apres = &reste[i + prefixe.len()..];
        match apres.find(fermant) {
            Some(f) => reste = &apres[f + 1..],
            None => break,
        }
    }
    // Une barre oblique finale ne distingue pas deux adresses ici.
    let taille = sortie.trim_end_matches('/').len();
    sortie.truncate(taille.max(1));
    sortie
}

/// La boucle qui compose le fil n'interroge pas la base.
///
/// Elle le faisait pour CHAQUE plan : l'auteur, sa fiche quand le genre est
/// filtré, et les demandes reçues. Jusqu'à trois cents plans sont lus — soit
/// jusqu'à neuf cents allers-retours pour composer un seul fil, sur une base
/// qui vit au bout du réseau. C'est le chemin le plus chaud du produit :
/// l'écran d'accueil de l'application.
///
/// Tout est désormais chargé en trois requêtes avant la boucle, et la boucle
/// ne fait plus que trier en mémoire.
///
/// ## Pourquoi ce test lit le source
///
/// Compter les requêtes réellement exécutées demanderait la journalisation de
/// SeaORM, donc une dépendance de plus dans le binaire de production — pour
/// éprouver une propriété qui se lit dans le texte. Et un test de durée ne
/// dirait rien : sur SQLite en mémoire, neuf cents requêtes sont rapides.
///
/// Le contrôle est donc structurel, et il est franc sur ce qu'il vérifie : que
/// le corps de la boucle ne contient aucun appel à la base.
#[test]
fn la_boucle_du_fil_n_interroge_pas_la_base() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/routes/fil.rs"),
    )
    .expect("le module du fil");

    let marque = "for ligne in lignes {";
    let debut = source
        .find(marque)
        .expect("la boucle qui compose le fil a changé de forme : ce test est à revoir");

    // Le corps s'arrête à l'accolade qui referme le `for`.
    let apres = &source[debut + marque.len()..];
    let mut profondeur = 1usize;
    let mut fin = apres.len();
    for (i, c) in apres.char_indices() {
        match c {
            '{' => profondeur += 1,
            '}' => {
                profondeur -= 1;
                if profondeur == 0 {
                    fin = i;
                    break;
                }
            }
            _ => {}
        }
    }
    let corps = &apres[..fin];

    let appels: Vec<&str> = ["(&state.db)", "(&transaction)", ".all(db)", ".one(db)"]
        .into_iter()
        .filter(|a| corps.contains(a))
        .collect();

    assert!(
        appels.is_empty(),
        "la boucle du fil interroge la base ({appels:?}) : une requête par plan, \
         jusqu'à trois cents fois. Tout doit être chargé avant la boucle."
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
    let cle = if champ == "sku" {
        "priceCents: "
    } else {
        "monthlyPriceCents: "
    };
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
        eprintln!(
            "modèles iOS absents en {} — accord non vérifié",
            racine.display()
        );
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
        (
            "Gender.swift",
            "Gender",
            liste_du_contrat(&contrat, "GENDERS"),
        ),
        (
            "Plan.swift",
            "PlanState",
            liste_du_type(&domaine, "PlanState"),
        ),
        // Les motifs de signalement ne passent pas par le contrat partagé :
        // le site n'en a pas l'usage. L'accord se tient donc directement
        // entre l'application et la route qui les refuse.
        (
            "Moderation.swift",
            "ReportReason",
            trier(&crate::routes::moderation::MOTIFS),
        ),
        // Un objet de consentement que l'application nommerait autrement se
        // ferait refuser par la route, et l'accord ne pourrait pas être donné
        // — le traitement resterait bloqué sans que rien ne dise pourquoi.
        (
            "Consentement.swift",
            "ConsentKind",
            liste_du_contrat(&contrat, "CONSENT_KINDS"),
        ),
        // Les quatre suivants tiennent les champs que le lancement traverse.
        //
        // `Me` porte `status` et `tier`, et l'application lit `/v1/me` avant
        // tout le reste : un palier ou un statut que Swift ignore ne fait pas
        // tomber un écran, il fait tomber l'ouverture. Les deux autres
        // décodent des tableaux — une demande dans un état inconnu emporte
        // « Mes demandes », un message d'un auteur inconnu emporte le fil de
        // la conversation.
        (
            "Account.swift",
            "AccountStatus",
            liste_du_type(&domaine, "AccountStatus"),
        ),
        (
            "Account.swift",
            "PlanTier",
            liste_du_contrat(&catalogue(), "PLAN_TIERS"),
        ),
        (
            "Plan.swift",
            "RequestState",
            liste_du_type(&domaine, "RequestState"),
        ),
        (
            "Plan.swift",
            "MessageAuthor",
            liste_du_type(&domaine, "MessageAuthor"),
        ),
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
        eprintln!(
            "modèle iOS absent en {} — accord non vérifié",
            chemin.display()
        );
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

/// Chaque requête de l'application iOS annonce la langue de son utilisateur.
///
/// Le service traduit ses messages, et l'application les rend TELS QUELS —
/// `WeaveAPI.swift` le dit de son `message`. Mais il n'a qu'un seul moyen de
/// savoir dans quelle langue répondre : l'en-tête `Accept-Language`. Une
/// requête qui l'omet reçoit du français, et une application anglaise affiche
/// « Ce plan est complet. » au milieu de son propre texte.
///
/// L'oubli ne casse rien de visible côté Swift : la requête part, le serveur
/// répond, les données arrivent. Seule la langue du refus change — et on ne
/// s'en aperçoit qu'en lisant une erreur, sur un téléphone réglé dans une
/// autre langue que la sienne.
#[test]
fn chaque_requete_ios_annonce_sa_langue() {
    let chemin = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/ios/WeaveKit/Sources/WeaveKit/Networking/WeaveAPI.swift");
    let source = std::fs::read_to_string(&chemin)
        .unwrap_or_else(|e| panic!("WeaveAPI.swift illisible en {} : {e}", chemin.display()));

    // Chaque `URLRequest(url:` ouvre la construction d'une requête. L'en-tête
    // doit être posé avant que celle-ci ne parte.
    let constructions = source.matches("URLRequest(url:").count();
    assert!(
        constructions > 0,
        "aucune construction de requête trouvée : le fichier a changé de forme"
    );

    let annonces = source
        .matches("forHTTPHeaderField: \"Accept-Language\"")
        .count();
    assert_eq!(
        annonces, constructions,
        "{constructions} requêtes construites mais {annonces} annoncent leur langue : \
         l'une d'elles recevra du français quoi qu'il arrive"
    );

    // L'en-tête vient des langues du système, et n'est pas écrit en dur : une
    // constante « fr » compilerait et annulerait toute la traduction.
    assert!(
        source.contains("Locale.preferredLanguages"),
        "la langue annoncée ne vient pas des réglages du téléphone"
    );
}

/// Le catalogue de chaînes iOS reste en phase avec le code qui les affiche.
///
/// En SwiftUI, `Text("Publier")` passe par `LocalizedStringKey` : le texte
/// français EST la clé. Renommer une phrase dans le code sans la renommer dans
/// le catalogue ne casse rien de visible — la chaîne retombe simplement sur le
/// français, dans une application anglaise, sans erreur ni avertissement.
///
/// Le contrôle porte sur les deux sens :
///
/// - chaque clé du catalogue existe encore dans le code Swift ; une clé
///   orpheline est une phrase qu'on croit traduite et qui ne l'est plus ;
/// - chaque clé a bien ses deux traductions, non vides.
#[test]
fn le_catalogue_ios_est_en_phase_avec_le_code() {
    let ios = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/ios");
    let catalogue = std::fs::read_to_string(ios.join("Weave/Localizable.xcstrings"))
        .expect("Localizable.xcstrings lisible");
    let catalogue: serde_json::Value =
        serde_json::from_str(&catalogue).expect("le catalogue est du JSON valide");

    assert_eq!(
        catalogue["sourceLanguage"], "fr",
        "la langue source a changé"
    );

    // Tout le Swift de l'application, d'un seul tenant : une clé peut être
    // écrite dans n'importe quelle vue.
    let mut sources = String::new();
    empiler_le_swift(&ios, &mut sources);
    assert!(
        sources.len() > 10_000,
        "seulement {} octets de Swift lus : le parcours des fichiers a dérivé",
        sources.len()
    );

    // Ces noms sont ceux de produits déclarés dans App Store Connect, et
    // « Pause » est le même mot en anglais : les seules identités voulues.
    const IDENTITES_VOULUES: [&str; 3] = ["Bilan", "Escale", "Pause"];

    let chaines = catalogue["strings"].as_object().expect("des chaînes");
    assert!(
        chaines.len() >= 120,
        "catalogue étonnamment court : {}",
        chaines.len()
    );

    let mut orphelines = Vec::new();
    for (cle, entree) in chaines {
        // La clé telle qu'elle est ÉCRITE dans le source : les sauts de ligne
        // y sont des échappements, pas des retours à la ligne.
        let litteral = cle
            .replace('\\', "\\\\")
            .replace('\n', "\\n")
            .replace('"', "\\\"");
        if !sources.contains(&format!("\"{litteral}\"")) {
            orphelines.push(cle.clone());
            continue;
        }

        for langue in ["en", "es"] {
            let valeur = entree["localizations"][langue]["stringUnit"]["value"]
                .as_str()
                .unwrap_or_else(|| panic!("« {cle} » n'a pas de traduction en « {langue} »"));
            assert!(
                !valeur.is_empty(),
                "« {cle} » : traduction « {langue} » vide"
            );
            if !IDENTITES_VOULUES.contains(&cle.as_str()) {
                assert_ne!(valeur, cle, "« {cle} » : « {langue} » reprend le français");
            }
        }
    }

    assert!(
        orphelines.is_empty(),
        "ces clés ne sont plus dans le code Swift, et leur traduction ne sert plus : {orphelines:?}"
    );
}

/// Concatène tout le Swift d'un répertoire, récursivement.
fn empiler_le_swift(repertoire: &std::path::Path, sortie: &mut String) {
    let Ok(entrees) = std::fs::read_dir(repertoire) else {
        return;
    };
    for entree in entrees.flatten() {
        let chemin = entree.path();
        if chemin.is_dir() {
            empiler_le_swift(&chemin, sortie);
        } else if chemin.extension().is_some_and(|e| e == "swift")
            && let Ok(contenu) = std::fs::read_to_string(&chemin)
        {
            sortie.push_str(&contenu);
            sortie.push('\n');
        }
    }
}

/// La position est arrondie SUR L'APPAREIL, comme la politique le promet.
///
/// La politique de confidentialité le dit en toutes lettres, et en gras : « La
/// position est arrondie sur votre appareil avant l'envoi […] Nos serveurs ne
/// disposent à aucun moment de vos coordonnées exactes : ce n'est pas une
/// politique de rétention, c'est une donnée que nous n'avons pas. »
///
/// L'application envoyait les coordonnées exactes, et le serveur les
/// arrondissait au dépôt. L'affirmation était donc fausse : les coordonnées
/// exactes traversaient le réseau, entraient dans le corps de la requête, et
/// passaient par tout ce qui journalise une requête.
///
/// Rien ne l'aurait signalé. Les deux bouts fonctionnaient, la base ne
/// contenait bien que des positions arrondies, et seul le trajet mentait.
#[test]
fn la_position_est_arrondie_avant_de_quitter_l_appareil() {
    let client = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/ios/WeaveKit/Sources/WeaveKit/Networking/WeaveAPI.swift");
    let swift = std::fs::read_to_string(&client)
        .unwrap_or_else(|e| panic!("WeaveAPI.swift illisible en {} : {e}", client.display()));

    for champ in ["latitude", "longitude"] {
        let attendu = format!("{champ}: Self.arrondirPosition({champ})");
        assert!(
            swift.contains(&attendu),
            "« {champ} » part sans être arrondie : la politique promet le contraire"
        );
    }

    // La grille du client est celle du serveur.
    //
    // Arrondir sur l'appareil à un pas plus fin laisserait le serveur
    // ré-arrondir et déplacer le point : l'arrondi de l'appareil ne
    // garantirait plus rien de ce qui est stocké.
    let pas_client: f64 = swift
        .split("static let pasDeLaGrillePosition = ")
        .nth(1)
        .and_then(|reste| reste.split_whitespace().next())
        .and_then(|v| v.parse().ok())
        .expect("le pas de la grille a disparu du client");

    let serveur = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/routes/me.rs"),
    )
    .expect("me.rs lisible");
    let facteur: f64 = serveur
        .split("let lat = (corps.latitude * ")
        .nth(1)
        .and_then(|reste| reste.split(')').next())
        .and_then(|v| v.trim().parse().ok())
        .expect("l'arrondi du serveur a changé de forme");

    assert!(
        (pas_client - 1.0 / facteur).abs() < f64::EPSILON,
        "l'appareil arrondit au {pas_client}° et le serveur au {}° : le serveur déplacerait le point",
        1.0 / facteur
    );
}

/// Chaque autorisation déclarée correspond à une API réellement appelée.
///
/// Un texte d'autorisation n'est pas une intention. C'est ce qu'iOS affiche à
/// l'utilisateur au moment de la demande, et c'est ce dont se nourrit
/// l'étiquette de confidentialité de la fiche App Store. En déclarer une que
/// le code n'emploie jamais fait annoncer une collecte qui n'a pas lieu.
///
/// L'application en portait deux :
///
/// - la POSITION, avec un texte promettant une valeur « arrondie au kilomètre
///   avant d'être enregistrée » — alors qu'aucune autorisation de localisation
///   n'est demandée nulle part. La ville est saisie, puis géocodée sur
///   l'appareil ;
/// - le MICROPHONE, pour « un fragment vocal de huit secondes » qui n'existe
///   pas : rien n'importe AVFoundation.
///
/// Rien ne l'aurait signalé. Une autorisation déclarée et jamais demandée ne
/// casse aucune compilation et ne fait échouer aucun écran — elle se voit sur
/// la fiche App Store, c'est-à-dire trop tard.
#[test]
fn chaque_autorisation_declaree_est_vraiment_employee() {
    let ios = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/ios");
    let plist = std::fs::read_to_string(ios.join("Weave/Info.plist")).expect("Info.plist lisible");

    let mut sources = String::new();
    empiler_le_swift(&ios, &mut sources);
    assert!(sources.len() > 10_000, "le parcours du Swift a dérivé");

    // Pour chaque autorisation, ce qu'il faudrait appeler pour la déclencher.
    // Une autorisation absente de cette table et présente dans le `plist`
    // arrête le test : c'est le cas qu'on n'a pas su juger.
    let attendus: [(&str, &[&str]); 5] = [
        (
            "NSLocationWhenInUseUsageDescription",
            &["CLLocationManager"],
        ),
        (
            "NSLocationAlwaysAndWhenInUseUsageDescription",
            &["CLLocationManager"],
        ),
        (
            "NSMicrophoneUsageDescription",
            &["AVAudioRecorder", "AVAudioSession", "AVCaptureDevice"],
        ),
        (
            "NSCameraUsageDescription",
            &["UIImagePickerController", "AVCaptureSession"],
        ),
        (
            "NSPhotoLibraryUsageDescription",
            &[
                "PhotosPicker",
                "PHPicker",
                "PHPhotoLibrary",
                "UIImagePickerController",
            ],
        ),
    ];

    for cle in plist
        .lines()
        .filter_map(|ligne| ligne.trim().strip_prefix("<key>"))
        .filter_map(|ligne| ligne.strip_suffix("</key>"))
        .filter(|cle| cle.ends_with("UsageDescription"))
    {
        let Some((_, marqueurs)) = attendus.iter().find(|(nom, _)| *nom == cle) else {
            panic!("« {cle} » est déclarée, et ce test ne sait pas à quelle API la rapporter");
        };
        assert!(
            marqueurs.iter().any(|m| sources.contains(m)),
            "« {cle} » est déclarée mais aucune de ses API n'est appelée : \
             l'étiquette App Store annoncera une collecte qui n'a pas lieu"
        );
    }
}

/// Et la réciproque : une API employée sans autorisation déclarée.
///
/// C'est l'erreur inverse, et elle ne se voit qu'à l'exécution : iOS TUE
/// l'application au moment de la demande quand le texte manque. Pas une
/// erreur de compilation, pas un avertissement — un plantage, sur l'appareil
/// d'un utilisateur, à l'écran précis où l'on demandait l'autorisation.
#[test]
fn aucune_api_sensible_n_est_employee_sans_autorisation() {
    let ios = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/ios");
    let plist = std::fs::read_to_string(ios.join("Weave/Info.plist")).expect("Info.plist lisible");

    let mut sources = String::new();
    empiler_le_swift(&ios, &mut sources);

    for (marqueur, cle) in [
        ("CLLocationManager", "NSLocationWhenInUseUsageDescription"),
        ("AVAudioRecorder", "NSMicrophoneUsageDescription"),
        ("AVCaptureSession", "NSCameraUsageDescription"),
        ("PHPhotoLibrary", "NSPhotoLibraryUsageDescription"),
    ] {
        if sources.contains(marqueur) {
            assert!(
                plist.contains(cle),
                "« {marqueur} » est appelée sans « {cle} » : iOS tuera l'application \
                 au moment de la demande"
            );
        }
    }
}

/// `app.json` fournit tout ce que le binaire EXIGE en production.
///
/// C'est le manifeste que lit le bouton « Deploy to Heroku », et il décide de
/// ce qui existe au premier démarrage. Une variable ajoutée aux exigences du
/// binaire et oubliée ici ne casse rien à la compilation, rien aux tests, et
/// rien en développement — où `exiger` accepte un repli. Elle arrête le dyno
/// au tout premier démarrage d'un déploiement neuf, c'est-à-dire chez
/// quelqu'un qui découvre le projet.
///
/// Deux variables ne figurent pas dans `env` et n'ont pas à y figurer : les
/// modules complémentaires les posent eux-mêmes.
#[test]
fn le_manifeste_heroku_fournit_ce_que_le_binaire_exige() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    let manifeste: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(racine.join("app.json")).expect("app.json"))
            .expect("app.json est du JSON valide");

    let declarees: std::collections::BTreeSet<String> = manifeste["env"]
        .as_object()
        .expect("app.json déclare des variables")
        .keys()
        .cloned()
        .collect();

    // Ce que les modules complémentaires posent d'eux-mêmes, sans passer par
    // `env` : les déclarer là les ferait demander à la main.
    let par_les_modules = ["DATABASE_URL", "REDIS_URL"];

    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/env.rs"),
    )
    .expect("env.rs lisible");

    // Chaque `exiger("X", …)` : ce que le binaire refuse de démarrer sans.
    let mut exigees = Vec::new();
    for morceau in source.split("exiger(").skip(1) {
        let Some(nom) = morceau
            .trim_start()
            .strip_prefix('"')
            .and_then(|reste| reste.split('"').next())
        else {
            continue;
        };
        if nom.chars().all(|c| c.is_ascii_uppercase() || c == '_') && !nom.is_empty() {
            exigees.push(nom.to_string());
        }
    }
    assert!(
        exigees.len() >= 3,
        "seulement {} variables exigées lues : l'analyse d'env.rs a dérivé",
        exigees.len()
    );

    let manquantes: Vec<&String> = exigees
        .iter()
        .filter(|nom| !declarees.contains(*nom) && !par_les_modules.contains(&nom.as_str()))
        .collect();

    assert!(
        manquantes.is_empty(),
        "le binaire exige ces variables et `app.json` ne les fournit pas : {manquantes:?} — \
         un déploiement neuf s'arrêterait au premier démarrage"
    );
}

/// `app.json` décrit la pile qu'on déploie vraiment.
///
/// Ses mots-clés annonçaient « elysia » et « prisma » : l'API TypeScript
/// d'avant le portage. Le manifeste est public — il s'affiche sur le bouton de
/// déploiement et dans le dépôt — et décrivait une pile qui n'existe plus.
#[test]
fn le_manifeste_ne_nomme_pas_une_pile_abandonnee() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifeste: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(racine.join("app.json")).expect("app.json"))
            .expect("app.json est du JSON valide");

    let mots: Vec<String> = manifeste["keywords"]
        .as_array()
        .expect("des mots-clés")
        .iter()
        .filter_map(|v| v.as_str().map(str::to_lowercase))
        .collect();

    for abandonnee in ["elysia", "prisma", "typescript", "node"] {
        assert!(
            !mots.iter().any(|m| m == abandonnee),
            "« {abandonnee} » n'est plus la pile de Weave : l'API est en Rust"
        );
    }
}

/// Le fil rend exactement ce que l'application sait décoder.
///
/// C'est le contrat le plus coûteux à rompre, et le seul qui ne se voie
/// nulle part avant l'exécution. `Codable` synthétise le décodage d'après les
/// propriétés déclarées : une clé manquante pour une propriété NON FACULTATIVE
/// fait lever `keyNotFound`, et ce n'est pas le plan qui tombe — c'est le
/// tableau entier, donc l'écran entier.
///
/// Côté Rust rien ne le signale : `PlanDuFil` sérialise sans savoir qui la
/// lit. Côté Swift rien non plus : le modèle compile seul. Les deux moitiés
/// sont correctes, et ne s'emboîtent pas.
#[test]
fn le_fil_rend_ce_que_l_application_sait_decoder() {
    let ios = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/ios");
    let swift = std::fs::read_to_string(ios.join("WeaveKit/Sources/WeaveKit/Models/Plan.swift"))
        .expect("Plan.swift lisible");

    let rust = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/routes/fil.rs"),
    )
    .expect("fil.rs lisible");

    let envoyees = champs_serialises(&rust, "PlanDuFil");
    assert!(
        envoyees.len() >= 8,
        "seulement {} champs lus dans PlanDuFil : l'analyse a dérivé",
        envoyees.len()
    );

    let attendues = proprietes_swift(&swift, "public struct Plan: Codable");
    assert!(
        attendues.len() >= 8,
        "seulement {} propriétés lues dans Plan : l'analyse a dérivé",
        attendues.len()
    );

    let manquantes: Vec<&String> = attendues
        .iter()
        .filter(|(_, facultative)| !facultative)
        .map(|(nom, _)| nom)
        .filter(|nom| !envoyees.contains(*nom))
        .collect();

    assert!(
        manquantes.is_empty(),
        "le fil n'envoie pas {manquantes:?}, que `Plan` déclare obligatoires : \
         le décodage lèvera `keyNotFound` et l'écran du fil sera vide"
    );
}

/// Le compte rend exactement ce que l'application sait décoder.
///
/// Même risque que pour le fil, et pour la même raison : `Me` est une des
/// trois structures typées qui atteignent le client, et une structure typée
/// sérialise sans savoir qui la lit. Les autres réponses sont bâties clé par
/// clé avec `json!`, à côté de ce qu'elles décrivent ; celles-ci sont écrites
/// une fois, puis oubliées.
///
/// Elle correspond aujourd'hui. Ce test est là pour qu'elle continue.
#[test]
fn le_compte_rend_ce_que_l_application_sait_decoder() {
    let swift = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/ios/WeaveKit/Sources/WeaveKit/Models/Account.swift"),
    )
    .expect("Account.swift lisible");

    let rust = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/routes/me.rs"),
    )
    .expect("me.rs lisible");

    let envoyees = champs_serialises(&rust, "Me");
    assert!(
        envoyees.len() >= 10,
        "seulement {} champs lus dans Me : l'analyse a dérivé",
        envoyees.len()
    );

    let attendues = proprietes_swift(&swift, "public struct Me: Codable");
    assert!(
        attendues.len() >= 10,
        "seulement {} propriétés lues : l'analyse a dérivé",
        attendues.len()
    );

    let manquantes: Vec<&String> = attendues
        .iter()
        .filter(|(_, facultative)| !facultative)
        .map(|(nom, _)| nom)
        .filter(|nom| !envoyees.contains(*nom))
        .collect();

    assert!(
        manquantes.is_empty(),
        "le compte n'envoie pas {manquantes:?}, que `Me` déclare obligatoires : \
         le décodage lèvera `keyNotFound` et l'écran du compte sera vide"
    );
}

/// Les clés JSON d'une structure Rust `#[serde(rename_all = "camelCase")]`.
fn champs_serialises(source: &str, nom: &str) -> std::collections::BTreeSet<String> {
    let debut = source
        .find(&format!("struct {nom} {{"))
        .unwrap_or_else(|| panic!("la structure « {nom} » a disparu"));
    let fin = source[debut..]
        .find("\n}")
        .unwrap_or_else(|| panic!("« {nom} » sans fin"))
        + debut;

    source[debut..fin]
        .lines()
        .skip(1)
        .map(str::trim)
        .filter(|l| !l.starts_with("//") && !l.starts_with("#["))
        .map(|l| l.strip_prefix("pub ").unwrap_or(l))
        .filter_map(|l| l.split(':').next())
        .map(str::trim)
        .filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
        .map(en_camel)
        .collect()
}

/// `starts_at` → `startsAt`, comme le fait serde.
fn en_camel(serpent: &str) -> String {
    let mut sortie = String::with_capacity(serpent.len());
    let mut majuscule = false;
    for c in serpent.chars() {
        if c == '_' {
            majuscule = true;
        } else if majuscule {
            sortie.extend(c.to_uppercase());
            majuscule = false;
        } else {
            sortie.push(c);
        }
    }
    sortie
}

/// Les propriétés stockées d'une structure Swift, et leur caractère facultatif.
///
/// Une `CodingKeys` renomme : la clé JSON est alors celle qu'elle donne, pas
/// le nom de la propriété.
fn proprietes_swift(source: &str, entete: &str) -> Vec<(String, bool)> {
    let debut = source
        .find(entete)
        .unwrap_or_else(|| panic!("« {entete} » a disparu"));
    let fin = source[debut..].find("\n}").expect("structure sans fin") + debut;
    let corps = &source[debut..fin];

    corps
        .lines()
        .map(str::trim)
        .filter_map(|l| l.strip_prefix("public let "))
        .filter_map(|l| {
            let (nom, reste) = l.split_once(':')?;
            let nom = nom.trim();
            // Une propriété calculée n'est pas décodée.
            if reste.contains('{') {
                return None;
            }
            let renommee = corps
                .lines()
                .map(str::trim)
                .find_map(|c| c.strip_prefix(&format!("case {nom} = \"")))
                .and_then(|c| c.split('"').next())
                .map(str::to_string);
            Some((
                renommee.unwrap_or_else(|| nom.to_string()),
                reste.trim().ends_with('?'),
            ))
        })
        .collect()
}

/// Tout ce que le catalogue vend, l'API sait le vendre.
///
/// Le contrôle des prix parcourt la liste de l'API et la compare au contrat :
/// il attrape un prix qui diverge, jamais un produit que le contrat annonce et
/// que l'API ignore. Or c'est ce sens-là qui coûte de l'argent — le site et
/// l'application affichent le catalogue partagé, et StoreKit encaisserait un
/// achat que `unite_depuis_produit` ne reconnaîtrait pas. La transaction est
/// refusée après le paiement.
#[test]
fn tout_ce_que_le_catalogue_vend_est_vendable_par_l_api() {
    let catalogue = catalogue();

    let debut = catalogue
        .find("export const UNIT_SKUS = [")
        .expect("UNIT_SKUS a disparu du catalogue");
    let fin = catalogue[debut..].find(']').expect("UNIT_SKUS sans fin") + debut;
    let declaration = &catalogue[debut..fin];

    let annonces: Vec<String> = declaration
        .match_indices('"')
        .collect::<Vec<_>>()
        .chunks(2)
        .filter_map(|paire| match paire {
            [(ouvre, _), (ferme, _)] => Some(declaration[ouvre + 1..*ferme].to_string()),
            _ => None,
        })
        .collect();
    assert!(
        annonces.len() >= 5,
        "seulement {} SKU lus : l'analyse du catalogue a dérivé",
        annonces.len()
    );

    let vendables: std::collections::BTreeSet<&str> = crate::routes::billing::unites_pour_test()
        .into_iter()
        .map(|(sku, _)| sku)
        .collect();

    let orphelins: Vec<&String> = annonces
        .iter()
        .filter(|sku| !vendables.contains(sku.as_str()))
        .collect();

    assert!(
        orphelins.is_empty(),
        "le catalogue annonce {orphelins:?}, que l'API ne sait pas vendre : \
         l'achat serait encaissé par Apple puis refusé ici"
    );
}

/// Tout crédit que l'API vend se dépense quelque part.
///
/// C'est l'autre bout, et le plus cher : un crédit accordé qu'aucune route ne
/// consomme est de l'argent pris pour rien. Rien ne le signale — le solde
/// monte, l'achat « réussit », et le produit ne rend simplement jamais ce
/// qu'il promet.
///
/// La dépense se reconnaît à un appel qui exige le crédit par son nom. Les
/// deux formes comptent : celle qui ouvre sa propre transaction et celle qui
/// s'inscrit dans une transaction déjà ouverte.
#[test]
fn tout_credit_vendu_se_depense_quelque_part() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut sources = String::new();
    empiler_le_rust(&racine, &mut sources);
    assert!(
        sources.len() > 100_000,
        "seulement {} octets de Rust lus : le parcours a dérivé",
        sources.len()
    );

    for (sku, _) in crate::routes::billing::unites_pour_test() {
        // La dépense se reconnaît à un appel qui EXIGE le crédit par son nom.
        //
        // Le motif est volontairement étroit : chercher seulement `"{sku}"`
        // quelque part aurait trouvé la table des unités elle-même, et le test
        // aurait passé pour tout le monde sans rien vérifier.
        let depense = sources.contains(&format!("exiger_credit(&state, &compte.id, \"{sku}\""))
            || sources.contains(&format!(
                "exiger_credit_dans(&transaction, &compte.id, \"{sku}\""
            ));
        assert!(
            depense,
            "« {sku} » se vend et ne se dépense nulle part : le crédit s'accumule \
             et le produit ne rend jamais ce qu'il promet"
        );
    }
}

/// Concatène tout le Rust d'un répertoire, hors tests.
fn empiler_le_rust(repertoire: &std::path::Path, sortie: &mut String) {
    let Ok(entrees) = std::fs::read_dir(repertoire) else {
        return;
    };
    for entree in entrees.flatten() {
        let chemin = entree.path();
        if chemin.is_dir() {
            if chemin.file_name().is_some_and(|n| n == "tests") {
                continue;
            }
            empiler_le_rust(&chemin, sortie);
        } else if chemin.extension().is_some_and(|e| e == "rs")
            && let Ok(contenu) = std::fs::read_to_string(&chemin)
        {
            sortie.push_str(&contenu);
            sortie.push('\n');
        }
    }
}

/// Ce que le serveur pousse sur l'écran verrouillé est ce que l'appareil sait
/// lire.
///
/// Trois accords tiennent une Live Activity, et aucun n'était gardé.
///
///  • `attributes-type` est repris **tel quel** par ActivityKit pour retrouver
///    le type à instancier. Le commentaire de `WeaveActivityAttributes` le dit
///    déjà — « le renommer casse le démarrage à distance » — mais rien ne
///    l'empêchait.
///  • `attributes` doit porter les propriétés stockées de ce type. Il n'y en a
///    qu'une, et une clé absente fait échouer le démarrage.
///  • `content-state` doit porter celles de `ContentState`. C'est le même
///    piège que le fil : une énumération `Codable` non facultative, ou un champ
///    manquant, et rien ne s'affiche — sur un écran verrouillé, où personne ne
///    verra jamais l'erreur.
#[test]
fn la_live_activity_pousse_ce_que_l_appareil_sait_lire() {
    let modeles = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ios/WeaveKit/Sources/WeaveKit/Models/WeaveActivityAttributes.swift");
    if !modeles.is_file() {
        eprintln!(
            "modèle iOS absent en {} — accord non vérifié",
            modeles.display()
        );
        return;
    }
    let swift = std::fs::read_to_string(&modeles).expect("modèle lisible");
    let rust = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/live_activity.rs"),
    )
    .expect("live_activity.rs lisible");

    // 1. Le nom du type, tel qu'ActivityKit le cherche.
    let annonce = rust
        .lines()
        .find_map(|l| l.trim().strip_prefix("const TYPE_ATTRIBUTS: &str = \""))
        .and_then(|l| l.split('"').next())
        .expect("« TYPE_ATTRIBUTS » a disparu du serveur");
    assert!(
        swift.contains(&format!("public struct {annonce}:")),
        "le serveur annonce « {annonce} », que les modèles iOS ne déclarent pas"
    );

    // 2. Les attributs du démarrage.
    //
    // `proprietes_swift` s'arrête au premier « \n} », ce qui, ici, tombe sur la
    // fin de `ContentState`. On repart donc de la fin de ce type imbriqué.
    let apres_content_state = swift
        .find("    public let accountHandle")
        .expect("« accountHandle » a disparu");
    let attributs_swift: Vec<String> = proprietes_swift(
        &swift[apres_content_state..],
        "    public let accountHandle",
    )
    .into_iter()
    .map(|(nom, _)| nom)
    .collect();
    assert_eq!(
        attributs_swift,
        vec!["accountHandle".to_string()],
        "les attributs du type ont changé sans que le serveur suive"
    );
    for attribut in &attributs_swift {
        assert!(
            rust.contains(&format!("\"{attribut}\"")),
            "le serveur n'envoie pas « {attribut} » dans « attributes » : \
             le démarrage à distance échouera"
        );
    }

    // 3. L'état dynamique.
    // Lues à la main plutôt qu'avec `proprietes_swift` : ce helper s'arrête au
    // premier « \n} », qui ferme ici le type ENGLOBANT et non le type imbriqué.
    // Le corps de `ContentState` s'arrête juste avant `accountHandle`.
    let debut_etat = swift
        .find("public struct ContentState")
        .expect("« ContentState » a disparu");
    let attendues: std::collections::BTreeSet<String> = swift[debut_etat..apres_content_state]
        .lines()
        .map(str::trim)
        .filter_map(|l| l.strip_prefix("public let "))
        .filter_map(|l| l.split_once(':'))
        // Une propriété calculée n'est pas décodée.
        .filter(|(_, reste)| !reste.contains('{'))
        .map(|(nom, _)| nom.trim().to_string())
        .collect();
    assert!(
        attendues.len() >= 5,
        "lecture de « ContentState » suspecte : {attendues:?}"
    );

    let envoyees = champs_serialises(&rust, "Etat");
    let manquantes: Vec<&String> = attendues.difference(&envoyees).collect();
    assert!(
        manquantes.is_empty(),
        "« content-state » n'a pas {manquantes:?} — l'appareil ne décodera rien, \
         et l'écran verrouillé restera sur l'état d'avant.\n\
         envoyées : {envoyees:?}"
    );
}

/// Rien de ce que Redis écrit ne doit pouvoir entrer dans le dépôt.
///
/// `dump.rdb` y était suivi. `redis-server` écrit son instantané dans son
/// répertoire courant — la racine du dépôt dès qu'on le lance à la main — et
/// le cache contient le résumé d'identité de chaque personne connectée : nom
/// affiché, pseudonyme, palier, fuseau, puis la composition de son fil, avec
/// les titres de plans, les prénoms, les âges et les adresses de photos. Un
/// fichier suivi posé exactement là où Redis écrit finit par être committé
/// avec tout cela dedans.
///
/// Deux choses l'empêchent, et ce test les tient l'une et l'autre : le fichier
/// est ignoré, et le crochet de session lance Redis sans instantané du tout.
#[test]
fn aucun_instantane_du_cache_ne_peut_entrer_dans_le_depot() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ignores = std::fs::read_to_string(racine.join(".gitignore")).expect(".gitignore lisible");
    for motif in ["dump.rdb", "*.rdb"] {
        assert!(
            ignores.lines().any(|l| l.trim() == motif),
            "« {motif} » ne figure plus dans .gitignore : un instantané du cache \
             peut à nouveau être committé"
        );
    }

    assert!(
        !racine.join("dump.rdb").exists(),
        "un instantané du cache traîne à la racine du dépôt"
    );

    // Le crochet de session lance Redis lui-même : c'est le seul endroit du
    // dépôt qui le fait, et `--save ''` est ce qui garantit qu'aucun instantané
    // n'est écrit, ignoré ou non.
    let crochets = racine.join(".claude/hooks");
    if !crochets.is_dir() {
        return;
    }
    for entree in std::fs::read_dir(&crochets)
        .expect("crochets lisibles")
        .flatten()
    {
        let source = std::fs::read_to_string(entree.path()).unwrap_or_default();
        if !source.contains("redis-server") {
            continue;
        }
        assert!(
            source.contains("--save ''") || source.contains("--save \"\""),
            "{} lance Redis sans « --save '' » : il écrira un instantané du \
             cache à la racine du dépôt",
            entree.path().display()
        );
    }
}

/// Les deux schémas décrivent la même base.
///
/// Le dépôt en porte deux : `migrations/` pour Postgres, qui est ce que le
/// dyno exécute, et `migrations-sqlite/` pour SQLite, qui est ce que ces tests
/// exécutent. Une colonne ajoutée d'un seul côté ne se voit nulle part : la
/// suite passe au vert sur un schéma que la production n'a pas, et la première
/// requête réelle rend une erreur de base que rien, dans l'intégration
/// continue, n'annonçait.
///
/// Seuls les noms sont comparés. Les types diffèrent légitimement — `TIMESTAMP(3)`
/// d'un côté, `DATETIME` de l'autre, `BOOLEAN` contre `INTEGER` — et c'est
/// l'ORM qui les réconcilie. Ce qui ne peut pas différer, ce sont les tables et
/// leurs colonnes.
#[test]
fn les_deux_schemas_decrivent_la_meme_base() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let postgres = colonnes_des_migrations(&racine.join("migrations"));
    let sqlite = colonnes_des_migrations(&racine.join("migrations-sqlite"));

    assert!(
        postgres.len() > 15,
        "lecture des migrations Postgres suspecte : {} tables",
        postgres.len()
    );

    let mut ecarts = Vec::new();
    let mut tables: Vec<&String> = postgres.keys().chain(sqlite.keys()).collect();
    tables.sort();
    tables.dedup();

    for table in tables {
        match (postgres.get(table), sqlite.get(table)) {
            (Some(pg), Some(lite)) => {
                let absentes_de_sqlite: Vec<&String> = pg.difference(lite).collect();
                let absentes_de_postgres: Vec<&String> = lite.difference(pg).collect();
                if !absentes_de_sqlite.is_empty() {
                    ecarts.push(format!(
                        "{table} : {absentes_de_sqlite:?} manquent à SQLite — la suite \
                         ne les éprouve pas"
                    ));
                }
                if !absentes_de_postgres.is_empty() {
                    ecarts.push(format!(
                        "{table} : {absentes_de_postgres:?} manquent à Postgres — la \
                         production ne les a pas"
                    ));
                }
            }
            (Some(_), None) => ecarts.push(format!("table « {table} » absente de SQLite")),
            (None, Some(_)) => ecarts.push(format!("table « {table} » absente de Postgres")),
            (None, None) => unreachable!(),
        }
    }

    assert!(
        ecarts.is_empty(),
        "les deux schémas ont divergé :\n  {}",
        ecarts.join("\n  ")
    );
}

/// Les colonnes déclarées par un jeu de migrations, table par table.
///
/// Lit les `CREATE TABLE`, puis applique les `ADD COLUMN` et `DROP COLUMN` des
/// migrations suivantes : c'est l'état final qui compte, pas la première
/// version.
fn colonnes_des_migrations(
    dossier: &std::path::Path,
) -> std::collections::BTreeMap<String, std::collections::BTreeSet<String>> {
    let mut tables: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
        std::collections::BTreeMap::new();

    let mut fichiers: Vec<std::path::PathBuf> = std::fs::read_dir(dossier)
        .unwrap_or_else(|e| panic!("migrations lisibles en {} : {e}", dossier.display()))
        .flatten()
        .map(|e| e.path().join("migration.sql"))
        .filter(|p| p.is_file())
        .collect();
    fichiers.sort();

    for fichier in fichiers {
        let sql = std::fs::read_to_string(&fichier).expect("migration lisible");
        let mut reste = sql.as_str();

        while let Some(debut) = reste.find("CREATE TABLE") {
            let apres = &reste[debut..];
            let Some(ouvrante) = apres.find('(') else {
                break;
            };
            let Some(fin) = apres.find("\n);") else { break };
            let nom = apres[..ouvrante]
                .rsplit_once('"')
                .and_then(|(avant, _)| avant.rsplit_once('"'))
                .map(|(_, nom)| nom.to_string());
            if let Some(nom) = nom {
                let entree = tables.entry(nom).or_default();
                for ligne in apres[ouvrante + 1..fin].lines() {
                    let ligne = ligne.trim();
                    let majuscules = ligne.to_uppercase();
                    if majuscules.starts_with("CONSTRAINT")
                        || majuscules.starts_with("PRIMARY")
                        || majuscules.starts_with("FOREIGN")
                        || majuscules.starts_with("UNIQUE")
                    {
                        continue;
                    }
                    if let Some(colonne) = nom_entre_guillemets(ligne) {
                        entree.insert(colonne);
                    }
                }
            }
            reste = &apres[fin + 3..];
        }

        for ligne in sql.lines() {
            let ligne = ligne.trim();
            let majuscules = ligne.to_uppercase();
            if !majuscules.starts_with("ALTER TABLE") {
                continue;
            }
            let noms: Vec<String> = ligne
                .split('"')
                .skip(1)
                .step_by(2)
                .map(str::to_string)
                .collect();
            let (Some(table), Some(colonne)) = (noms.first(), noms.get(1)) else {
                continue;
            };
            if majuscules.contains("ADD COLUMN") {
                tables
                    .entry(table.clone())
                    .or_default()
                    .insert(colonne.clone());
            } else if majuscules.contains("DROP COLUMN")
                && let Some(entree) = tables.get_mut(table)
            {
                entree.remove(colonne);
            }
        }
    }
    tables
}

/// Le premier nom entre guillemets d'une ligne, s'il ouvre la ligne.
fn nom_entre_guillemets(ligne: &str) -> Option<String> {
    let reste = ligne.strip_prefix('"')?;
    let (nom, _) = reste.split_once('"')?;
    Some(nom.to_string())
}

/// Chaque méthode que l'application appelle existe dans WeaveKit, avec ses
/// étiquettes.
///
/// C'est le seul accord qu'aucun compilateur ne tient ici. WeaveKit se compile
/// sous Linux — `apps/ios/verification-linux/` s'en charge — mais les écrans
/// dépendent de SwiftUI, et rien hors d'un Mac ne les type. Une méthode
/// renommée, ou un paramètre re-étiqueté, laisse donc ses appelants intacts et
/// muets jusqu'à la prochaine ouverture d'Xcode.
///
/// Ce que ce test NE vérifie PAS, et il faut le dire : ni les types, ni les
/// valeurs de retour, ni rien de ce qui touche SwiftUI. Les noms et les
/// étiquettes, seulement. C'est étroit, mais jamais faux.
///
/// Les porteurs sont listés à la main, et c'est le point faible : la première
/// version en oubliait deux — `plans` et `sessionStore` —, si bien que tout
/// `modele.plans.join(...)` passait sans être regardé, et qu'une étiquette
/// renommée là ne faisait rien échouer. Un nom absent de cette table n'est pas
/// signalé, il est ignoré. L'assertion sur le nombre d'appels retrouvés est là
/// pour ça : si elle tombe, c'est qu'un porteur manque.
#[test]
fn chaque_methode_appelee_par_l_application_existe_dans_weavekit() {
    let ios = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ios");
    if !ios.is_dir() {
        eprintln!(
            "application iOS absente en {} — accord non vérifié",
            ios.display()
        );
        return;
    }

    // Le nom sous lequel l'application tient chaque objet, et le fichier qui le
    // déclare. `ModeleApplication` porte les autres : `modele.plans.join(...)`
    // se lit donc sous « plans ».
    let porteurs = [
        ("api", "WeaveKit/Sources/WeaveKit/Networking/WeaveAPI.swift"),
        ("plans", "WeaveKit/Sources/WeaveKit/Stores/PlansStore.swift"),
        (
            "sessionStore",
            "WeaveKit/Sources/WeaveKit/Stores/SessionStore.swift",
        ),
        (
            "boutique",
            "WeaveKit/Sources/WeaveKit/Stores/BoutiqueController.swift",
        ),
        (
            "activites",
            "WeaveKit/Sources/WeaveKit/Stores/ActivityController.swift",
        ),
        (
            "notifications",
            "WeaveKit/Sources/WeaveKit/Stores/NotificationsController.swift",
        ),
        ("modele", "Weave/WeaveApp.swift"),
    ];

    let mut declarees: std::collections::BTreeMap<&str, Declarations> =
        std::collections::BTreeMap::new();
    for (nom, chemin) in porteurs {
        let source = std::fs::read_to_string(ios.join(chemin))
            .unwrap_or_else(|e| panic!("{chemin} illisible : {e}"));
        let methodes = methodes_declarees(&source);
        assert!(
            !methodes.is_empty(),
            "aucune méthode lue dans {chemin} — la lecture est à revoir"
        );
        declarees.insert(nom, methodes);
    }

    let mut sources = Vec::new();
    for dossier in ["Weave", "WeaveActivity", "WeaveWatchWidgets"] {
        empiler_les_fichiers_swift(&ios.join(dossier), &mut sources);
    }

    let mut vus = 0usize;
    let mut ecarts = Vec::new();
    for (fichier, source) in &sources {
        for (porteur, connues) in &declarees {
            for (methode, etiquettes) in appels_sur(source, porteur) {
                vus += 1;
                let Some(signatures) = connues.get(&methode) else {
                    ecarts.push(format!(
                        "{porteur}.{methode}() — appelé dans {fichier}, jamais déclaré"
                    ));
                    continue;
                };
                // Une valeur par défaut autorise à omettre son argument : les
                // étiquettes de l'appel doivent former une sous-suite ORDONNÉE
                // de celles d'une des signatures.
                if !signatures.iter().any(|sig| sous_suite(&etiquettes, sig)) {
                    ecarts.push(format!(
                        "{porteur}.{methode}({}) dans {fichier} — déclaré {signatures:?}",
                        etiquettes.join(", ")
                    ));
                }
            }
        }
    }

    assert!(
        vus > 70,
        "seulement {vus} appels retrouvés : un porteur manque à la table"
    );
    assert!(
        ecarts.is_empty(),
        "l'application n'appelle pas WeaveKit comme WeaveKit se déclare :\n  {}",
        ecarts.join("\n  ")
    );
}

/// Les signatures déclarées, par nom de méthode : une entrée par surcharge.
type Declarations = std::collections::BTreeMap<String, Vec<Vec<String>>>;

/// `petite` est-elle une sous-suite ordonnée de `grande` ?
fn sous_suite(petite: &[String], grande: &[String]) -> bool {
    let mut reste = grande.iter();
    petite
        .iter()
        .all(|attendu| reste.any(|candidat| candidat == attendu))
}

/// Le texte entre la parenthèse ouvrante en `ouvrante` et sa fermante.
fn entre_parentheses(source: &str, ouvrante: usize) -> Option<&str> {
    let octets = source.as_bytes();
    let mut profondeur = 0usize;
    let mut dans_chaine = false;
    let mut echappe = false;
    for (position, octet) in octets.iter().enumerate().skip(ouvrante) {
        let c = *octet as char;
        if dans_chaine {
            if echappe {
                echappe = false;
            } else if c == '\\' {
                echappe = true;
            } else if c == '"' {
                dans_chaine = false;
            }
            continue;
        }
        match c {
            '"' => dans_chaine = true,
            '(' => profondeur += 1,
            ')' => {
                profondeur -= 1;
                if profondeur == 0 {
                    return source.get(ouvrante + 1..position);
                }
            }
            _ => {}
        }
    }
    None
}

/// Découpe une liste d'arguments aux virgules de premier niveau.
fn arguments(texte: &str) -> Vec<String> {
    let mut morceaux = Vec::new();
    let mut courant = String::new();
    let mut profondeur = 0i32;
    let mut dans_chaine = false;
    let mut echappe = false;
    for c in texte.chars() {
        if dans_chaine {
            courant.push(c);
            if echappe {
                echappe = false;
            } else if c == '\\' {
                echappe = true;
            } else if c == '"' {
                dans_chaine = false;
            }
            continue;
        }
        match c {
            '"' => dans_chaine = true,
            '(' | '[' | '{' | '<' => profondeur += 1,
            ')' | ']' | '}' | '>' => profondeur -= 1,
            _ => {}
        }
        if c == ',' && profondeur == 0 {
            morceaux.push(std::mem::take(&mut courant));
        } else {
            courant.push(c);
        }
    }
    if !courant.trim().is_empty() {
        morceaux.push(courant);
    }
    morceaux
}

/// L'étiquette d'un argument au site d'APPEL : « nom: valeur », ou « _ ».
fn etiquette_d_appel(argument: &str) -> String {
    let argument = argument.trim();
    let Some((avant, apres)) = argument.split_once(':') else {
        return "_".to_string();
    };
    // « Plan.self » ou « a ? b : c » ne sont pas des étiquettes.
    if apres.starts_with(':') || avant.contains(' ') || avant.contains('?') {
        return "_".to_string();
    }
    if !avant.is_empty() && avant.chars().all(|c| c.is_alphanumeric() || c == '_') {
        avant.to_string()
    } else {
        "_".to_string()
    }
}

/// L'étiquette d'un paramètre DÉCLARÉ : « étiquette nom: Type », ou « nom: Type ».
fn etiquette_declaree(parametre: &str) -> String {
    let parametre = parametre.trim();
    let Some((avant, _)) = parametre.split_once(':') else {
        return "_".to_string();
    };
    let mots: Vec<&str> = avant.split_whitespace().collect();
    match mots.first() {
        Some(&"_") | None => "_".to_string(),
        Some(premier) => (*premier).to_string(),
    }
}

/// Les signatures déclarées dans une source Swift.
fn methodes_declarees(source: &str) -> Declarations {
    let mut declarations: Declarations = std::collections::BTreeMap::new();
    for (position, _) in source.match_indices("func ") {
        let reste = &source[position + 5..];
        let nom: String = reste
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if nom.is_empty() {
            continue;
        }
        let Some(decalage) = reste[nom.len()..].find('(') else {
            continue;
        };
        let ouvrante = position + 5 + nom.len() + decalage;
        let Some(parametres) = entre_parentheses(source, ouvrante) else {
            continue;
        };
        declarations.entry(nom).or_default().push(
            arguments(parametres)
                .iter()
                .map(|p| etiquette_declaree(p))
                .collect(),
        );
    }
    declarations
}

/// Les appels sur `porteur`, avec les étiquettes passées.
fn appels_sur(source: &str, porteur: &str) -> Vec<(String, Vec<String>)> {
    let marque = format!("{porteur}.");
    let mut appels = Vec::new();
    for (position, _) in source.match_indices(&marque) {
        // Le porteur doit ouvrir le mot : « monModele. » n'est pas « modele. ».
        //
        // Le point, lui, est accepté : l'application atteint le client réseau
        // par `modele.api.`, et c'est la forme la plus fréquente.
        if position > 0 {
            let avant = source[..position].chars().next_back().unwrap_or(' ');
            if avant.is_alphanumeric() || avant == '_' {
                continue;
            }
        }
        let reste = &source[position + marque.len()..];
        let nom: String = reste
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if nom.is_empty() {
            continue;
        }
        // Seuls les appels comptent : une propriété n'a pas de parenthèse.
        let apres = reste[nom.len()..].trim_start();
        if !apres.starts_with('(') {
            continue;
        }
        let ouvrante =
            position + marque.len() + nom.len() + (reste[nom.len()..].len() - apres.len());
        let Some(arguments_bruts) = entre_parentheses(source, ouvrante) else {
            continue;
        };
        appels.push((
            nom,
            arguments(arguments_bruts)
                .iter()
                .map(|a| etiquette_d_appel(a))
                .collect(),
        ));
    }
    appels
}

/// Empile les sources Swift d'un répertoire, avec leur nom de fichier.
fn empiler_les_fichiers_swift(repertoire: &std::path::Path, sortie: &mut Vec<(String, String)>) {
    let Ok(entrees) = std::fs::read_dir(repertoire) else {
        return;
    };
    for entree in entrees.flatten() {
        let chemin = entree.path();
        if chemin.is_dir() {
            empiler_les_fichiers_swift(&chemin, sortie);
        } else if chemin.extension().is_some_and(|e| e == "swift")
            && let Ok(source) = std::fs::read_to_string(&chemin)
        {
            let nom = chemin
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string();
            sortie.push((nom, source));
        }
    }
}
