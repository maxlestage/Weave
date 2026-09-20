//! Les phrases que ce service dit au client, dans les trois langues.
//!
//! ## Pourquoi elles sont ici et non à leur place d'appel
//!
//! Elles y étaient, écrites en français au fond de chaque route. L'application
//! iOS les rend telles quelles — `WeaveAPI.swift` : « `message` est celui du
//! serveur, et il est rendu tel quel ». Une application en anglais affichait
//! donc « Ce plan est complet. » au milieu de son propre texte.
//!
//! Le code d'erreur ne suffit pas à s'en passer. Il est stable et partagé,
//! mais treize codes ne distinguent pas quatre-vingts situations : « Ce plan
//! est complet. », « Ce plan a déjà eu lieu. » et « Ce plan n'accepte plus de
//! demandes. » portent tous `plan_closed`. C'est la phrase qui dit ce qui
//! s'est passé.
//!
//! ## Ce que l'énumération garantit
//!
//! Chaque variante rend ses trois langues par un `match` exhaustif : ajouter
//! une phrase sans l'écrire en anglais et en espagnol ne compile pas. C'est la
//! seule garantie qui tienne — une table de correspondance se remplit à moitié
//! et personne ne s'en aperçoit avant qu'un client affiche une clé.
//!
//! ## Ce qui n'est pas traduit
//!
//! Les valeurs que le client a envoyées et que nous lui répétons — une
//! catégorie inconnue, un identifiant de produit — restent telles quelles :
//! ce sont ses données, pas notre prose.

use crate::langue::{Langue, courante};

/// Une phrase adressée au client. Les paramètres sont ceux qui varient d'un
/// appel à l'autre ; les constantes du domaine sont passées par l'appelant,
/// qui les a déjà sous la main.
///
/// Les nombres sont tous des `i64`, quel que soit le type de la constante
/// d'origine — `usize` ici, `i32` là. Un catalogue de phrases n'a rien à
/// dire sur la largeur d'un entier, et faire porter à chaque variante le
/// type de sa constante rendrait le fichier illisible pour rien.
#[derive(Debug, Clone)]
pub enum Msg {
    // — Authentification et session ——————————————————————————————
    AuthentificationRequise,
    SessionExpiree,
    JetonDeMiseAJourInvalide,
    AdresseEmailInvalide,
    CodeIncorrect,
    CodeExpire,
    TropDeTentativesSurCeCode,
    CompteSuspendu,
    CompteEnSuppression,
    CompteIntrouvable,

    // — Profil ————————————————————————————————————————————————————
    DateDeNaissanceInvalide,
    AgeMinimumRequis {
        minimum: i64,
    },
    AgeHorsBornes {
        nom: String,
        minimum: i64,
        maximum: i64,
    },
    AgeMinSuperieurAuMax,
    NomAfficheTropLong {
        maximum: i64,
    },
    PhraseTropLongue {
        maximum: i64,
    },
    IndiquezUneVille,
    RenseignezDAbordVotreVille,
    CoordonneesInvalides,
    FuseauHoraireInvalide,
    LangueInvalide,
    GenreInconnu {
        valeur: String,
        acceptees: String,
    },
    /// Le même refus, là où la liste des valeurs acceptées a déjà été dite.
    GenreInconnuSimple {
        valeur: String,
    },
    ProfilDejaVerifie,

    // — Plans ——————————————————————————————————————————————————————
    PlanIntrouvable,
    PlanDisparu,
    PlanPasLeVotre,
    PlanDejaEuLieu,
    PlanComplet,
    PlanNAcceptePlusDeDemandes,
    ToutesLesPlacesSontPrises,
    VotreProprePlan,
    TitreLongueur {
        minimum: i64,
        maximum: i64,
    },
    NoteTropLongue {
        maximum: i64,
    },
    MotTropLong {
        maximum: i64,
    },
    CapaciteHorsBornes {
        minimum: i64,
        maximum: i64,
    },
    DelaiDePublicationTropCourt {
        minutes: i64,
    },
    HorizonDePublicationDepasse {
        jours: i64,
    },
    DateDeRendezVousIllisible,
    CategorieInconnue,
    CategorieInconnueNommee {
        valeur: String,
    },
    HuitCategoriesAuPlus,
    SeptJoursAuPlus,
    JourInconnu {
        valeur: String,
    },
    RayonHorsBornes {
        maximum: i64,
    },
    ChoixTropNombreux {
        maximum: i64,
    },
    CriteresIntrouvables,
    BilanTropPeuDePlans {
        minimum: i64,
        passes: i64,
    },

