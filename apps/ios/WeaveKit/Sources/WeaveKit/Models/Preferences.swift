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
    /// Le rayon que le fil applique réellement.
    ///
    /// Sans « critères précis », il est rabattu sur l'un de quelques crans : le
    /// réglage reste celui qu'on a choisi — il revient exact dès que l'offre le
    /// permet — mais le fil, lui, en retient un autre. Afficher `maxDistanceKm`
    /// seul montrerait « 27 km » à quelqu'un dont le fil en retient 25, sans
    /// jamais le lui dire.
    public let effectiveDistanceKm: Int

    /// Le réglage choisi diffère-t-il de ce qui est appliqué ?
    public var distanceRabattue: Bool { effectiveDistanceKm != maxDistanceKm }
    public let seeking: [Gender]
    public let categories: [PlanCategory]
    /// Jours retenus, au sens ISO : 1 lundi, 7 dimanche. Vide = tous.
    public let days: [Int]
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
