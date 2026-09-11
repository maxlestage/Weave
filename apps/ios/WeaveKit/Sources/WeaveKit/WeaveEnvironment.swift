import Foundation

/// Configuration d'exécution du client.
///
/// L'URL de l'API vient du `Info.plist` (`WEAVE_API_URL`), lui-même alimenté
/// par un fichier `.xcconfig` : rien n'est codé en dur dans le binaire, et les
/// builds de développement, de préproduction et de production ne se distinguent
/// que par leur configuration.
public enum WeaveEnvironment {
    public static var apiBaseURL: URL {
        if let raw = Bundle.main.object(forInfoDictionaryKey: "WEAVE_API_URL") as? String,
           let url = URL(string: raw) {
            return url
        }
        #if DEBUG
        return URL(string: "http://localhost:3000")!
        #else
        // En production, une configuration absente est une erreur de build :
        // mieux vaut s'en apercevoir immédiatement qu'appeler un hôte factice.
        preconditionFailure("WEAVE_API_URL manquant dans Info.plist.")
        #endif
    }

    /// Groupe d'accès trousseau partagé entre l'application, l'extension widget
    /// et l'application montre.
    public static let keychainAccessGroup: String? =
        Bundle.main.object(forInfoDictionaryKey: "WEAVE_KEYCHAIN_GROUP") as? String

    /// Groupe d'application, pour les préférences partagées avec les extensions.
    public static let appGroup = "group.com.weave.app"
}
