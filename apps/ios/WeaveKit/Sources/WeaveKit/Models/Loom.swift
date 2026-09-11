import Foundation

/// État d'un fil, du plus froid au plus engagé.
///
/// Les valeurs brutes correspondent exactement à celles de l'API
/// (`packages/contracts/src/domain.ts`) : le vocabulaire du produit est le même
/// des deux côtés du réseau.
public enum ThreadState: String, Codable, Sendable {
    /// Tissé, jamais engagé.
    case propose
    /// Au moins une réponse envoyée.
    case engage
    /// Réponse mutuelle : la conversation est ouverte.
    case tisse
    /// Expiré ou relâché.
    case denoue
}

public enum FragmentKind: String, Codable, Sendable {
    case question
    case voix
    case motif
}

/// Un élément de la trame d'un fil : une question et sa réponse.
public struct Fragment: Codable, Identifiable, Hashable, Sendable {
    public let id: String
    public let kind: FragmentKind
    public let prompt: String
    public let body: String
    public let durationSeconds: Int?

    public init(id: String, kind: FragmentKind, prompt: String, body: String, durationSeconds: Int? = nil) {
        self.id = id
        self.kind = kind
        self.prompt = prompt
        self.body = body
        self.durationSeconds = durationSeconds
    }
}

/// Un fil du métier.
///
/// Cette structure n'est jamais persistée sur l'appareil : elle vit le temps
/// d'une session, exactement comme sa contrepartie côté serveur ne vit que dans
/// le cache. C'est la même règle appliquée des deux côtés.
public struct ThreadCard: Codable, Identifiable, Hashable, Sendable {
    public let id: String
    public let state: ThreadState
    public let displayName: String
    public let age: Int
    public let distanceKm: Int
    public let city: String
    public let motif: [String]
    public let fragments: [Fragment]
    /// Netteté de la photo : 0, 33, 66 ou 100.
    public let revealPercent: Int
    public let photoURL: URL?
    public let expiresAt: Date
    public let exchanges: Int
    public let awaitingYou: Bool

    private enum CodingKeys: String, CodingKey {
        case id, state, displayName, age, distanceKm, city, motif, fragments
        case revealPercent
        case photoURL = "photoUrl"
        case expiresAt, exchanges, awaitingYou
    }

    /// Temps restant avant dénouage. Nul si le fil est déjà échu.
    public var timeRemaining: TimeInterval {
        max(0, expiresAt.timeIntervalSinceNow)
    }

    public var isExpired: Bool { timeRemaining <= 0 }

    /// Rayon de flou à appliquer localement, en points.
    ///
    /// Le serveur floute déjà l'image qu'il renvoie ; ce flou-ci n'est qu'un
    /// raccord visuel, jamais une protection. Ce qui protège, c'est que l'image
    /// nette n'a pas quitté le serveur.
    public var blurRadius: Double {
        Double(100 - revealPercent) / 100.0 * 18.0
    }
}

/// Le métier : un nombre fixe de fils, et de quoi savoir quand la suite arrive.
public struct Loom: Codable, Hashable, Sendable {
    /// Invariant produit, identique côté serveur (`MAX_ACTIVE_THREADS`).
    public static let maxActiveThreads = 12

    public let threads: [ThreadCard]
    public let nextWeavingAt: Date
    public let freeSlots: Int
    public let nextRefillAt: Date?
    public let fromCache: Bool

    public static let empty = Loom(
        threads: [],
        nextWeavingAt: .distantFuture,
        freeSlots: maxActiveThreads,
        nextRefillAt: nil,
        fromCache: true
    )

    public init(
        threads: [ThreadCard],
        nextWeavingAt: Date,
        freeSlots: Int,
        nextRefillAt: Date?,
        fromCache: Bool
    ) {
        // Le plafond est une garantie du produit, pas une supposition : on le
        // fait respecter aussi à la réception, au cas où l'API dérive un jour.
        // `maxActiveThreads` doit rester égal à `MAX_ACTIVE_THREADS` côté
        // serveur — les tests de WeaveKit le vérifient sur des charges utiles
        // réelles.
        self.threads = Array(threads.prefix(Self.maxActiveThreads))
        self.nextWeavingAt = nextWeavingAt
        self.freeSlots = freeSlots
        self.nextRefillAt = nextRefillAt
        self.fromCache = fromCache
    }

    public var awaitingYou: [ThreadCard] { threads.filter(\.awaitingYou) }

    /// Fil le plus proche du dénouage : celui que l'on met en avant partout.
    public var soonest: ThreadCard? {
        threads.min { $0.expiresAt < $1.expiresAt }
    }
}
