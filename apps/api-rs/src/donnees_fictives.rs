//! Des comptes et une activité fictifs, pour développer et tester sur un fil
//! rempli.
//!
//! `weave-api seed` crée seize personnes, un plan chacune pour les jours qui
//! viennent, des demandes entre elles et quelques conversations. `weave-api
//! seed effacer` les retire, avec tout ce qu'elles ont produit.
//!
//! ## Ce que ce module ne doit jamais faire
//!
//! Montrer un faux profil à une vraie personne. Sur une application de
//! rencontre, c'est la tromperie même que le produit refuse — on ne gonfle pas
//! un fil pour donner l'impression qu'il est vivant. D'où trois gardes, toutes
//! levées AVANT la première écriture :
//!
//! 1. **En production, il faut le demander** : `WEAVE_DONNEES_FICTIVES=oui`
//!    sur l'application Heroku, et seulement avant le lancement.
//! 2. **Jamais avec l'App Store en production** : `APPSTORE_ENVIRONMENT`
//!    à `production` veut dire que l'application est publiée.
//! 3. **Jamais à côté d'un vrai compte** : la base ne doit contenir que des
//!    comptes fictifs et ceux de l'équipe (`COMPTES_GRANDTOUR`). Un seul
//!    inconnu, et la commande refuse. C'est la garde qui compte : les deux
//!    autres se règlent par une variable, celle-ci regarde qui verrait le fil.
//!
//! Et au lancement, le serveur les retire seul : voir `retirer_si_publiee`.
//!
//! Effacer, en revanche, n'a pas de garde : retirer des comptes fictifs ne
//! trompe personne.
//!
//! ## Comment on les reconnaît
//!
//! Par leur adresse, sous `fictif.weave.invalid`. Le domaine `.invalid` est
//! réservé (RFC 2606) : aucun code de connexion ne peut y partir, et personne
//! ne peut ouvrir de session sur ces comptes. Leur présentation le dit aussi,
//! en clair, pour qui tomberait dessus dans l'application.

use crate::crypto::hash_email;
use crate::entities::{
    accounts, conversations, join_requests, messages, plans, preferences, profiles, subscriptions,
};
use crate::env::Env;
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, Set, TransactionTrait,
};

/// Le domaine de toutes les adresses fictives.
pub const DOMAINE: &str = "fictif.weave.invalid";

/// La variable qui autorise la commande en production.
pub const VARIABLE: &str = "WEAVE_DONNEES_FICTIVES";

const MENTION: &str = "Compte fictif — données de test.";

const AIDE: &str = "\
weave-api seed — comptes et activité fictifs, pour tester

  seed [ville]        crée 16 comptes, leurs plans, des demandes et des
                      conversations autour d'une ville (Paris par défaut)
  seed <e-mail>       pareil, autour du compte de l'équipe qui porte cette
                      adresse — qui reçoit en plus trois demandes sur son
                      premier plan ouvert
  seed effacer        retire tous les comptes fictifs et ce qu'ils ont produit

Relancer `seed` efface d'abord les fictifs précédents : la commande ne cumule
jamais.

En production (Heroku), elle exige WEAVE_DONNEES_FICTIVES=oui, refuse si
APPSTORE_ENVIRONMENT vaut production, et refuse si la base contient un seul
compte qui ne soit ni fictif ni dans COMPTES_GRANDTOUR. Ce sont des comptes
de test : ils ne doivent jamais apparaître dans le fil d'une vraie personne.";

/// Les villes connues, et leur centre arrondi au centième.
const VILLES: [(&str, f64, f64); 10] = [
    ("Paris", 48.86, 2.35),
    ("Lyon", 45.76, 4.84),
    ("Marseille", 43.30, 5.37),
    ("Bordeaux", 44.84, -0.58),
    ("Lille", 50.63, 3.06),
    ("Toulouse", 43.60, 1.44),
    ("Nantes", 47.22, -1.55),
    ("Nice", 43.70, 7.27),
    ("Strasbourg", 48.58, 7.75),
    ("Montpellier", 43.61, 3.88),
];

struct Personne {
    prenom: &'static str,
    genre: &'static str,
    annee: i32,
    bio: &'static str,
    categorie: &'static str,
    titre: &'static str,
    note: &'static str,
    /// Dans combien de jours, et à quelle heure (heure de Paris).
    jour: i64,
    heure: u32,
    capacite: i32,
}

