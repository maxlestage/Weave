#if os(iOS)
import Foundation
import UIKit
import UserNotifications

/// Les notifications d'alerte : demander l'autorisation, obtenir le jeton, le
/// transmettre.
///
/// ## Ce qui manquait
///
/// L'application n'a jamais demandé l'autorisation de notifier, et le serveur
/// stockait un `apnsToken` que personne ne lui envoyait — un champ prévu,
/// resté vide des deux côtés. Un message arrivait donc sans que personne ne
/// l'apprenne : la Live Activity ne compte que des plans et des demandes,
/// jamais une conversation.
///
/// ## Ce qu'on demande, et quand
///
/// L'autorisation n'est pas réclamée au premier lancement. Une demande
/// d'autorisation qui tombe avant qu'on ait compris à quoi sert l'application
/// se refuse par réflexe, et iOS ne la repose jamais : on ne dispose que d'une
/// seule occasion. Elle est donc demandée au moment où elle a un sens — à la
/// connexion, quand il y a désormais quelque chose à notifier.
///
/// Aucune alerte ne cite jamais un message ni un prénom : c'est la règle tenue
/// côté serveur, et elle est rappelée ici parce que c'est ici qu'on serait
/// tenté d'en afficher davantage.
@MainActor
@Observable
public final class NotificationsController: NSObject {
    private let api: WeaveAPI
    private let vendorID: String

    /// Ce que le système a répondu. `nil` tant qu'on n'a rien demandé.
    public private(set) var autorise: Bool?

    public init(api: WeaveAPI, vendorID: String) {
        self.api = api
        self.vendorID = vendorID
        super.init()
    }

    /// Demande l'autorisation puis, si elle est accordée, enregistre l'appareil
    /// auprès d'APNs.
    ///
    /// Sans effet si la personne a déjà répondu : iOS conserve son choix, et
    /// redemander ne rouvre pas la boîte de dialogue.
    public func demanderAutorisation() async {
        let centre = UNUserNotificationCenter.current()
        let etat = await centre.notificationSettings()

        switch etat.authorizationStatus {
        case .notDetermined:
            let accorde =
                (try? await centre.requestAuthorization(options: [.alert, .sound])) ?? false
            autorise = accorde
            guard accorde else { return }
        case .denied:
            autorise = false
            return
        default:
            autorise = true
        }

        UIApplication.shared.registerForRemoteNotifications()
    }

    /// Transmet au serveur le jeton qu'APNs vient d'attribuer.
    ///
    /// Appelée depuis le délégué de l'application : c'est le seul endroit où
    /// iOS remet ce jeton, et il peut changer sans prévenir — à la restauration
    /// d'une sauvegarde, par exemple. On le réenvoie donc à chaque fois plutôt
    /// que de supposer qu'il est stable.
    public func deposer(jeton: Data) async {
        try? await api.registerDevice(
            DeviceRegistration(
                vendorId: vendorID,
                platform: "ios",
                apnsToken: jeton.hexString,
                apnsEnvironment: Self.apnsEnvironment
            )
        )
    }

    /// Retire le jeton du compte : après une déconnexion, plus rien ne doit
    /// arriver sur cet appareil pour quelqu'un qui n'y est plus connecté.
    public func oublier() async {
        UIApplication.shared.unregisterForRemoteNotifications()
        autorise = nil
    }

    private static var apnsEnvironment: String {
        #if DEBUG
        "sandbox"
        #else
        "production"
        #endif
    }
}
#endif
