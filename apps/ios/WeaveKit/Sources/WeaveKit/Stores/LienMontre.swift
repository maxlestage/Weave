#if canImport(WatchConnectivity)
import Foundation
import WatchConnectivity

/// Le transport entre l'iPhone et la montre : WatchConnectivity.
///
/// Ce fichier ne fait que transporter. Ce qui part et ce qui arrive est défini
/// par `MessageMontre`, éprouvé ailleurs ; ce que la montre en fait, par
/// `MontreStore`, éprouvé aussi. `WCSession` n'existe que sur les appareils
/// d'Apple — d'où le garde, et d'où la minceur de ce qu'il protège.
///
/// ## Trois façons d'envoyer, et pourquoi chacune
///
/// - `transferUserInfo` pour ce qui DOIT arriver — une session, une
///   déconnexion. Le système le met en file et le remet même si l'autre
///   application ne tourne pas.
/// - `updateApplicationContext` pour le résumé : seul le dernier compte, les
///   précédents n'ont plus d'intérêt et le système les écrase.
/// - `sendMessage` pour la demande de session de la montre, quand l'iPhone est
///   joignable tout de suite ; sinon, elle part elle aussi en file.
@MainActor
public final class LienMontre: NSObject {
    private let surMessage: @MainActor (MessageMontre) async -> Void

    /// `surMessage` reçoit ce que l'autre appareil envoie, déjà relu.
    public init(surMessage: @escaping @MainActor (MessageMontre) async -> Void) {
        self.surMessage = surMessage
        super.init()
    }

    public func demarrer() {
        guard WCSession.isSupported() else { return }
        let session = WCSession.default
        session.delegate = self
        session.activate()
    }

    /// Une montre peut-elle recevoir ? Sur l'iPhone : appairée ET avec
    /// l'application installée. Sur la montre, l'iPhone est toujours là.
    public var autreAppareilPresent: Bool {
        guard WCSession.isSupported() else { return false }
        let session = WCSession.default
        guard session.activationState == .activated else { return false }
        #if os(iOS)
        return session.isPaired && session.isWatchAppInstalled
        #else
        return true
        #endif
    }

    public func envoyer(_ message: MessageMontre) {
        guard autreAppareilPresent, let dictionnaire = try? message.dictionnaire() else { return }
        let session = WCSession.default
        switch message {
        case .resume:
            try? session.updateApplicationContext(dictionnaire)
        case .demandeDeSession where session.isReachable:
            session.sendMessage(dictionnaire, replyHandler: nil) { _ in
                // Joignable à l'instant, plus maintenant : la file prend le
                // relais, et la demande arrivera au prochain réveil.
                //
                // Le dictionnaire est RÉÉCRIT ici plutôt que capturé : un
                // `[String: Any]` ne traverse pas une frontière de concurrence,
                // le message — `Sendable` — si.
                if let encore = try? message.dictionnaire() {
                    WCSession.default.transferUserInfo(encore)
                }
            }
        case .session, .deconnexion, .demandeDeSession:
            session.transferUserInfo(dictionnaire)
        }
    }

    /// Les messages arrivent sur une file du système, pas sur l'acteur
    /// principal : ils sont relus ici, puis remis à qui les attend.
    nonisolated private func relayer(_ dictionnaire: [String: Any]) {
        guard let message = MessageMontre(dictionnaire) else { return }
        Task { @MainActor in await self.surMessage(message) }
    }
}

extension LienMontre: WCSessionDelegate {
    nonisolated public func session(
        _ session: WCSession,
        activationDidCompleteWith activationState: WCSessionActivationState,
        error: (any Error)?
    ) {
        // Un contexte reçu pendant que l'application était fermée attend ici.
        let enAttente = session.receivedApplicationContext
        if !enAttente.isEmpty { relayer(enAttente) }
    }

    #if os(iOS)
    nonisolated public func sessionDidBecomeInactive(_ session: WCSession) {}

    /// Changement de montre appairée : on se réactive pour la nouvelle.
    nonisolated public func sessionDidDeactivate(_ session: WCSession) {
        session.activate()
    }
    #endif

    nonisolated public func session(
        _ session: WCSession,
        didReceiveUserInfo userInfo: [String: Any]
    ) {
        relayer(userInfo)
    }

    nonisolated public func session(
        _ session: WCSession,
        didReceiveApplicationContext applicationContext: [String: Any]
    ) {
        relayer(applicationContext)
    }

    nonisolated public func session(
        _ session: WCSession,
        didReceiveMessage message: [String: Any]
    ) {
        relayer(message)
    }
}
#endif