const PERSONNES: [Personne; 16] = [
    Personne {
        prenom: "Léa",
        genre: "femme",
        annee: 1996,
        bio: "Grimpe le jeudi, lit le dimanche.",
        categorie: "sport",
        titre: "Bloc en salle, niveau débutant",
        note: "Je prête des chaussons, pointure 39.",
        jour: 1,
        heure: 19,
        capacite: 2,
    },
    Personne {
        prenom: "Thomas",
        genre: "homme",
        annee: 1993,
        bio: "Batteur à mes heures.",
        categorie: "musique",
        titre: "Concert jazz au sous-sol",
        note: "Entrée libre, on part après le premier set si c'est nul.",
        jour: 2,
        heure: 21,
        capacite: 1,
    },
    Personne {
        prenom: "Camille",
        genre: "femme",
        annee: 1998,
        bio: "Toujours partante pour une expo.",
        categorie: "culture",
        titre: "Expo photo, dernière semaine",
        note: "On en parle autour d'un café ensuite.",
        jour: 3,
        heure: 15,
        capacite: 3,
    },
    Personne {
        prenom: "Hugo",
        genre: "homme",
        annee: 1991,
        bio: "Cuisine du marché, beaucoup trop de piment.",
        categorie: "repas",
        titre: "Brunch dans le nouveau resto",
        note: "J'ai réservé pour deux, on peut pousser à trois.",
        jour: 4,
        heure: 11,
        capacite: 2,
    },
    Personne {
        prenom: "Emma",
        genre: "femme",
        annee: 1995,
        bio: "Je marche vite, je parle encore plus vite.",
        categorie: "balade",
        titre: "Balade le long du canal",
        note: "Une heure trente, rythme tranquille.",
        jour: 1,
        heure: 18,
        capacite: 3,
    },
    Personne {
        prenom: "Nour",
        genre: "non_binaire",
        annee: 1997,
        bio: "Jeux de société, surtout les longs.",
        categorie: "jeux",
        titre: "Soirée jeux au bar ludique",
        note: "Je ramène Cascadia, vous ramenez votre bonne humeur.",
        jour: 2,
        heure: 20,
        capacite: 4,
    },
    Personne {
        prenom: "Julien",
        genre: "homme",
        annee: 1990,
        bio: "Coureur du dimanche, même le samedi.",
        categorie: "sport",
        titre: "Footing de dix kilomètres",
        note: "Allure 5'30, on attend tout le monde.",
        jour: 5,
        heure: 9,
        capacite: 3,
    },
    Personne {
        prenom: "Inès",
        genre: "femme",
        annee: 1999,
        bio: "Bénévole à la banque alimentaire.",
        categorie: "benevolat",
        titre: "Collecte au supermarché du quartier",
        note: "Deux heures, on tient le stand à deux.",
        jour: 3,
        heure: 10,
        capacite: 2,
    },
    Personne {
        prenom: "Malik",
        genre: "homme",
        annee: 1994,
        bio: "Cinéphile, popcorn salé.",
        categorie: "culture",
        titre: "Ciné en VO, séance de 20 h",
        note: "Je n'ai pas encore choisi le film, on décide ensemble.",
        jour: 2,
        heure: 20,
        capacite: 1,
    },
    Personne {
        prenom: "Chloé",
        genre: "femme",
        annee: 1992,
        bio: "Danse le week-end, télétravaille la semaine.",
        categorie: "sortie",
        titre: "Afterwork sur une terrasse",
        note: "Juste un verre, ou deux.",
        jour: 1,
        heure: 19,
        capacite: 3,
    },
    Personne {
        prenom: "Sacha",
        genre: "autre",
        annee: 1996,
        bio: "Je collectionne les vinyles et les cafés.",
        categorie: "musique",
        titre: "Disquaire puis café",
        note: "Une heure à fouiller les bacs, une heure à en parler.",
        jour: 6,
        heure: 14,
        capacite: 2,
    },
    Personne {
        prenom: "Antoine",
        genre: "homme",
        annee: 1989,
        bio: "Randonneur, photographe amateur.",
        categorie: "balade",
        titre: "Petite rando hors de la ville",
        note: "Départ en train, retour avant 18 h.",
        jour: 5,
        heure: 8,
        capacite: 4,
    },
    Personne {
        prenom: "Zoé",
        genre: "femme",
        annee: 2000,
        bio: "Étudiante en archi, toujours un carnet.",
        categorie: "culture",
        titre: "Croquis au musée, gratuit le soir",
        note: "Pas besoin de savoir dessiner.",
        jour: 4,
        heure: 18,
        capacite: 3,
    },
    Personne {
        prenom: "Yanis",
        genre: "homme",
        annee: 1997,
        bio: "Padel, pétanque, et tout ce qui se joue.",
        categorie: "sport",
        titre: "Padel en double, il manque un joueur",
        note: "Terrain réservé, raquette fournie.",
        jour: 3,
        heure: 18,
        capacite: 1,
    },
    Personne {
        prenom: "Lina",
        genre: "femme",
        annee: 1994,
        bio: "Je teste toutes les cantines du quartier.",
        categorie: "repas",
        titre: "Dîner libanais, table pour quatre",
        note: "Mezzés à partager, addition aussi.",
        jour: 6,
        heure: 20,
        capacite: 3,
    },
    Personne {
        prenom: "Robin",
        genre: "non_binaire",
        annee: 1993,
        bio: "Quiz, blind tests, et mauvaise foi.",
        categorie: "jeux",
        titre: "Quiz du mardi au pub",
        note: "On a perdu trois fois, on vise la quatrième place.",
        jour: 4,
        heure: 20,
        capacite: 3,
    },
];