    // — Demandes ————————————————————————————————————————————————————
    DemandeIntrouvable,
    DemandePasLaVotre,
    DemandeDejaRepondue,
    DemandeDejaTranchee,
    /// Rendre sa place suppose qu'on l'ait : ni une demande encore en
    /// attente, ni une déjà rendue, ni une refusée.
    PlaceNonRendable,
    /// Se désister après l'heure dite n'apprend plus rien à personne.
    RendezVousDejaPasse,
    DejaDemandeAVenir,
    MessageTropCourt {
        minimum: i64,
    },
    MessageDeDemandeTropLong {
        maximum: i64,
    },

    // — Conversations ————————————————————————————————————————————————
    ConversationIntrouvable,
    ConversationClose,
    ConversationPasLaVotre,
    MessageVide,
    MessageTropLong {
        maximum: i64,
    },

    // — Modération ———————————————————————————————————————————————————
    MotifDeSignalementInconnu,
    PrecisionsTropLongues {
        maximum: i64,
    },
    PasDeAutoBlocage,

    // — Médias ————————————————————————————————————————————————————————
    MediaDisparu,
    LienMediaExpire,
    ImageTropLourde,
    FormatDImageNonReconnu,
    FlouProgressifNonApplique,

    // — Appareils ——————————————————————————————————————————————————————
    AppareilInconnu,
    IdentifiantDAppareilInvalide,

    // — Achats ——————————————————————————————————————————————————————————
    TransactionStoreKitIllisible,
    TransactionStoreKitRefusee,
    TransactionIncomplete,
    ProduitDAbonnementInconnu {
        identifiant: String,
    },
    ProduitALUniteInconnu {
        identifiant: String,
    },
    EscaleDejaEnCours,
    PauseHorsEtat,
    VerificationDesAchatsIndisponible,

    // — Consentements ————————————————————————————————————————————————————
    ObjetDeConsentementInconnu,
    ConsentementSurVersionPerimee,

    // — Divers ————————————————————————————————————————————————————————————
    CurseurAvantIso8601,
    TropDeRenforts {
        maximum: i64,
    },
    LimiteAtteinte {
        quoi: String,
        secondes: i64,
    },
    ErreurInterne,
}

impl Msg {
    /// La phrase, dans la langue de la requête en cours.
    pub fn t(&self) -> String {
        self.dans(courante())
    }

    /// La phrase, dans une langue donnée. Publique pour les tests, qui n'ont
    /// pas de requête autour d'eux.
    pub fn dans(&self, langue: Langue) -> String {
        match langue {
            Langue::Fr => self.fr(),
            Langue::En => self.en(),
            Langue::Es => self.es(),
        }
    }

