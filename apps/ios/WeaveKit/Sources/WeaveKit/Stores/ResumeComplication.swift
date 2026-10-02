import Foundation

/// Le résumé que l'application montre laisse à sa complication.
///
/// La complication ne fait aucun appel réseau : elle serait réveillée bien trop
/// souvent pour une donnée qui change quelques fois par jour. Elle relit donc
/// ce que l'application montre a reçu en dernier.
///
/// ## Pourquoi le trousseau, et non un groupe d'application
///
/// Le résumé passait par les préférences d'un groupe d'application. Mais un
/// groupe doit être enregistré chez Apple, et c'est la seule capacité que la
/// signature automatique ne sait pas gérer avec une clé d'API : la construction
/// pour TestFlight échouait sur les deux cibles qui en déclaraient un
/// (« Authentication failed », puis « No profiles »).
///
/// Le trousseau partagé, lui, est permis d'office entre les applications d'une
/// même équipe — la session y vit déjà. Le résumé n'est pas un secret, mais il
/// dit à qui regarde le cadran combien de personnes veulent venir : qu'il soit
/// rangé comme la session ne lui fait pas de tort.
public enum ResumeComplication {
    static let service = "app.weave.complication"
    static let compte = "resume"

    /// Dépose le résumé, en remplaçant le précédent.
    public static func deposer(_ resume: WatchSummary, accessGroup: String?) {
        let encodeur = JSONEncoder()
        encodeur.dateEncodingStrategy = .iso8601
        guard let octets = try? encodeur.encode(resume) else { return }

        let requete = requeteDeBase(accessGroup: accessGroup)
        let attributs: [String: Any] = [
            kSecValueData as String: octets,
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlock,
        ]
        let statut = SecItemUpdate(requete as CFDictionary, attributs as CFDictionary)
        if statut == errSecItemNotFound {
            SecItemAdd(requete.merging(attributs) { _, nouveau in nouveau } as CFDictionary, nil)
        }
    }

    /// Le dernier résumé déposé, ou un résumé vide s'il n'y en a pas — ou
    /// s'il ne se relit plus (format changé entre deux versions).
    public static func lire(accessGroup: String?) -> WatchSummary {
        var requete = requeteDeBase(accessGroup: accessGroup)
        requete[kSecReturnData as String] = true
        requete[kSecMatchLimit as String] = kSecMatchLimitOne

        var element: CFTypeRef?
        guard SecItemCopyMatching(requete as CFDictionary, &element) == errSecSuccess,
              let octets = element as? Data
        else { return .empty }

        let decodeur = JSONDecoder()
        decodeur.dateDecodingStrategy = .iso8601
        return (try? decodeur.decode(WatchSummary.self, from: octets)) ?? .empty
    }

    private static func requeteDeBase(accessGroup: String?) -> [String: Any] {
        var requete: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: compte,
        ]
        if let accessGroup {
            requete[kSecAttrAccessGroup as String] = accessGroup
        }
        return requete
    }
}
