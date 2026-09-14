#if canImport(ActivityKit)
import ActivityKit
#endif
import Foundation
import Observation

/// Pilote la Live Activity « Prochain plan ».
///
/// Deux jetons entrent en jeu, et il faut les distinguer :
///
///  • le jeton « push-to-start », propre à l'appareil, permet au serveur de
///    DÉMARRER une activité à distance — c'est lui qui fait apparaître la
///    bannière quand quelqu'un demande à venir, application fermée ;
///  • le jeton de mise à jour, propre à une activité en cours, permet d'en
///    modifier le contenu.
///
/// Les deux peuvent être renouvelés par le système à tout moment : on écoute
/// leurs flux pour la durée de vie du contrôleur, jamais une seule fois.
@MainActor
@Observable
public final class ActivityController {
    public private(set) var isRunning = false
    public private(set) var pushToStartToken: String?

    private let api: WeaveAPI
    private let vendorID: String
    private var observationTasks: [Task<Void, Never>] = []

    public init(api: WeaveAPI, vendorID: String) {
        self.api = api
        self.vendorID = vendorID
    }

    /// `isolated deinit` — sans quoi la classe ne compile pas.
    ///
    /// La classe est `@MainActor` : ses propriétés stockées sont isolées.
    /// Un `deinit` ordinaire, lui, ne l'est jamais — il s'exécute sur le fil
    /// qui relâche la dernière référence, quel qu'il soit. Lire
    /// `observationTasks` depuis là est refusé par le mode Swift 6, que ce
    /// paquet demande explicitement.
    isolated deinit {
        for task in observationTasks { task.cancel() }
    }

    #if canImport(ActivityKit)

    /// Vrai si la personne autorise les Live Activities dans les réglages.
    public var isEnabled: Bool {
        ActivityAuthorizationInfo().areActivitiesEnabled
    }

    /// À appeler une fois au démarrage : met en place l'écoute des jetons et
    /// reprend la main sur une activité déjà en cours.
    public func start() {
        guard observationTasks.isEmpty else { return }

        observationTasks.append(Task { [weak self] in
            await self?.observePushToStartTokens()
        })
        observationTasks.append(Task { [weak self] in
            await self?.adoptRunningActivities()
        })
    }

    /// Démarre l'activité localement — au retour au premier plan, quand il y a
    /// un plan à venir mais qu'aucune bannière n'est affichée.
    public func startLocally(handle: String, state: WeaveActivityAttributes.ContentState) async {
        guard isEnabled, !isRunning, !state.isIdle else { return }

        do {
            let activity = try Activity.request(
                attributes: WeaveActivityAttributes(accountHandle: handle),
                content: .init(state: state, staleDate: state.planStartsAt),
                pushType: .token
            )
            isRunning = true
            observationTasks.append(Task { [weak self] in
                await self?.observeUpdateTokens(of: activity)
            })
        } catch {
            // Un refus n'est pas une panne : l'application reste utilisable
            // sans Live Activity, elle perd seulement le rappel discret.
            isRunning = false
        }
    }

    /// Termine les activités en cours (déconnexion, mise en pause du compte).
    public func end() async {
        for activity in Activity<WeaveActivityAttributes>.activities {
            await activity.end(nil, dismissalPolicy: .immediate)
            if let token = activity.pushToken?.hexString {
                try? await api.endActivity(updateToken: token)
            }
        }
        isRunning = false
    }

    // MARK: - Écoutes

    private func observePushToStartTokens() async {
        for await tokenData in Activity<WeaveActivityAttributes>.pushToStartTokenUpdates {
            let token = tokenData.hexString
            pushToStartToken = token
            try? await api.registerDevice(
                DeviceRegistration(
                    vendorId: vendorID,
                    platform: "ios",
                    pushToStartToken: token,
                    apnsEnvironment: Self.apnsEnvironment
                )
            )
        }
    }

    private func adoptRunningActivities() async {
        for activity in Activity<WeaveActivityAttributes>.activities {
            isRunning = true
            observationTasks.append(Task { [weak self] in
                await self?.observeUpdateTokens(of: activity)
            })
        }
    }

    private func observeUpdateTokens(of activity: Activity<WeaveActivityAttributes>) async {
        for await tokenData in activity.pushTokenUpdates {
            try? await api.registerActivity(vendorID: vendorID, updateToken: tokenData.hexString)
        }
    }

    /// APNs sandbox pour les builds de développement, production sinon.
    private static var apnsEnvironment: String {
        #if DEBUG
        "sandbox"
        #else
        "production"
        #endif
    }

    #else

    public var isEnabled: Bool { false }
    public func start() {}
    public func startLocally(handle: String, state: WeaveActivityAttributes.ContentState) async {}
    public func end() async {}

    #endif
}

extension Data {
    /// Représentation hexadécimale attendue par APNs pour un jeton.
    var hexString: String {
        map { String(format: "%02x", $0) }.joined()
    }
}
