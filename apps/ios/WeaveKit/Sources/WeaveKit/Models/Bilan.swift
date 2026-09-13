import Foundation

/// Le « Bilan » : ce qui attire, ce qui tombe à plat.
///
/// Rien n'y est comparé aux autres, et aucun message de demande n'y est lu :
/// c'est une lecture de ses propres plans, pas un classement.
public struct Bilan: Decodable, Sendable, Identifiable {
    /// La date d'établissement fait l'identité : un bilan est un instantané,
    /// et deux bilans du même compte à deux moments sont deux choses.
    public var id: Date { etabliLe }

    public let etabliLe: Date
    public let periode: Periode
    public let plansPasses: Int
    public let demandesRecues: Int
    public let demandesAcceptees: Int
    public let plansSansAucuneDemande: Int
    public let parCategorie: [RendementCategorie]
    public let cequiAttire: [PlanCite]
    public let ceQuiTombeAPlat: [PlanCite]
    public let delai: Delai

    public struct Periode: Decodable, Sendable {
        public let duPremierPlan: Date
        public let auDernier: Date
    }

    public struct RendementCategorie: Decodable, Sendable, Identifiable {
        public let categorie: PlanCategory
        public let plans: Int
        public let demandes: Int
        public let demandesParPlan: Double

        public var id: String { categorie.rawValue }
    }

    public struct PlanCite: Decodable, Sendable, Identifiable {
        public let titre: String
        public let categorie: PlanCategory
        public let demandes: Int
        public let publieJoursAvant: Int

        public var id: String { titre }
    }

    /// Le délai de publication, comparé entre ce qui a pris et ce qui n'a rien
    /// eu. `nil` d'un côté quand il n'y a rien à moyenner.
    public struct Delai: Decodable, Sendable {
        public let joursAvantQuandCaPrend: Double?
        public let joursAvantQuandCaNePrendPas: Double?
    }
}