const MOTS_DE_DEMANDE: [&str; 4] = [
    "Ça me tente vraiment, je suis dispo !",
    "J'en rêvais depuis des semaines, je peux venir ?",
    "Première fois pour moi, mais motivé·e.",
    "Je connais l'endroit, je peux même guider.",
];

/// Ce que la commande a fait.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Bilan {
    pub comptes: usize,
    pub plans: usize,
    pub demandes: usize,
    pub conversations: usize,
    pub messages: usize,
    /// Les demandes posées sur le plan du compte de l'équipe, si on l'a visé.
    pub demandes_pour_l_equipe: usize,
}

/// Les conditions dans lesquelles la commande s'exécute.
///
/// Séparées de `Env` pour qu'un test puisse les poser une à une.
#[derive(Debug, Clone)]
pub struct Garde {
    pub production: bool,
    pub demandee: bool,
    pub app_store_en_production: bool,
    pub equipe: Vec<String>,
}

impl Garde {
    pub fn depuis(config: &Env) -> Self {
        Self {
            production: config.is_production(),
            demandee: std::env::var(VARIABLE)
                .map(|v| matches!(v.trim().to_lowercase().as_str(), "oui" | "true" | "1"))
                .unwrap_or(false),
            app_store_en_production: config.app_store.environnement == "production",
            equipe: config.auth.comptes_offerts.clone(),
        }
    }
}

#[cfg(test)]
fn est_fictive(email: &str) -> bool {
    email.ends_with(&format!("@{DOMAINE}"))
}

/// Lève les trois gardes, ou dit laquelle tient.
pub async fn verifier<C: ConnectionTrait>(db: &C, garde: &Garde) -> anyhow::Result<()> {
    if garde.production && !garde.demandee {
        anyhow::bail!(
            "refusé : en production, posez d'abord {VARIABLE}=oui — et seulement avant \
             le lancement"
        );
    }
    if garde.app_store_en_production {
        anyhow::bail!(
            "refusé : APPSTORE_ENVIRONMENT vaut « production », l'application est \
             publiée. Des comptes fictifs n'ont rien à faire là"
        );
    }

    let empreintes_equipe: Vec<String> = garde.equipe.iter().map(|e| hash_email(e)).collect();
    let inconnus = accounts::Entity::find()
        .filter(accounts::Column::Email.not_like(format!("%@{DOMAINE}")))
        .filter(accounts::Column::EmailHash.is_not_in(empreintes_equipe))
        .count(db)
        .await?;
    if inconnus > 0 {
        anyhow::bail!(
            "refusé : la base contient {inconnus} compte(s) qui ne sont ni fictifs ni dans \
             COMPTES_GRANDTOUR. Ces personnes verraient des profils fictifs dans leur fil. \
             Utilisez une base de test, ou ajoutez les adresses de l'équipe à \
             COMPTES_GRANDTOUR"
        );
    }
    Ok(())
}

/// Retire tous les comptes fictifs. Les clés étrangères en cascade emportent
/// fiches, préférences, plans, demandes, conversations et messages.
pub async fn effacer<C: ConnectionTrait>(db: &C) -> anyhow::Result<u64> {
    let resultat = accounts::Entity::delete_many()
        .filter(accounts::Column::Email.like(format!("%@{DOMAINE}")))
        .exec(db)
        .await?;
    Ok(resultat.rows_affected)
}

