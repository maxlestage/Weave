// Un trousseau simulé, pour la vérification sous Linux UNIQUEMENT.
//
// `SessionStore` parle au cadre Security, qui n'existe que chez Apple. Sans ces
// symboles, WeaveKit ne compile pas hors d'un Mac — et c'est tout le reste de
// la bibliothèque qu'on ne pourrait plus éprouver. Le magasin ci-dessous garde
// les octets en mémoire : il ne protège rien, ne persiste rien, et ne quitte
// jamais ce répertoire. Il n'est jamais compilé dans l'application : il ne fait
// pas partie de la cible WeaveKit, et le garde `!canImport(Security)` le
// viderait de toute façon sur une plateforme Apple.
#if !canImport(Security)
import Foundation

typealias CFTypeRef = AnyObject
typealias CFDictionary = [String: Any]

let kSecClass = "class"
let kSecClassGenericPassword = "genp"
let kSecAttrService = "svce"
let kSecAttrAccount = "acct"
let kSecAttrAccessGroup = "agrp"
let kSecReturnData = "r_Data"
let kSecMatchLimit = "m_Limit"
let kSecMatchLimitOne = "m_LimitOne"
let kSecValueData = "v_Data"
let kSecAttrAccessible = "pdmn"
let kSecAttrAccessibleAfterFirstUnlock = "ck"
let errSecSuccess: Int32 = 0
let errSecItemNotFound: Int32 = -25300

private final class Trousseau: @unchecked Sendable {
    static let partage = Trousseau()
    private let verrou = NSLock()
    private var contenu: [String: Data] = [:]

    private func cle(_ requete: [String: Any]) -> String {
        let service = requete[kSecAttrService] as? String ?? ""
        let compte = requete[kSecAttrAccount] as? String ?? ""
        return "\(service)/\(compte)"
    }

    func lire(_ requete: [String: Any]) -> Data? {
        verrou.lock(); defer { verrou.unlock() }
        return contenu[cle(requete)]
    }

    func ecrire(_ requete: [String: Any], _ octets: Data) {
        verrou.lock(); defer { verrou.unlock() }
        contenu[cle(requete)] = octets
    }

    func existe(_ requete: [String: Any]) -> Bool {
        verrou.lock(); defer { verrou.unlock() }
        return contenu[cle(requete)] != nil
    }

    func effacer(_ requete: [String: Any]) {
        verrou.lock(); defer { verrou.unlock() }
        contenu[cle(requete)] = nil
    }
}

func SecItemCopyMatching(_ requete: CFDictionary, _ sortie: inout CFTypeRef?) -> Int32 {
    guard let octets = Trousseau.partage.lire(requete) else { return errSecItemNotFound }
    sortie = octets as NSData
    return errSecSuccess
}

func SecItemUpdate(_ requete: CFDictionary, _ attributs: CFDictionary) -> Int32 {
    guard Trousseau.partage.existe(requete) else { return errSecItemNotFound }
    if let octets = attributs[kSecValueData] as? Data {
        Trousseau.partage.ecrire(requete, octets)
    }
    return errSecSuccess
}

@discardableResult
func SecItemAdd(_ requete: CFDictionary, _ inutilise: CFTypeRef?) -> Int32 {
    guard let octets = requete[kSecValueData] as? Data else { return errSecItemNotFound }
    Trousseau.partage.ecrire(requete, octets)
    return errSecSuccess
}

@discardableResult
func SecItemDelete(_ requete: CFDictionary) -> Int32 {
    Trousseau.partage.effacer(requete)
    return errSecSuccess
}
#endif
