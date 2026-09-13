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

/// Taille maximale d'une photo de profil, côté serveur comme ici.
///
/// L'application recompresse avant d'envoyer : cette borne est ce que le
/// serveur refuse, pas ce qu'on vise. Mieux vaut le dire pendant la
/// préparation que rejeter l'envoi une fois la photo choisie.
public let photoMaxBytes = 2 * 1024 * 1024

/// Côté le plus long d'une photo après recompression.
///
/// Une photo de profil s'affiche dans une vignette ; envoyer les douze
/// mégapixels du capteur ferait payer à la personne un téléversement qu'elle
/// ne verra jamais, et à la base un stockage qui ne sert à rien.
public let photoMaxCote: Double = 1080