/// Le filet du lancement : dès que l'App Store passe en production, le
/// serveur retire les comptes fictifs à son démarrage.
///
/// Ils se sèment sur l'application du public tant qu'elle n'a pas de vrais
/// inscrits — c'est la seule que l'application iOS sait joindre. Oublier de
/// les effacer avant la publication les montrerait aux premiers venus : ce
/// filet rend l'oubli impossible.
pub async fn retirer_si_publiee<C: ConnectionTrait>(
    db: &C,
    app_store_en_production: bool,
) -> anyhow::Result<u64> {
    if !app_store_en_production {
        return Ok(0);
    }
    effacer(db).await
}

/// Où poser les comptes : une ville connue, ou la fiche d'un compte de
/// l'équipe.
pub enum Autour {
    Ville(String),
    Compte(String),
}

impl Autour {
    pub fn depuis(argument: Option<&str>) -> Self {
        match argument {
            Some(a) if a.contains('@') => Autour::Compte(a.to_string()),
            Some(a) => Autour::Ville(a.to_string()),
            None => Autour::Ville("Paris".to_string()),
        }
    }
}

struct Lieu {
    ville: String,
    lat: f64,
    lon: f64,
    /// Le compte de l'équipe visé, s'il y en a un.
    compte: Option<String>,
}

async fn situer<C: ConnectionTrait>(
    db: &C,
    autour: &Autour,
    garde: &Garde,
) -> anyhow::Result<Lieu> {
    match autour {
        Autour::Ville(nom) => {
            let Some((ville, lat, lon)) =
                VILLES.iter().find(|(v, _, _)| v.eq_ignore_ascii_case(nom))
            else {
                let connues: Vec<&str> = VILLES.iter().map(|(v, _, _)| *v).collect();
                anyhow::bail!("ville inconnue « {nom} ». Connues : {}", connues.join(", "));
            };
            Ok(Lieu {
                ville: (*ville).to_string(),
                lat: *lat,
                lon: *lon,
                compte: None,
            })
        }
        Autour::Compte(email) => {
            // On ne vise qu'un compte de l'équipe : même en développement, poser
            // des demandes fictives chez quelqu'un d'autre n'a pas de sens.
            let empreinte = hash_email(email);
            if garde.production && !garde.equipe.iter().any(|e| hash_email(e) == empreinte) {
                anyhow::bail!("« {email} » n'est pas dans COMPTES_GRANDTOUR");
            }
            let compte = accounts::Entity::find()
                .filter(accounts::Column::EmailHash.eq(empreinte))
                .one(db)
                .await?
                .ok_or_else(|| anyhow::anyhow!("aucun compte ne porte l'adresse « {email} »"))?;
            let fiche = profiles::Entity::find()
                .filter(profiles::Column::AccountId.eq(compte.id.as_str()))
                .one(db)
                .await?
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "« {email} » n'a pas encore de fiche : terminez l'inscription dans \
                         l'application, ou donnez une ville"
                    )
                })?;
            Ok(Lieu {
                ville: fiche.city,
                lat: fiche.lat_rounded,
                lon: fiche.lon_rounded,
                compte: Some(compte.id),
            })
        }
    }
}

/// L'instant d'un plan : dans `jour` jours, à `heure` heure de Paris.
fn debut(maintenant: DateTime<Utc>, jour: i64, heure: u32) -> DateTime<Utc> {
    let paris = chrono_tz::Europe::Paris;
    let date = (maintenant.with_timezone(&paris) + Duration::days(jour)).date_naive();
    let local = date.and_hms_opt(heure, 0, 0).expect("heure valide");
    paris
        .from_local_datetime(&local)
        .earliest()
        .map(|d| d.with_timezone(&Utc))
        // Un changement d'heure qui avale cette heure-là : une heure plus tard.
        .unwrap_or_else(|| maintenant + Duration::days(jour))
}

