#if canImport(ActivityKit)
import ActivityKit
#endif
import Foundation

/// Attributs de la Live Activity « Prochain plan ».
///
/// Le nom de ce type est repris tel quel par le serveur dans la charge utile
/// APNs (`attributes-type`) : le renommer casse le démarrage à distance. Il est
/// défini dans WeaveKit parce que l'application, l'extension widget et — pour
/// les tests — la cible de test doivent toutes en partager la définition.
///
/// L'état dynamique ne contient qu'un titre, une date et deux compteurs. Rien
/// de ce qui s'affiche sur un écran verrouillé ne doit trahir avec qui l'on
/// parle : pas de nom, pas de photo, pas de message.
public struct WeaveActivityAttributes: Sendable, Codable, Hashable {
    public struct ContentState: Sendable, Codable, Hashable {
        /// Titre du plan le plus proche, publié ou rejoint.
        public let planTitle: String?
        public let planStartsAt: Date?
        /// Demandes reçues sur ses propres plans, encore sans décision.
        public let pendingRequests: Int
        /// Demandes envoyées, encore sans réponse.
        public let awaitingReply: Int
        public let updatedAt: Date

        public init(
            planTitle: String?,
            planStartsAt: Date?,
            pendingRequests: Int,
            awaitingReply: Int,
            updatedAt: Date = .now
        ) {
            self.planTitle = planTitle
            self.planStartsAt = planStartsAt
            self.pendingRequests = pendingRequests
            self.awaitingReply = awaitingReply
            self.updatedAt = updatedAt
        }

        /// Intervalle à passer à `Text(timerInterval:)` pour un décompte animé
        /// sans réveiller l'application.
        public var countdown: ClosedRange<Date>? {
            guard let planStartsAt, planStartsAt > .now else { return nil }
            return Date.now...planStartsAt
        }

        /// Ce qu'on lit d'un coup d'œil. Les demandes reçues passent devant le
        /// rendez-vous : c'est la seule chose qui attend une action.
        public var summary: String {
            if pendingRequests > 0 {
                return pendingRequests > 1
                    ? "\(pendingRequests) personnes veulent venir"
                    : "Quelqu'un veut venir"
            }
            if let planTitle { return planTitle }
            if awaitingReply > 0 {
                return awaitingReply > 1
                    ? "\(awaitingReply) demandes en attente"
                    : "Une demande en attente"
            }
            return "Aucun plan à venir"
        }

        /// Vrai quand il n'y a plus rien à montrer : le serveur termine alors
        /// l'activité plutôt que d'afficher une bannière vide.
        public var isIdle: Bool {
            planTitle == nil && pendingRequests == 0 && awaitingReply == 0
        }

        public static let idle = ContentState(
            planTitle: nil,
            planStartsAt: nil,
            pendingRequests: 0,
            awaitingReply: 0
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

/// Résumé compact destiné à watchOS : quelques centaines d'octets, pas plus.
public struct WatchSummary: Codable, Sendable, Hashable {
    public struct NextPlan: Codable, Sendable, Hashable {
        public let title: String
        public let startsAt: Date
        public let city: String
    }

    public let pendingRequests: Int
    public let awaitingReply: Int
    public let nextPlan: NextPlan?
    public let generatedAt: Date

    public static let empty = WatchSummary(
        pendingRequests: 0,
        awaitingReply: 0,
        nextPlan: nil,
        generatedAt: .distantPast
    )
}