    fn fr(&self) -> String {
        use Msg::*;
        match self {
            AuthentificationRequise => "Authentification requise.".into(),
            SessionExpiree => "Session expirée. Reconnectez-vous.".into(),
            JetonDeMiseAJourInvalide => "Jeton de mise à jour invalide.".into(),
            AdresseEmailInvalide => "Adresse e-mail invalide.".into(),
            CodeIncorrect => "Code incorrect.".into(),
            CodeExpire => "Code expiré ou déjà utilisé.".into(),
            TropDeTentativesSurCeCode => {
                "Trop de tentatives sur ce code. Demandez-en un nouveau.".into()
            }
            CompteSuspendu => "Ce compte est suspendu.".into(),
            CompteEnSuppression => {
                "Ce compte est en cours de suppression. Écrivez à l'assistance pour l'annuler."
                    .into()
            }
            CompteIntrouvable => "Compte introuvable.".into(),

            DateDeNaissanceInvalide => "Date de naissance invalide.".into(),
            AgeMinimumRequis { minimum } => {
                format!("Weave est réservé aux personnes de {minimum} ans et plus.")
            }
            AgeHorsBornes {
                nom,
                minimum,
                maximum,
            } => {
                format!("L'âge {nom} doit être compris entre {minimum} et {maximum} ans.")
            }
            AgeMinSuperieurAuMax => "L'âge minimum ne peut pas dépasser l'âge maximum.".into(),
            NomAfficheTropLong { maximum } => {
                format!("Le nom affiché tient en {maximum} caractères.")
            }
            PhraseTropLongue { maximum } => format!("La phrase tient en {maximum} caractères."),
            IndiquezUneVille => "Indiquez une ville.".into(),
            RenseignezDAbordVotreVille => "Renseignez d'abord votre ville.".into(),
            CoordonneesInvalides => "Coordonnées invalides.".into(),
            FuseauHoraireInvalide => "Fuseau horaire invalide.".into(),
            LangueInvalide => "Langue invalide.".into(),
            GenreInconnu { valeur, acceptees } => {
                format!("Genre inconnu : {valeur}. Valeurs acceptées : {acceptees}.")
            }
            GenreInconnuSimple { valeur } => format!("Genre inconnu : {valeur}."),
            ProfilDejaVerifie => "Votre profil est déjà vérifié.".into(),

            PlanIntrouvable => "Plan introuvable.".into(),
            PlanDisparu => "Ce plan n'existe plus.".into(),
            PlanPasLeVotre => "Ce plan n'est pas le vôtre.".into(),
            PlanDejaEuLieu => "Ce plan a déjà eu lieu.".into(),
            PlanComplet => "Ce plan est complet.".into(),
            PlanNAcceptePlusDeDemandes => "Ce plan n'accepte plus de demandes.".into(),
            ToutesLesPlacesSontPrises => "Toutes les places sont prises.".into(),
            VotreProprePlan => "C'est votre propre plan.".into(),
            TitreLongueur { minimum, maximum } => {
                format!("Le titre doit faire entre {minimum} et {maximum} caractères.")
            }
            NoteTropLongue { maximum } => {
                format!("La note ne peut pas dépasser {maximum} caractères.")
            }
            MotTropLong { maximum } => {
                format!("Ce mot ne peut pas dépasser {maximum} caractères.")
            }
            CapaciteHorsBornes { minimum, maximum } => {
                format!("La capacité doit être comprise entre {minimum} et {maximum}.")
            }
            DelaiDePublicationTropCourt { minutes } => {
                format!("Un plan se publie au moins {minutes} minutes à l'avance.")
            }
            HorizonDePublicationDepasse { jours } => {
                format!("Un plan se publie au plus {jours} jours à l'avance.")
            }
            DateDeRendezVousIllisible => "Date de rendez-vous illisible.".into(),
            CategorieInconnue => "Catégorie inconnue.".into(),
            CategorieInconnueNommee { valeur } => format!("Catégorie inconnue : {valeur}."),
            HuitCategoriesAuPlus => "Huit catégories au plus.".into(),
            SeptJoursAuPlus => "Sept jours au plus.".into(),
            JourInconnu { valeur } => {
                format!("Jour inconnu : {valeur}. De 1 (lundi) à 7 (dimanche).")
            }
            RayonHorsBornes { maximum } => {
                format!("Le rayon va de 1 à {maximum} kilomètres.")
            }
            ChoixTropNombreux { maximum } => format!("{maximum} choix au plus."),
            CriteresIntrouvables => "Critères introuvables.".into(),
            BilanTropPeuDePlans { minimum, passes } => format!(
                "Il faut au moins {minimum} plans passés pour qu'un bilan dise quelque chose. \
                 Vous en avez {passes}. Votre crédit n'a pas été utilisé."
            ),

            DemandeIntrouvable => "Demande introuvable.".into(),
            DemandePasLaVotre => "Cette demande n'est pas la vôtre.".into(),
            DemandeDejaRepondue => "Cette demande a déjà reçu une réponse.".into(),
            DemandeDejaTranchee => "Cette demande est déjà tranchée.".into(),
            PlaceNonRendable => "Vous n'avez pas de place à rendre sur ce plan.".into(),
            RendezVousDejaPasse => {
                "Le rendez-vous est passé : il n'y a plus de place à rendre.".into()
            }
            DejaDemandeAVenir => {
                "Vous avez déjà demandé à venir. On ne redemande pas deux fois.".into()
            }
            MessageTropCourt { minimum } => format!(
                "Écrivez au moins {minimum} caractères : c'est ce qui distingue une demande d'un geste."
            ),
            MessageDeDemandeTropLong { maximum } => {
                format!("Un message ne peut pas dépasser {maximum} caractères.")
            }

            ConversationIntrouvable => "Conversation introuvable.".into(),
            ConversationClose => "Cette conversation est close.".into(),
            ConversationPasLaVotre => "Cette conversation n'est pas la vôtre.".into(),
            MessageVide => "Un message vide ne dit rien.".into(),
            MessageTropLong { maximum } => {
                format!("Le message ne peut pas dépasser {maximum} caractères.")
            }

            MotifDeSignalementInconnu => "Motif de signalement inconnu.".into(),
            PrecisionsTropLongues { maximum } => {
                format!("Les précisions ne peuvent pas dépasser {maximum} caractères.")
            }
            PasDeAutoBlocage => "Vous ne pouvez pas vous bloquer vous-même.".into(),

            MediaDisparu => "Ce média n'existe plus.".into(),
            LienMediaExpire => "Lien média expiré ou invalide.".into(),
            ImageTropLourde => "Cette image dépasse 2 Mo.".into(),
            FormatDImageNonReconnu => {
                "Format d'image non reconnu. JPEG, PNG ou HEIC sont acceptés.".into()
            }
            FlouProgressifNonApplique => {
                "Le flou progressif n'est pas encore appliqué par ce service.".into()
            }

            AppareilInconnu => "Appareil inconnu. Enregistrez-le d'abord.".into(),
            IdentifiantDAppareilInvalide => "Identifiant d'appareil invalide.".into(),

            TransactionStoreKitIllisible => "Transaction StoreKit illisible.".into(),
            TransactionStoreKitRefusee => "Transaction StoreKit refusée.".into(),
            TransactionIncomplete => "Transaction incomplète.".into(),
            ProduitDAbonnementInconnu { identifiant } => {
                format!("Produit d'abonnement inconnu : {identifiant}")
            }
            ProduitALUniteInconnu { identifiant } => {
                format!("Produit à l'unité inconnu : {identifiant}")
            }
            EscaleDejaEnCours => {
                "Une escale est déjà en cours. Attendez sa fin, ou fermez-la.".into()
            }
            PauseHorsEtat => "Ce compte n'est pas dans un état où la pause s'applique.".into(),
            VerificationDesAchatsIndisponible => {
                "Vérification des achats indisponible : la validation cryptographique des \
                 transactions App Store n'est pas encore en service."
                    .into()
            }

            ObjetDeConsentementInconnu => "Objet de consentement inconnu.".into(),
            ConsentementSurVersionPerimee => {
                "Ce consentement porte sur une version du texte qui n'est plus en vigueur.".into()
            }

            CurseurAvantIso8601 => "Le curseur « before » attend une date ISO 8601.".into(),
            TropDeRenforts { maximum } => {
                format!("Au plus {maximum} renforts par jour. Vos demandes reviennent à minuit.")
            }
            LimiteAtteinte { quoi, secondes } => {
                format!("Limite atteinte pour « {quoi} ». Réessayez dans {secondes} s.")
            }
            ErreurInterne => "Une erreur interne est survenue.".into(),
        }
    }

