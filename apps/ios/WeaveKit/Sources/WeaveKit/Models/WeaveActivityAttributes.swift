#if canImport(ActivityKit)
import ActivityKit
#endif
import Foundation

/// Attributs de la Live Activity « Métier ».
///
/// Le nom de ce type est repris tel quel par le serveur dans la charge utile
/// APNs (`attributes-type`) : le renommer casse le démarrage à distance. Il est
/// défini dans WeaveKit parce que l'application, l'extension widget et — pour
/// les tests — la cible de test doivent toutes en partager la définition.
///
/// L'état dynamique ne contient que des compteurs et des dates. Rien de ce qui
/// s'affiche sur un écran verrouillé ne doit trahir avec qui l'on parle : un
/// prénom, une échéance, c'est tout.
public struct WeaveActivityAttributes: Sendable, Codable, Hashable {
    public struct ContentState: Sendable, Codable, Hashable {
        public let activeThreads: Int
        public let awaitingYou: Int
        public let soonestExpiryAt: Date?
        public let soonestName: String?
        public let nextRefillAt: Date?
        public let updatedAt: Date

        public init(
            activeThreads: Int,
            awaitingYou: Int,
            soonestExpiryAt: Date?,
            soonestName: String?,
            nextRefillAt: Date?,
            updatedAt: Date = .now
        ) {
            self.activeThreads = activeThreads
            self.awaitingYou = awaitingYou
            self.soonestExpiryAt = soonestExpiryAt
            self.soonestName = soonestName
            self.nextRefillAt = nextRefillAt
            self.updatedAt = updatedAt
        }

        /// Intervalle à passer à `Text(timerInterval:)` pour un décompte animé
        /// sans réveiller l'application.
        public var countdown: ClosedRange<Date>? {
            guard let soonestExpiryAt, soonestExpiryAt > .now else { return nil }
            return Date.now...soonestExpiryAt
        }

        public var summary: String {
            switch (activeThreads, awaitingYou) {
            case (0, _): "Aucun fil sur le métier"
            case (let total, 0): "\(total) fil\(total > 1 ? "s" : "") en cours"
            case (_, let waiting): "\(waiting) fil\(waiting > 1 ? "s" : "") attend\(waiting > 1 ? "ent" : "") votre réponse"
            }
        }

        public static let idle = ContentState(
            activeThreads: 0,
            awaitingYou: 0,
            soonestExpiryAt: nil,
            soonestName: nil,
            nextRefillAt: nil
        )
    }

    /// Identifiant court du compte, utile pour distinguer deux activités sur un
    /// appareil partagé. Volontairement non identifiant en soi.
    public let accountHandle: String

    public init(accountHandle: String) {
        self.accountHandle = accountHandle
    }
}

#if canImport(ActivityKit)
extension WeaveActivityAttributes: ActivityAttributes {}
#endif

/// Résumé compact destiné à watchOS.
public struct WatchSummary: Codable, Sendable, Hashable {
    public struct Entry: Codable, Sendable, Hashable, Identifiable {
        public let id: String
        public let name: String
        public let expiresAt: Date
        public let awaitingYou: Bool
    }

    public let activeThreads: Int
    public let awaitingYou: Int
    public let soonestExpiryAt: Date?
    public let entries: [Entry]
    public let generatedAt: Date

    public static let empty = WatchSummary(
        activeThreads: 0,
        awaitingYou: 0,
        soonestExpiryAt: nil,
        entries: [],
        generatedAt: .distantPast
    )
}
