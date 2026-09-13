import Foundation

/// Les genres, pour la fiche et pour les critères du fil.
///
/// Les valeurs brutes sont celles de `GENDERS` dans le contrat partagé, et le
/// test du contrat tient l'accord des trois côtés. Un vocabulaire fixe est ce
/// qui rend la correspondance possible : le fil retient un plan quand le genre
/// de son auteur figure parmi ceux que le lecteur cherche, comparés caractère
/// par caractère. « Femme » et « femme » ne se rencontreraient jamais.
public enum Gender: String, CaseIterable, Identifiable, Codable, Sendable {
    case femme
    case homme
    case nonBinaire = "non_binaire"
    case autre

    public var id: String { rawValue }

    public var libelle: String {
        switch self {
        case .femme: "Femme"
        case .homme: "Homme"
        case .nonBinaire: "Non binaire"
        case .autre: "Autre"
        }
    }
}

/// Longueur maximale de la phrase de présentation, côté serveur comme ici.
public let bioMaxChars = 160
