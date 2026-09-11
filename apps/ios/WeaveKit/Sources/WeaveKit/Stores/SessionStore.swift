import Foundation

/// Conservation de la session dans le trousseau.
///
/// Les jetons ne sont jamais écrits dans `UserDefaults` ni dans un fichier :
/// ils vont au trousseau, avec `kSecAttrAccessibleAfterFirstUnlock` pour que
/// l'extension Live Activity et les rafraîchissements en arrière-plan puissent
/// les lire appareil verrouillé — mais seulement après un premier déverrouillage.
public actor SessionStore {
    private let service: String
    private let account = "session"
    private let accessGroup: String?
    private var cached: Session?

    public init(service: String = "app.weave.session", accessGroup: String? = nil) {
        self.service = service
        self.accessGroup = accessGroup
        self.cached = Self.read(service: service, account: account, accessGroup: accessGroup)
    }

    public var current: Session? { cached }

    public var isAuthenticated: Bool { cached != nil }

    public func save(_ session: Session) {
        cached = session
        Self.write(session, service: service, account: account, accessGroup: accessGroup)
    }

    public func clear() {
        cached = nil
        Self.delete(service: service, account: account, accessGroup: accessGroup)
    }

    // MARK: - Trousseau

    private static func baseQuery(
        service: String,
        account: String,
        accessGroup: String?
    ) -> [String: Any] {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
        ]
        if let accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }
        return query
    }

    private static func read(service: String, account: String, accessGroup: String?) -> Session? {
        var query = baseQuery(service: service, account: account, accessGroup: accessGroup)
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne

        var item: CFTypeRef?
        guard SecItemCopyMatching(query as CFDictionary, &item) == errSecSuccess,
              let data = item as? Data
        else { return nil }

        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        return try? decoder.decode(Session.self, from: data)
    }

    private static func write(
        _ session: Session,
        service: String,
        account: String,
        accessGroup: String?
    ) {
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        guard let data = try? encoder.encode(session) else { return }

        let query = baseQuery(service: service, account: account, accessGroup: accessGroup)
        let attributes: [String: Any] = [
            kSecValueData as String: data,
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlock,
        ]

        let status = SecItemUpdate(query as CFDictionary, attributes as CFDictionary)
        if status == errSecItemNotFound {
            SecItemAdd(query.merging(attributes) { _, new in new } as CFDictionary, nil)
        }
    }

    private static func delete(service: String, account: String, accessGroup: String?) {
        SecItemDelete(baseQuery(service: service, account: account, accessGroup: accessGroup) as CFDictionary)
    }
}