    fn en(&self) -> String {
        use Msg::*;
        match self {
            AuthentificationRequise => "You need to sign in.".into(),
            SessionExpiree => "Your session has expired. Please sign in again.".into(),
            JetonDeMiseAJourInvalide => "Invalid refresh token.".into(),
            AdresseEmailInvalide => "That email address isn't valid.".into(),
            CodeIncorrect => "Wrong code.".into(),
            CodeExpire => "That code has expired or has already been used.".into(),
            TropDeTentativesSurCeCode => {
                "Too many attempts on this code. Ask for a new one.".into()
            }
            CompteSuspendu => "This account is suspended.".into(),
            CompteEnSuppression => {
                "This account is being deleted. Write to support to cancel that.".into()
            }
            CompteIntrouvable => "Account not found.".into(),

            DateDeNaissanceInvalide => "That date of birth isn't valid.".into(),
            AgeMinimumRequis { minimum } => {
                format!("Weave is for people aged {minimum} and over.")
            }
            AgeHorsBornes {
                nom,
                minimum,
                maximum,
            } => {
                format!("The {nom} age must be between {minimum} and {maximum}.")
            }
            AgeMinSuperieurAuMax => "The minimum age can't be above the maximum age.".into(),
            NomAfficheTropLong { maximum } => {
                format!("Your display name has to fit in {maximum} characters.")
            }
            PhraseTropLongue { maximum } => {
                format!("Your sentence has to fit in {maximum} characters.")
            }
            IndiquezUneVille => "Give a town.".into(),
            RenseignezDAbordVotreVille => "Set your town first.".into(),
            CoordonneesInvalides => "Those coordinates aren't valid.".into(),
            FuseauHoraireInvalide => "That time zone isn't valid.".into(),
            LangueInvalide => "That language isn't valid.".into(),
            GenreInconnu { valeur, acceptees } => {
                format!("Unknown gender: {valeur}. Accepted values: {acceptees}.")
            }
            GenreInconnuSimple { valeur } => format!("Unknown gender: {valeur}."),
            ProfilDejaVerifie => "Your profile is already verified.".into(),

            PlanIntrouvable => "Plan not found.".into(),
            PlanDisparu => "That plan no longer exists.".into(),
            PlanPasLeVotre => "That plan isn't yours.".into(),
            PlanDejaEuLieu => "That plan has already happened.".into(),
            PlanComplet => "That plan is full.".into(),
            PlanNAcceptePlusDeDemandes => "That plan isn't taking requests any more.".into(),
            ToutesLesPlacesSontPrises => "Every spot is taken.".into(),
            VotreProprePlan => "That's your own plan.".into(),
            TitreLongueur { minimum, maximum } => {
                format!("The title has to be between {minimum} and {maximum} characters.")
            }
            NoteTropLongue { maximum } => format!("The note can't be over {maximum} characters."),
            MotTropLong { maximum } => format!("That note can't be over {maximum} characters."),
            CapaciteHorsBornes { minimum, maximum } => {
                format!("The capacity has to be between {minimum} and {maximum}.")
            }
            DelaiDePublicationTropCourt { minutes } => {
                format!("A plan goes up at least {minutes} minutes ahead.")
            }
            HorizonDePublicationDepasse { jours } => {
                format!("A plan goes up at most {jours} days ahead.")
            }
            DateDeRendezVousIllisible => "That date and time can't be read.".into(),
            CategorieInconnue => "Unknown category.".into(),
            CategorieInconnueNommee { valeur } => format!("Unknown category: {valeur}."),
            HuitCategoriesAuPlus => "Eight categories at most.".into(),
            SeptJoursAuPlus => "Seven days at most.".into(),
            JourInconnu { valeur } => {
                format!("Unknown day: {valeur}. From 1 (Monday) to 7 (Sunday).")
            }
            RayonHorsBornes { maximum } => {
                format!("The radius runs from 1 to {maximum} kilometres.")
            }
            ChoixTropNombreux { maximum } => format!("{maximum} choices at most."),
            CriteresIntrouvables => "Filters not found.".into(),
            BilanTropPeuDePlans { minimum, passes } => format!(
                "A Bilan needs at least {minimum} past plans to say anything. You have {passes}. \
                 Your credit has not been used."
            ),

            DemandeIntrouvable => "Request not found.".into(),
            DemandePasLaVotre => "That request isn't yours.".into(),
            DemandeDejaRepondue => "That request has already been answered.".into(),
            DemandeDejaTranchee => "That request has already been settled.".into(),
            PlaceNonRendable => "You have no spot to give back on that plan.".into(),
            RendezVousDejaPasse => {
                "The meeting time has passed: there's no spot left to give back.".into()
            }
            DejaDemandeAVenir => "You've already asked to come. You don't ask twice.".into(),
            MessageTropCourt { minimum } => format!(
                "Write at least {minimum} characters: that's what separates a request from a gesture."
            ),
            MessageDeDemandeTropLong { maximum } => {
                format!("A message can't be over {maximum} characters.")
            }

            ConversationIntrouvable => "Conversation not found.".into(),
            ConversationClose => "That conversation is closed.".into(),
            ConversationPasLaVotre => "That conversation isn't yours.".into(),
            MessageVide => "An empty message says nothing.".into(),
            MessageTropLong { maximum } => {
                format!("The message can't be over {maximum} characters.")
            }

            MotifDeSignalementInconnu => "Unknown reason for reporting.".into(),
            PrecisionsTropLongues { maximum } => {
                format!("The details can't be over {maximum} characters.")
            }
            PasDeAutoBlocage => "You can't block yourself.".into(),

            MediaDisparu => "That media no longer exists.".into(),
            LienMediaExpire => "That media link has expired or isn't valid.".into(),
            ImageTropLourde => "That image is over 2 MB.".into(),
            FormatDImageNonReconnu => {
                "Image format not recognised. JPEG, PNG or HEIC are accepted.".into()
            }
            FlouProgressifNonApplique => {
                "Progressive blurring isn't applied by this service yet.".into()
            }

            AppareilInconnu => "Unknown device. Register it first.".into(),
            IdentifiantDAppareilInvalide => "Invalid device identifier.".into(),

            TransactionStoreKitIllisible => "That StoreKit transaction can't be read.".into(),
            TransactionStoreKitRefusee => "That StoreKit transaction was refused.".into(),
            TransactionIncomplete => "Incomplete transaction.".into(),
            ProduitDAbonnementInconnu { identifiant } => {
                format!("Unknown subscription product: {identifiant}")
            }
            ProduitALUniteInconnu { identifiant } => {
                format!("Unknown single-purchase product: {identifiant}")
            }
            EscaleDejaEnCours => {
                "An Escale is already running. Wait for it to end, or close it.".into()
            }
            PauseHorsEtat => "This account isn't in a state where pausing applies.".into(),
            VerificationDesAchatsIndisponible => {
                "Purchase verification is unavailable: cryptographic validation of App Store \
                 transactions isn't in service yet."
                    .into()
            }

            ObjetDeConsentementInconnu => "Unknown consent subject.".into(),
            ConsentementSurVersionPerimee => {
                "That consent refers to a version of the text that is no longer in force.".into()
            }

            CurseurAvantIso8601 => "The “before” cursor expects an ISO 8601 date.".into(),
            TropDeRenforts { maximum } => {
                format!("At most {maximum} Renforts a day. Your requests come back at midnight.")
            }
            LimiteAtteinte { quoi, secondes } => {
                format!("Limit reached for “{quoi}”. Try again in {secondes} s.")
            }
            ErreurInterne => "Something went wrong on our side.".into(),
        }
    }