/// Crée les comptes fictifs et leur activité, après avoir levé les gardes et
/// effacé les fictifs précédents.
pub async fn semer(
    db: &DatabaseConnection,
    garde: &Garde,
    autour: &Autour,
) -> anyhow::Result<Bilan> {
    verifier(db, garde).await?;
    let lieu = situer(db, autour, garde).await?;

    let transaction = db.begin().await?;
    effacer(&transaction).await?;

    let maintenant = Utc::now();
    let instant = maintenant.naive_utc();
    let mut bilan = Bilan::default();
    let mut comptes_ids = Vec::new();
    let mut plans_crees: Vec<plans::Model> = Vec::new();

    for (i, p) in PERSONNES.iter().enumerate() {
        let pseudo = format!("fictif-{}", i + 1);
        let email = format!("{pseudo}@{DOMAINE}");
        let id = cuid2::create_id();

        accounts::ActiveModel {
            id: Set(id.clone()),
            email: Set(email.clone()),
            email_hash: Set(hash_email(&email)),
            handle: Set(pseudo),
            display_name: Set(p.prenom.to_string()),
            birth_date: Set(
                NaiveDate::from_ymd_opt(p.annee, 1 + (i as u32 % 12), 1 + i as u32)
                    .expect("date valide")
                    .and_hms_opt(0, 0, 0)
                    .expect("heure valide"),
            ),
            status: Set("active".to_string()),
            timezone: Set("Europe/Paris".to_string()),
            locale: Set("fr-FR".to_string()),
            verified: Set(i % 3 == 0),
            last_seen_at: Set(Some(instant - Duration::minutes(7 * i as i64))),
            deletion_requested_at: Set(None),
            last_bilan_at: Set(None),
            created_at: Set(instant),
            updated_at: Set(instant),
        }
        .insert(&transaction)
        .await?;

        // Dispersés à quelques kilomètres du centre, sur une grille : la
        // position reste arrondie au centième, comme celles des vraies fiches.
        let lat = ((lieu.lat + ((i % 5) as f64 - 2.0) * 0.01) * 100.0).round() / 100.0;
        let lon = ((lieu.lon + ((i / 5 % 5) as f64 - 2.0) * 0.01) * 100.0).round() / 100.0;

        profiles::ActiveModel {
            id: Set(cuid2::create_id()),
            account_id: Set(id.clone()),
            city: Set(lieu.ville.clone()),
            lat_rounded: Set(lat),
            lon_rounded: Set(lon),
            gender: Set(p.genre.to_string()),
            bio: Set(format!("{} {MENTION}", p.bio)),
            photo_key: Set(None),
            photo_reviewed_at: Set(None),
            created_at: Set(instant),
            updated_at: Set(instant),
        }
        .insert(&transaction)
        .await?;

        preferences::ActiveModel {
            id: Set(cuid2::create_id()),
            account_id: Set(id.clone()),
            updated_at: Set(instant),
            ..Default::default()
        }
        .insert(&transaction)
        .await?;

        subscriptions::ActiveModel {
            id: Set(cuid2::create_id()),
            account_id: Set(id.clone()),
            tier: Set("depart".to_string()),
            created_at: Set(instant),
            updated_at: Set(instant),
            ..Default::default()
        }
        .insert(&transaction)
        .await?;

        let plan = plans::ActiveModel {
            id: Set(cuid2::create_id()),
            author_id: Set(id.clone()),
            title: Set(p.titre.to_string()),
            note: Set(p.note.to_string()),
            category: Set(p.categorie.to_string()),
            starts_at: Set(debut(maintenant, p.jour, p.heure).naive_utc()),
            city: Set(lieu.ville.clone()),
            lat_rounded: Set(lat),
            lon_rounded: Set(lon),
            capacity: Set(p.capacite),
            state: Set("ouvert".to_string()),
            created_at: Set(instant - Duration::hours(i as i64 + 1)),
            updated_at: Set(instant),
            ..Default::default()
        }
        .insert(&transaction)
        .await?;

        comptes_ids.push(id);
        plans_crees.push(plan);
        bilan.comptes += 1;
        bilan.plans += 1;
    }

    // Chacun demande à venir au plan du suivant. Un sur trois est accepté,
    // quand le plan a la place de rester ouvert après — et la conversation
    // s'ouvre, avec trois messages.
    let n = comptes_ids.len();
    for i in 0..n {
        let invite = &comptes_ids[i];
        let plan = &plans_crees[(i + 1) % n];
        let acceptee = i % 3 == 0 && plan.capacity > 1;
        let envoyee = instant - Duration::minutes(30 + 11 * i as i64);

        let demande = join_requests::ActiveModel {
            id: Set(cuid2::create_id()),
            plan_id: Set(plan.id.clone()),
            author_id: Set(invite.clone()),
            message: Set(MOTS_DE_DEMANDE[i % MOTS_DE_DEMANDE.len()].to_string()),
            state: Set(if acceptee { "acceptee" } else { "envoyee" }.to_string()),
            sent_at: Set(envoyee),
            decided_at: Set(acceptee.then_some(envoyee + Duration::minutes(5))),
        }
        .insert(&transaction)
        .await?;
        bilan.demandes += 1;

        if !acceptee {
            continue;
        }

        let conversation = conversations::ActiveModel {
            id: Set(cuid2::create_id()),
            plan_id: Set(plan.id.clone()),
            request_id: Set(demande.id.clone()),
            host_id: Set(plan.author_id.clone()),
            guest_id: Set(invite.clone()),
            opened_at: Set(envoyee + Duration::minutes(5)),
            ..Default::default()
        }
        .insert(&transaction)
        .await?;
        bilan.conversations += 1;

        let echange = [
            (
                &plan.author_id,
                "Avec plaisir ! On se retrouve devant l'entrée ?",
            ),
            (invite, "Parfait, j'y serai cinq minutes avant."),
            (&plan.author_id, "Top, à tout à l'heure 🙂"),
        ];
        let mut dernier = envoyee;
        for (k, (auteur, texte)) in echange.iter().enumerate() {
            dernier = envoyee + Duration::minutes(6 + k as i64 * 2);
            messages::ActiveModel {
                id: Set(cuid2::create_id()),
                conversation_id: Set(conversation.id.clone()),
                author_id: Set((*auteur).clone()),
                body: Set((*texte).to_string()),
                sent_at: Set(dernier),
                ..Default::default()
            }
            .insert(&transaction)
            .await?;
            bilan.messages += 1;
        }
        let mut maj: conversations::ActiveModel = conversation.into();
        maj.last_message_at = Set(Some(dernier));
        maj.update(&transaction).await?;
    }

    // Le compte de l'équipe reçoit trois demandes sur son premier plan
    // ouvert : de quoi éprouver l'écran des demandes reçues.
    if let Some(compte_equipe) = &lieu.compte {
        let premier = plans::Entity::find()
            .filter(plans::Column::AuthorId.eq(compte_equipe.as_str()))
            .filter(plans::Column::State.eq("ouvert"))
            .one(&transaction)
            .await?;
        if let Some(plan) = premier {
            for (k, invite) in comptes_ids.iter().take(3).enumerate() {
                join_requests::ActiveModel {
                    id: Set(cuid2::create_id()),
                    plan_id: Set(plan.id.clone()),
                    author_id: Set(invite.clone()),
                    message: Set(MOTS_DE_DEMANDE[k].to_string()),
                    state: Set("envoyee".to_string()),
                    sent_at: Set(instant - Duration::minutes(3 + 4 * k as i64)),
                    decided_at: Set(None),
                }
                .insert(&transaction)
                .await?;
                bilan.demandes_pour_l_equipe += 1;
            }
        }
    }

    transaction.commit().await?;
    Ok(bilan)
}

