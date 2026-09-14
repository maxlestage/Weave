import Foundation

/// Catégorie d'un plan. Les valeurs brutes correspondent exactement à celles de
/// l'API (`packages/contracts/src/invariants.ts`) : le vocabulaire du produit
/// est le même des deux côtés du réseau.
public enum PlanCategory: String, Codable, CaseIterable, Sendable {
    case sortie
    case sport
    case culture
    case repas
    case musique
    case jeux
    case balade
    case benevolat

    public var displayName: String {
        switch self {
        case .sortie: "Sortir"
        case .sport: "Bouger"
        case .culture: "Voir quelque chose"
        case .repas: "Manger"
        case .musique: "Écouter"
        case .jeux: "Jouer"
        case .balade: "Marcher"
        case .benevolat: "Donner un coup de main"
        }
    }

    public var symbolName: String {
        switch self {
        case .sortie: "sparkles"
        case .sport: "figure.run"
        case .culture: "theatermasks"
        case .repas: "fork.knife"
        case .musique: "music.note"
        case .jeux: "dice"
        case .balade: "figure.walk"
        case .benevolat: "hands.and.sparkles"
        }
    }
}

public enum PlanState: String, Codable, Sendable {
    /// Publié, des places restent.
    case ouvert
    /// Toutes les places sont prises.
    case complet
    /// L'heure du rendez-vous est dépassée.
    case passe
    case annule
    /// Son auteur est en pause : le plan est retiré du fil des autres, et
    /// revient tel quel à la reprise.
    ///
    /// Ce cas manquait, et l'énumération n'est pas facultative : l'API rendant
    /// « suspendu » pour tout plan d'un compte en pause, le décodage de
    /// « Mes plans » échouait d'un bloc. L'écran entier tombait, pour un état
    /// que le contrat ne déclarait pas non plus.
    case suspendu
}

/// Auteur d'un plan, tel qu'affiché dans le fil. Volontairement maigre : c'est
/// le plan qui porte la substance, pas la fiche.
public struct Author: Codable, Identifiable, Hashable, Sendable {
    public let id: String
    public let displayName: String
    public let age: Int
    public let photoURL: URL?
    public let verified: Bool

    private enum CodingKeys: String, CodingKey {
        case id, displayName, age
        case photoURL = "photoUrl"
        case verified
    }
}

/// Un plan : ce que quelqu'un compte faire, et à quoi d'autres peuvent se joindre.
public struct Plan: Codable, Identifiable, Hashable, Sendable {
    public let id: String
    public let author: Author
    public let title: String
    public let note: String
    public let category: PlanCategory
    public let startsAt: Date
    public let city: String
    /// Distance arrondie. Weave n'expose jamais de position précise.
    public let distanceKm: Int
    public let capacity: Int
    public let seatsLeft: Int
    public let state: PlanState
    /// Vrai si l'on a déjà demandé à venir. On ne redemande pas deux fois.
    public let requested: Bool
    public let createdAt: Date

    public var isJoinable: Bool {
        state == .ouvert && seatsLeft > 0 && !requested && startsAt > .now
    }
}

/// Le fil : les plans à venir, autour de soi.
///
/// L'ordre vient du serveur et n'est jamais retrié ici : imminence puis
/// proximité, et rien d'autre. Un tri local, même « utile », introduirait un
/// classement que personne n'a annoncé.
public struct Feed: Codable, Hashable, Sendable {
    public let plans: [Plan]
    /// Demandes restantes aujourd'hui. C'est l'invariant central du produit :
    /// on ne peut pas arroser.
    public let requestsLeftToday: Int
    public let fromCache: Bool
    public let generatedAt: Date

    public static let empty = Feed(
        plans: [],
        requestsLeftToday: 0,
        fromCache: true,
        generatedAt: .distantPast
    )

    public init(plans: [Plan], requestsLeftToday: Int, fromCache: Bool, generatedAt: Date) {
        self.plans = plans
        self.requestsLeftToday = requestsLeftToday
        self.fromCache = fromCache
        self.generatedAt = generatedAt
    }

    public var soonest: Plan? { plans.first }
}

/// Un de ses propres plans, avec ce qu'il a suscité.
public struct MyPlan: Codable, Identifiable, Hashable, Sendable {
    /// Plafond de plans ouverts, identique côté serveur (`MAX_OPEN_PLANS`).
    public static let maxOpen = 3

    public let id: String
    public let title: String
    public let note: String
    public let category: PlanCategory
    public let startsAt: Date
    public let city: String
    public let capacity: Int
    public let seatsLeft: Int
    public let state: PlanState
    public let pendingRequests: Int
}

// MARK: - Demandes

public enum RequestState: String, Codable, Sendable {
    case envoyee
    case acceptee
    case refusee
    case expiree
    case retiree

    public var displayName: String {
        switch self {
        case .envoyee: "En attente"
        case .acceptee: "Acceptée"
        case .refusee: "Sans suite"
        case .expiree: "Close"
        case .retiree: "Retirée"
        }
    }
}

/// Une demande de rejoindre un plan. Elle contient un message : c'est
/// l'écriture qui engage, jamais un geste.
public struct JoinRequest: Codable, Identifiable, Hashable, Sendable {
    /// Longueur minimale d'un message, identique côté serveur.
    public static let minimumMessageLength = 20
    public static let maximumMessageLength = 600

    public let id: String
    public let planId: String
    public let planTitle: String
    public let planStartsAt: Date
    public let author: Author
    public let message: String
    public let state: RequestState
    public let sentAt: Date
    public let decidedAt: Date?
    public let conversationId: String?
}

/// Une demande reçue sur un de ses plans.
public struct IncomingRequest: Codable, Identifiable, Hashable, Sendable {
    public let id: String
    public let message: String
    public let sentAt: Date
    public let author: Author
}

// MARK: - Conversations

public struct Conversation: Codable, Identifiable, Hashable, Sendable {
    public let id: String
    public let planId: String
    public let planTitle: String
    public let planStartsAt: Date
    public let other: Author
    public let lastMessage: String?
    public let lastMessageAt: Date?
    public let unread: Int
    public let closed: Bool
}

public enum MessageAuthor: String, Codable, Sendable {
    case moi
    case autre
    case systeme
}

/// Longueur maximale d'un message de conversation.
///
/// Le serveur refuse au-delà. L'application l'ignorait : on pouvait écrire
/// sans fin, et perdre son texte à l'envoi.
public let conversationMaxChars = 2000

public struct Message: Codable, Identifiable, Hashable, Sendable {
    public let id: String
    public let conversationId: String
    public let author: MessageAuthor
    public let body: String
    public let sentAt: Date
    public let readAt: Date?
}

// MARK: - Mise en forme

extension Date {
    /// « demain à 19 h », « jeudi à 10 h » — la forme qu'on emploierait à l'oral.
    public var weaveWhenLabel: String {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "fr_FR")
        formatter.doesRelativeDateFormatting = true
        formatter.dateStyle = .full
        formatter.timeStyle = .short
        return formatter.string(from: self)
    }
}
