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

    /// Le même plan, marqué comme demandé.
    ///
    /// Le magasin le rebâtissait champ par champ — treize arguments recopiés à
    /// la main pour n'en changer qu'un. Le compilateur exige qu'ils soient tous
    /// là, mais rien n'empêche d'en INTERVERTIR deux de même type : `capacity`
    /// et `seatsLeft` sont deux entiers, `title`, `note` et `city` trois
    /// chaînes. Un plan complet se serait affiché avec des places libres, et
    /// une demande aurait été possible dessus.
    public func demande() -> Plan {
        Plan(
            id: id,
            author: author,
            title: title,
            note: note,
            category: category,
            startsAt: startsAt,
            city: city,
            distanceKm: distanceKm,
            capacity: capacity,
            seatsLeft: seatsLeft,
            state: state,
            requested: true,
            createdAt: createdAt
        )
    }
}

/// Ce qu'on corrige sur un plan déjà publié.
///
/// Tous les champs sont facultatifs : on n'envoie que ce qui change. La
/// catégorie et la ville n'y figurent pas, et c'est délibéré — les changer ne
/// corrige pas un plan, cela en fait un autre, auquel des gens ont dit oui
/// sans le connaître. Un autre plan se publie.
public struct PlanEdit: Encodable, Sendable {
    public let title: String?
    public let note: String?
    public let startsAt: Date?
    public let capacity: Int?

    public init(
        title: String? = nil,
        note: String? = nil,
        startsAt: Date? = nil,
        capacity: Int? = nil
    ) {
        self.title = title
        self.note = note
        self.startsAt = startsAt
        self.capacity = capacity
    }

    /// Y a-t-il seulement quelque chose à envoyer ?
    ///
    /// Une requête vide aboutirait — le serveur accepte un ajustement sans
    /// champ — mais elle ferait croire à une modification qui n'a pas eu lieu,
    /// et elle invaliderait le cache du fil pour rien.
    public var vide: Bool {
        title == nil && note == nil && startsAt == nil && capacity == nil
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

    /// Le même fil, avec d'autres plans — et le reste conservé.
    ///
    /// Trois endroits du magasin rebâtissaient un `Feed` champ par champ pour
    /// n'en changer qu'un. `fromCache` y devient vrai : ce qu'on tient ne vient
    /// plus du serveur, il a été retouché ici, et l'écran doit pouvoir le dire.
    /// `generatedAt` ne bouge pas — c'est l'heure de composition du fil, et la
    /// retoucher ferait croire à une composition qui n'a pas eu lieu.
    public func remplacant(plans: [Plan], requestsLeftToday: Int? = nil) -> Feed {
        Feed(
            plans: plans,
            requestsLeftToday: requestsLeftToday ?? self.requestsLeftToday,
            fromCache: true,
            generatedAt: generatedAt
        )
    }
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

    /// Les places déjà accordées.
    ///
    /// Déduite plutôt que reçue : le serveur rend la capacité et ce qu'il
    /// reste, et leur différence est exactement ce qui a été donné. L'écran de
    /// modification s'en sert pour ne pas proposer de reprendre la parole à
    /// quelqu'un — le serveur refuse de toute façon, mais un bouton qu'on
    /// presse pour lire un refus est un bouton mal fait.
    public var seatsAccordees: Int { max(capacity - seatsLeft, 0) }
}

// MARK: - Demandes

/// L'état d'une demande. `packages/contracts` fait foi : `REQUEST_STATES`.
///
/// Un état que le serveur écrit et que ce type ignore ne casse pas une ligne :
/// il casse la RÉPONSE ENTIÈRE, puisque Swift échoue à décoder une énumération
/// sans cas correspondant. C'est déjà arrivé sur l'état d'un plan, et « Mes
/// plans » ne s'ouvrait plus. Un test de contrat rapproche donc les deux listes.
public enum RequestState: String, Codable, Sendable, CaseIterable {
    case envoyee
    case acceptee
    case refusee
    case expiree
    case retiree
    /// La place avait été accordée, et elle a été rendue.
    case desistee

    public var displayName: String {
        switch self {
        case .envoyee: "En attente"
        case .acceptee: "Acceptée"
        case .refusee: "Sans suite"
        case .expiree: "Close"
        case .retiree: "Retirée"
        case .desistee: "Place rendue"
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