    fn es(&self) -> String {
        use Msg::*;
        match self {
            AuthentificationRequise => "Hay que iniciar sesión.".into(),
            SessionExpiree => "Tu sesión ha caducado. Vuelve a iniciar sesión.".into(),
            JetonDeMiseAJourInvalide => "Token de renovación no válido.".into(),
            AdresseEmailInvalide => "Esa dirección de correo no es válida.".into(),
            CodeIncorrect => "Código incorrecto.".into(),
            CodeExpire => "Ese código ha caducado o ya se ha usado.".into(),
            TropDeTentativesSurCeCode => {
                "Demasiados intentos con este código. Pide uno nuevo.".into()
            }
            CompteSuspendu => "Esta cuenta está suspendida.".into(),
            CompteEnSuppression => {
                "Esta cuenta se está eliminando. Escribe a soporte para cancelarlo.".into()
            }
            CompteIntrouvable => "Cuenta no encontrada.".into(),

            DateDeNaissanceInvalide => "Esa fecha de nacimiento no es válida.".into(),
            AgeMinimumRequis { minimum } => {
                format!("Weave es para mayores de {minimum} años.")
            }
            AgeHorsBornes {
                nom,
                minimum,
                maximum,
            } => {
                format!("La edad {nom} debe estar entre {minimum} y {maximum} años.")
            }
            AgeMinSuperieurAuMax => "La edad mínima no puede superar a la máxima.".into(),
            NomAfficheTropLong { maximum } => {
                format!("El nombre visible cabe en {maximum} caracteres.")
            }
            PhraseTropLongue { maximum } => format!("La frase cabe en {maximum} caracteres."),
            IndiquezUneVille => "Indica una ciudad.".into(),
            RenseignezDAbordVotreVille => "Indica antes tu ciudad.".into(),
            CoordonneesInvalides => "Esas coordenadas no son válidas.".into(),
            FuseauHoraireInvalide => "Esa zona horaria no es válida.".into(),
            LangueInvalide => "Ese idioma no es válido.".into(),
            GenreInconnu { valeur, acceptees } => {
                format!("Género desconocido: {valeur}. Valores aceptados: {acceptees}.")
            }
            GenreInconnuSimple { valeur } => format!("Género desconocido: {valeur}."),
            ProfilDejaVerifie => "Tu perfil ya está verificado.".into(),

            PlanIntrouvable => "Plan no encontrado.".into(),
            PlanDisparu => "Ese plan ya no existe.".into(),
            PlanPasLeVotre => "Ese plan no es tuyo.".into(),
            PlanDejaEuLieu => "Ese plan ya ha tenido lugar.".into(),
            PlanComplet => "Ese plan está completo.".into(),
            PlanNAcceptePlusDeDemandes => "Ese plan ya no acepta peticiones.".into(),
            ToutesLesPlacesSontPrises => "Todas las plazas están ocupadas.".into(),
            VotreProprePlan => "Es tu propio plan.".into(),
            TitreLongueur { minimum, maximum } => {
                format!("El título debe tener entre {minimum} y {maximum} caracteres.")
            }
            NoteTropLongue { maximum } => {
                format!("La nota no puede superar los {maximum} caracteres.")
            }
            MotTropLong { maximum } => {
                format!("Esa nota no puede superar los {maximum} caracteres.")
            }
            CapaciteHorsBornes { minimum, maximum } => {
                format!("La capacidad debe estar entre {minimum} y {maximum}.")
            }
            DelaiDePublicationTropCourt { minutes } => {
                format!("Un plan se publica con al menos {minutes} minutos de antelación.")
            }
            HorizonDePublicationDepasse { jours } => {
                format!("Un plan se publica como máximo con {jours} días de antelación.")
            }
            DateDeRendezVousIllisible => "Esa fecha y hora no se pueden leer.".into(),
            CategorieInconnue => "Categoría desconocida.".into(),
            CategorieInconnueNommee { valeur } => format!("Categoría desconocida: {valeur}."),
            HuitCategoriesAuPlus => "Ocho categorías como máximo.".into(),
            SeptJoursAuPlus => "Siete días como máximo.".into(),
            JourInconnu { valeur } => {
                format!("Día desconocido: {valeur}. Del 1 (lunes) al 7 (domingo).")
            }
            RayonHorsBornes { maximum } => {
                format!("El radio va de 1 a {maximum} kilómetros.")
            }
            ChoixTropNombreux { maximum } => format!("{maximum} opciones como máximo."),
            CriteresIntrouvables => "Criterios no encontrados.".into(),
            BilanTropPeuDePlans { minimum, passes } => format!(
                "Un Bilan necesita al menos {minimum} planes pasados para decir algo. Tienes \
                 {passes}. Tu crédito no se ha usado."
            ),

            DemandeIntrouvable => "Petición no encontrada.".into(),
            DemandePasLaVotre => "Esa petición no es tuya.".into(),
            DemandeDejaRepondue => "Esa petición ya ha recibido respuesta.".into(),
            DemandeDejaTranchee => "Esa petición ya está resuelta.".into(),
            PlaceNonRendable => "No tienes ningún sitio que devolver en ese plan.".into(),
            RendezVousDejaPasse => {
                "La hora de la cita ya pasó: no queda sitio que devolver.".into()
            }
            DejaDemandeAVenir => "Ya has pedido venir. No se pide dos veces.".into(),
            MessageTropCourt { minimum } => format!(
                "Escribe al menos {minimum} caracteres: es lo que distingue una petición de un gesto."
            ),
            MessageDeDemandeTropLong { maximum } => {
                format!("Un mensaje no puede superar los {maximum} caracteres.")
            }

            ConversationIntrouvable => "Conversación no encontrada.".into(),
            ConversationClose => "Esa conversación está cerrada.".into(),
            ConversationPasLaVotre => "Esa conversación no es tuya.".into(),
            MessageVide => "Un mensaje vacío no dice nada.".into(),
            MessageTropLong { maximum } => {
                format!("El mensaje no puede superar los {maximum} caracteres.")
            }

            MotifDeSignalementInconnu => "Motivo de denuncia desconocido.".into(),
            PrecisionsTropLongues { maximum } => {
                format!("Los detalles no pueden superar los {maximum} caracteres.")
            }
            PasDeAutoBlocage => "No puedes bloquearte a ti mismo.".into(),

            MediaDisparu => "Ese archivo ya no existe.".into(),
            LienMediaExpire => "Ese enlace ha caducado o no es válido.".into(),
            ImageTropLourde => "Esa imagen supera los 2 MB.".into(),
            FormatDImageNonReconnu => {
                "Formato de imagen no reconocido. Se aceptan JPEG, PNG o HEIC.".into()
            }
            FlouProgressifNonApplique => {
                "Este servicio todavía no aplica el desenfoque progresivo.".into()
            }

            AppareilInconnu => "Dispositivo desconocido. Regístralo antes.".into(),
            IdentifiantDAppareilInvalide => "Identificador de dispositivo no válido.".into(),

            TransactionStoreKitIllisible => "Esa transacción de StoreKit no se puede leer.".into(),
            TransactionStoreKitRefusee => "Esa transacción de StoreKit ha sido rechazada.".into(),
            TransactionIncomplete => "Transacción incompleta.".into(),
            ProduitDAbonnementInconnu { identifiant } => {
                format!("Producto de suscripción desconocido: {identifiant}")
            }
            ProduitALUniteInconnu { identifiant } => {
                format!("Producto por unidades desconocido: {identifiant}")
            }
            EscaleDejaEnCours => {
                "Ya hay una Escale en curso. Espera a que termine, o ciérrala.".into()
            }
            PauseHorsEtat => {
                "Esta cuenta no está en un estado en el que se aplique la pausa.".into()
            }
            VerificationDesAchatsIndisponible => {
                "La verificación de compras no está disponible: la validación criptográfica de \
                 las transacciones de App Store todavía no está en servicio."
                    .into()
            }

            ObjetDeConsentementInconnu => "Objeto de consentimiento desconocido.".into(),
            ConsentementSurVersionPerimee => {
                "Ese consentimiento se refiere a una versión del texto que ya no está en vigor."
                    .into()
            }

            CurseurAvantIso8601 => "El cursor « before » espera una fecha ISO 8601.".into(),
            TropDeRenforts { maximum } => format!(
                "Como máximo {maximum} Renforts al día. Tus peticiones vuelven a medianoche."
            ),
            LimiteAtteinte { quoi, secondes } => {
                format!("Límite alcanzado para « {quoi} ». Inténtalo de nuevo en {secondes} s.")
            }
            ErreurInterne => "Ha ocurrido un error por nuestra parte.".into(),
        }
    }
}