/// `weave-api seed …`
pub async fn executer(
    db: &DatabaseConnection,
    config: &Env,
    arguments: &[String],
) -> anyhow::Result<()> {
    match arguments.first().map(String::as_str) {
        Some("aide" | "--help" | "-h") => {
            println!("{AIDE}");
            Ok(())
        }
        Some("effacer") => {
            let n = effacer(db).await?;
            println!("  • comptes fictifs effacés : {n}");
            Ok(())
        }
        argument => {
            let garde = Garde::depuis(config);
            let bilan = semer(db, &garde, &Autour::depuis(argument)).await?;
            println!("  • comptes        : {}", bilan.comptes);
            println!("  • plans          : {}", bilan.plans);
            println!("  • demandes       : {}", bilan.demandes);
            println!("  • conversations  : {}", bilan.conversations);
            println!("  • messages       : {}", bilan.messages);
            if bilan.demandes_pour_l_equipe > 0 {
                println!(
                    "  • demandes reçues sur votre plan : {}",
                    bilan.demandes_pour_l_equipe
                );
            } else if matches!(Autour::depuis(argument), Autour::Compte(_)) {
                println!(
                    "  • aucune demande pour vous : publiez un plan dans l'application, \
                     puis relancez"
                );
            }
            println!("\nLe fil se met à jour en cinq minutes au plus (cache).");
            println!("Pour tout retirer : weave-api seed effacer");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::base_de_test;

    fn garde_de_dev() -> Garde {
        Garde {
            production: false,
            demandee: false,
            app_store_en_production: false,
            equipe: vec!["max@exemple.fr".to_string()],
        }
    }

    async fn vrai_compte(db: &DatabaseConnection, email: &str) -> String {
        let id = cuid2::create_id();
        let instant = Utc::now().naive_utc();
        accounts::ActiveModel {
            id: Set(id.clone()),
            email: Set(email.to_string()),
            email_hash: Set(hash_email(email)),
            handle: Set(id.clone()),
            display_name: Set("Vraie personne".to_string()),
            birth_date: Set(NaiveDate::from_ymd_opt(1995, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()),
            status: Set("active".to_string()),
            timezone: Set("Europe/Paris".to_string()),
            locale: Set("fr-FR".to_string()),
            verified: Set(false),
            last_seen_at: Set(None),
            deletion_requested_at: Set(None),
            last_bilan_at: Set(None),
            created_at: Set(instant),
            updated_at: Set(instant),
        }
        .insert(db)
        .await
        .expect("compte inséré");
        id
    }

    async fn fiche(db: &DatabaseConnection, compte: &str) {
        let instant = Utc::now().naive_utc();
        profiles::ActiveModel {
            id: Set(cuid2::create_id()),
            account_id: Set(compte.to_string()),
            city: Set("Lyon".to_string()),
            lat_rounded: Set(45.76),
            lon_rounded: Set(4.84),
            gender: Set("homme".to_string()),
            bio: Set(String::new()),
            photo_key: Set(None),
            photo_reviewed_at: Set(None),
            created_at: Set(instant),
            updated_at: Set(instant),
        }
        .insert(db)
        .await
        .expect("fiche insérée");
    }

    async fn nombre_de_comptes(db: &DatabaseConnection) -> u64 {
        accounts::Entity::find().count(db).await.unwrap()
    }

    #[tokio::test]
    async fn seme_un_fil_complet_et_reconnaissable() {
        let db = base_de_test().await;
        let bilan = semer(&db, &garde_de_dev(), &Autour::depuis(None))
            .await
            .unwrap();

        assert_eq!(bilan.comptes, 16);
        assert_eq!(bilan.plans, 16);
        assert_eq!(bilan.demandes, 16);
        assert!(bilan.conversations > 0 && bilan.messages == bilan.conversations * 3);

        for compte in accounts::Entity::find().all(&db).await.unwrap() {
            assert!(
                est_fictive(&compte.email),
                "{} n'est pas fictive",
                compte.email
            );
        }
        for f in profiles::Entity::find().all(&db).await.unwrap() {
            assert!(
                f.bio.contains(MENTION),
                "la fiche ne dit pas qu'elle est fictive"
            );
            assert_eq!(f.city, "Paris");
        }
        for p in plans::Entity::find().all(&db).await.unwrap() {
            assert!(
                p.starts_at > Utc::now().naive_utc(),
                "un plan fictif est déjà passé"
            );
            assert!(crate::routes::plans::CATEGORIES.contains(&p.category.as_str()));
        }
    }

    #[tokio::test]
    async fn relancer_ne_cumule_pas() {
        let db = base_de_test().await;
        semer(&db, &garde_de_dev(), &Autour::depuis(None))
            .await
            .unwrap();
        semer(&db, &garde_de_dev(), &Autour::depuis(Some("lyon")))
            .await
            .unwrap();
        assert_eq!(nombre_de_comptes(&db).await, 16);
    }

    #[tokio::test]
    async fn effacer_emporte_tout_et_epargne_l_equipe() {
        let db = base_de_test().await;
        let equipe = vrai_compte(&db, "max@exemple.fr").await;
        semer(&db, &garde_de_dev(), &Autour::depuis(None))
            .await
            .unwrap();

        assert_eq!(effacer(&db).await.unwrap(), 16);
        assert_eq!(nombre_de_comptes(&db).await, 1);
        assert!(
            accounts::Entity::find_by_id(equipe)
                .one(&db)
                .await
                .unwrap()
                .is_some()
        );
        assert_eq!(plans::Entity::find().count(&db).await.unwrap(), 0);
        assert_eq!(join_requests::Entity::find().count(&db).await.unwrap(), 0);
        assert_eq!(messages::Entity::find().count(&db).await.unwrap(), 0);
    }

    /// La garde qui compte : un seul vrai compte, et rien n'est écrit.
    #[tokio::test]
    async fn refuse_a_cote_d_une_vraie_personne() {
        let db = base_de_test().await;
        vrai_compte(&db, "inconnue@exemple.fr").await;

        let erreur = semer(&db, &garde_de_dev(), &Autour::depuis(None))
            .await
            .unwrap_err();
        assert!(
            erreur.to_string().contains("ni fictifs ni dans"),
            "{erreur}"
        );
        assert_eq!(
            nombre_de_comptes(&db).await,
            1,
            "la commande a écrit malgré le refus"
        );
    }

    #[tokio::test]
    async fn en_production_il_faut_le_demander() {
        let db = base_de_test().await;
        let mut garde = garde_de_dev();
        garde.production = true;
        assert!(semer(&db, &garde, &Autour::depuis(None)).await.is_err());

        garde.demandee = true;
        assert!(semer(&db, &garde, &Autour::depuis(None)).await.is_ok());
    }

    #[tokio::test]
    async fn jamais_avec_l_app_store_en_production() {
        let db = base_de_test().await;
        let mut garde = garde_de_dev();
        garde.production = true;
        garde.demandee = true;
        garde.app_store_en_production = true;
        assert!(semer(&db, &garde, &Autour::depuis(None)).await.is_err());
        assert_eq!(nombre_de_comptes(&db).await, 0);
    }

    #[tokio::test]
    async fn autour_d_un_compte_de_l_equipe() {
        let db = base_de_test().await;
        let max = vrai_compte(&db, "max@exemple.fr").await;
        fiche(&db, &max).await;
        let instant = Utc::now().naive_utc();
        plans::ActiveModel {
            id: Set("plan-max".to_string()),
            author_id: Set(max.clone()),
            title: Set("Mon plan de test".to_string()),
            note: Set(String::new()),
            category: Set("sortie".to_string()),
            starts_at: Set(instant + Duration::days(1)),
            city: Set("Lyon".to_string()),
            lat_rounded: Set(45.76),
            lon_rounded: Set(4.84),
            capacity: Set(2),
            state: Set("ouvert".to_string()),
            created_at: Set(instant),
            updated_at: Set(instant),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();

        let bilan = semer(
            &db,
            &garde_de_dev(),
            &Autour::depuis(Some("Max@Exemple.fr")),
        )
        .await
        .unwrap();
        assert_eq!(bilan.demandes_pour_l_equipe, 3);
        let recues = join_requests::Entity::find()
            .filter(join_requests::Column::PlanId.eq("plan-max"))
            .count(&db)
            .await
            .unwrap();
        assert_eq!(recues, 3);
        for f in profiles::Entity::find()
            .filter(profiles::Column::AccountId.ne(max.as_str()))
            .all(&db)
            .await
            .unwrap()
        {
            assert_eq!(f.city, "Lyon");
        }
    }

    #[tokio::test]
    async fn une_ville_inconnue_echoue() {
        let db = base_de_test().await;
        assert!(
            semer(&db, &garde_de_dev(), &Autour::depuis(Some("Atlantide")))
                .await
                .is_err()
        );
    }

    /// De bout en bout : les plans semés paraissent dans le vrai fil d'un
    /// compte de l'équipe, lu par la vraie route.
    #[tokio::test]
    async fn les_plans_semes_paraissent_dans_le_fil() {
        let service = crate::tests::Service::monter().await;
        let max = service.compte("c_equipe_seed", "depart").await;
        let garde = Garde {
            equipe: vec![format!("{max}@exemple.fr")],
            ..garde_de_dev()
        };
        semer(&service.db, &garde, &Autour::depuis(Some("Lyon")))
            .await
            .unwrap();

        let (statut, corps) = service
            .get("/v1/plans", Some(&service.jeton("c_equipe_seed")))
            .await;
        assert_eq!(statut, axum::http::StatusCode::OK, "{corps}");
        let titres: Vec<&str> = corps["plans"]
            .as_array()
            .expect("une liste")
            .iter()
            .filter_map(|p| p["title"].as_str())
            .collect();
        assert!(
            titres
                .iter()
                .any(|t| PERSONNES.iter().any(|p| p.titre == *t)),
            "aucun plan fictif dans le fil : {titres:?}"
        );
    }

    #[tokio::test]
    async fn la_publication_retire_les_fictifs() {
        let db = base_de_test().await;
        semer(&db, &garde_de_dev(), &Autour::depuis(None))
            .await
            .unwrap();

        assert_eq!(retirer_si_publiee(&db, false).await.unwrap(), 0);
        assert_eq!(nombre_de_comptes(&db).await, 16);

        assert_eq!(retirer_si_publiee(&db, true).await.unwrap(), 16);
        assert_eq!(nombre_de_comptes(&db).await, 0);
    }
}
