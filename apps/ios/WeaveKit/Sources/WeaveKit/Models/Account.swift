import Foundation

public enum AccountStatus: String, Codable, Sendable {
    case onboarding
    case active
    case paused
    case suspended
    case deleting
}

/// Palier d'abonnement. Miroir de `PlanTier` côté contrats partagés.
public enum PlanTier: String, Codable, CaseIterable, Sendable {
    case fil
    case trame
    case chaine
    case navette
    case metier

    public var displayName: String {
        switch self {
        case .fil: "Fil"
        case .trame: "Trame"
        case .chaine: "Chaîne"
        case .navette: "Navette"
        case .metier: "Métier"
        }
    }
}

/// Produit achetable à l'unité. Miroir de `UnitSku`.
public enum UnitSku: String, Codable, CaseIterable, Sendable {
    case echo
    case prolonge
    case relais
    case motif
    case escale
    case atelier

    public var displayName: String {
        switch self {
        case .echo: "Écho"
        case .prolonge: "Prolonge"
        case .relais: "Relais"
        case .motif: "Motif"
        case .escale: "Escale"
        case .atelier: "Atelier"
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
    public let status: AccountStatus
    public let plan: PlanTier
    public let weavingHour: Int
    public let timezone: String
    public let verified: Bool
    public let motif: [String]
    public let credits: [String: Int]
    public let createdAt: Date

    public func credits(for sku: UnitSku) -> Int {
        credits[sku.rawValue] ?? 0
    }
}

public struct Entitlement: Codable, Sendable {
    public let plan: PlanTier
    public let renewsAt: Date?
    public let credits: [String: Int]
    public let inGracePeriod: Bool
}
