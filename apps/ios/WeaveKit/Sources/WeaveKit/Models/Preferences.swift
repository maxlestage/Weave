import Foundation

/// Les critères du fil, tels que le serveur les tient.
///
/// ## Pourquoi ce modèle manquait
///
/// `PreferencesPatch` n'était qu'`Encodable` : l'application écrivait ses
/// critères sans jamais les relire. L'écran des réglages affichait donc des
/// valeurs écrites en dur — 25 km, 18 à 32 ans — quels que soient les réglages
/// réels.
///
/// Ce n'était pas qu'un affichage faux. Les deux curseurs d'âge s'appliquent
/// ensemble : ajuster le minimum réécrivait aussi le maximum, avec la valeur
/// par défaut affichée plutôt que la vraie. Ouvrir les réglages et toucher un
/// curseur suffisait à perdre l'autre.
public struct Preferences: Decodable, Sendable {
    public let minAge: Int
    public let maxAge: Int
    public let maxDistanceKm: Int
    public let seeking: [Gender]
    public let categories: [PlanCategory]
    /// Ville de l'escale en cours, s'il y en a une.
    public let escaleCity: String?
    /// Fin de l'escale en cours.
    public let escaleUntil: Date?

    /// Une escale court-elle en ce moment ?
    ///
    /// La date fait foi plutôt que la seule présence d'une ville : le serveur
    /// laisse les deux colonnes en place après le terme, et c'est l'échéance
    /// qui décide si le fil s'y compose encore.
    public var escaleEnCours: Bool {
        guard let escaleUntil else { return false }
        return escaleUntil > Date()
    }
}

/// Une escale ouverte.
public struct Escale: Decodable, Sendable {
    public let escaleCity: String
    public let escaleUntil: Date
}
