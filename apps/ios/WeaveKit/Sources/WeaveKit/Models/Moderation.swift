import Foundation

/// Les motifs de signalement, tels que le serveur les accepte.
///
/// Les valeurs brutes sont celles de `MOTIFS` dans `routes/moderation.rs` : un
/// motif inconnu est refusé. Les libellés sont ici plutôt que dans la vue —
/// c'est la même liste qui sert au menu et à l'envoi, et deux listes
/// divergeraient.
///
/// L'ordre est celui de l'affichage : du plus grave au plus vague. « Autre »
/// vient en dernier parce qu'il demande des précisions, et qu'on ne veut pas
/// qu'il serve de premier réflexe.
public enum ReportReason: String, CaseIterable, Identifiable, Sendable {
    case mineur
    case harcelement
    case contenuSexuel = "contenu_sexuel"
    case fauxProfil = "faux_profil"
    case arnaque
    case autre

    public var id: String { rawValue }

    /// Ce qui s'affiche dans le menu.
    public var libelle: String {
        switch self {
        case .mineur: "Il s'agit d'un mineur"
        case .harcelement: "Harcèlement ou menaces"
        case .contenuSexuel: "Contenu sexuel"
        case .fauxProfil: "Faux profil"
        case .arnaque: "Arnaque ou sollicitation"
        case .autre: "Autre"
        }
    }

    /// Les précisions sont exigées quand le motif, seul, ne dit rien.
    public var exigeDesPrecisions: Bool { self == .autre }
}

/// Longueur maximale des précisions, côté serveur comme ici.
///
/// Le serveur refuse au-delà : mieux vaut le dire pendant la saisie que
/// rejeter l'envoi une fois le texte écrit.
public let reportDetailsMaxChars = 1000
