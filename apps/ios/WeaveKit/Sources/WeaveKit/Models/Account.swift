import Foundation

/// Délai entre la demande de suppression et l'effacement réel, en jours.
///
/// Miroir manuel de `ACCOUNT_PURGE_DAYS` (`packages/contracts/src/invariants.ts`),
/// comme les autres valeurs partagées : on ne transporte pas de types entre
/// TypeScript et Swift. La valeur est affichée à qui demande la suppression —
/// annoncer autre chose que ce que le serveur applique serait mentir sur un
/// délai que la politique de confidentialité engage.
public let accountPurgeDays = 30

public enum AccountStatus: String, Codable, Sendable {
    case onboarding
    case active
    case paused
    case suspended
    case deleting
}

/// Palier d'abonnement. Miroir de `PlanTier` côté contrats partagés.
///
/// Aucun palier n'achète de visibilité : payer ne fait jamais remonter un plan.
/// Ce qui se paie, c'est l'horizon de publication, la finesse des critères et
/// les plans de groupe.
public enum PlanTier: String, Codable, CaseIterable, Sendable {
    case depart
    case viree
    case escapade
    case expedition
    case grandtour

    public var displayName: String {
        switch self {
        case .depart: "Départ"
        case .viree: "Virée"
        case .escapade: "Escapade"
        case .expedition: "Expédition"
        case .grandtour: "Grand Tour"
        }
    }
}

/// Produit achetable à l'unité. Miroir de `UnitSku`.
///
/// Il n'existe volontairement aucun produit de « remontée » : ce serait vendre
/// de la visibilité, ce que Weave s'interdit.
public enum UnitSku: String, Codable, CaseIterable, Sendable {
    case renfort
    case horizon
    case tablee
    case escale
    case bilan

    public var displayName: String {
        switch self {
        case .renfort: "Renfort"
        case .horizon: "Horizon"
        case .tablee: "Tablée"
        case .escale: "Escale"
        case .bilan: "Bilan"
        }
    }

    /// Identifiant StoreKit correspondant, aligné sur le catalogue serveur.
    public var productID: String { "com.weave.app.unit.\(rawValue)" }
}

public struct Session: Codable, Sendable {
    public let accessToken: String
    public let refreshToken: String
    public let expiresAt: Date

    public init(accessToken: String, refreshToken: String, expiresAt: Date) {
        self.accessToken = accessToken
        self.refreshToken = refreshToken
        self.expiresAt = expiresAt
    }

    /// Vrai lorsqu'il faut renouveler : on anticipe d'une minute pour ne pas
    /// se faire refuser une requête en vol.
    public var needsRefresh: Bool {
        expiresAt.timeIntervalSinceNow < 60
    }
}

public struct Me: Codable, Sendable {
    public let id: String
    public let handle: String
    public let displayName: String
    public let age: Int
    public let status: AccountStatus
    public let tier: PlanTier
    public let city: String
    public let bio: String
    public let photoURL: URL?
    public let verified: Bool
    /// Demandes restantes aujourd'hui.
    public let requestsLeftToday: Int
    public let credits: [String: Int]
    public let createdAt: Date

    private enum CodingKeys: String, CodingKey {
        case id, handle, displayName, age, status, tier, city, bio
        case photoURL = "photoUrl"
        case verified, requestsLeftToday, credits, createdAt
    }

    public func credits(for sku: UnitSku) -> Int {
        credits[sku.rawValue] ?? 0
    }
}

public struct Entitlement: Codable, Sendable {
    public let tier: PlanTier
    public let renewsAt: Date?
    public let credits: [String: Int]
    public let requestsLeftToday: Int
    public let inGracePeriod: Bool
}
