import Foundation

/// Erreurs remontées par l'API, alignées sur les codes stables du serveur
/// (`packages/contracts/src/errors.ts`).
public enum WeaveAPIError: Error, Sendable, Equatable {
    case unauthorized
    case forbidden
    case notFound
    case validation(String)
    case rateLimited(String)
    /// Le métier a déjà atteint son plafond de fils.
    case loomFull
    /// Le fil n'existe plus : il s'est dénoué.
    case threadGone
    /// Il manque un crédit ou un palier. Porte de quoi proposer l'achat.
    case entitlementRequired(sku: String?, productID: String?)
    case server(status: Int, message: String)
    case transport(String)

    public var userMessage: String {
        switch self {
        case .unauthorized: "Votre session a expiré. Reconnectez-vous."
        case .forbidden: "Cette action ne vous est pas permise."
        case .notFound: "Introuvable."
        case .validation(let message): message
        case .rateLimited(let message): message
        case .loomFull: "Votre métier est complet. Dénouez un fil pour faire de la place."
        case .threadGone: "Ce fil s'est dénoué."
        case .entitlementRequired: "Cette action demande un crédit."
        case .server(_, let message): message
        case .transport: "Connexion impossible. Réessayez."
        }
    }
}

private struct APIErrorBody: Decodable {
    struct Details: Decodable {
        let sku: String?
        let unitProductId: String?
    }
    let error: String
    let message: String
    let details: Details?
}

/// Client HTTP de Weave.
///
/// C'est un `actor` : le renouvellement de session doit être sérialisé, sinon
/// trois requêtes lancées en parallèle au retour d'arrière-plan déclenchent
/// trois rotations concurrentes du jeton de rafraîchissement — et la rotation
/// côté serveur invalide les deux perdantes.
public actor WeaveAPI {
    private let baseURL: URL
    private let session: URLSession
    private let store: SessionStore

    private let encoder: JSONEncoder = {
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        return encoder
    }()

    private let decoder: JSONDecoder = {
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .custom { decoder in
            let text = try decoder.singleValueContainer().decode(String.self)
            // L'API renvoie des dates ISO 8601 avec millisecondes ; certaines
            // routes n'en mettent pas. On accepte les deux formes.
            if let date = ISO8601DateFormatter.weaveWithFraction.date(from: text) { return date }
            if let date = ISO8601DateFormatter.weave.date(from: text) { return date }
            throw DecodingError.dataCorrupted(
                .init(codingPath: decoder.codingPath, debugDescription: "Date illisible : \(text)")
            )
        }
        return decoder
    }()

    /// Renouvellement en cours, partagé par tous les appelants.
    private var refreshTask: Task<Session, any Error>?

    public init(baseURL: URL, store: SessionStore, session: URLSession = .shared) {
        self.baseURL = baseURL
        self.store = store
        self.session = session
    }

    // MARK: - Le métier

    public func loom() async throws -> Loom {
        try await request(.get, "/v1/loom")
    }

    public func respond(threadID: String, fragmentID: String, body: String) async throws -> RespondResult {
        try await request(
            .post,
            "/v1/loom/threads/\(threadID)/respond",
            body: ["fragmentId": fragmentID, "body": body]
        )
    }

    public func release(threadID: String, reason: String?) async throws {
        let _: EmptyResponse = try await request(
            .post,
            "/v1/loom/threads/\(threadID)/release",
            body: reason.map { ["reason": $0] } ?? [:]
        )
    }

    public func extend(threadID: String) async throws -> Date {
        struct Result: Decodable { let expiresAt: Date }
        let result: Result = try await request(.post, "/v1/loom/threads/\(threadID)/extend")
        return result.expiresAt
    }

    /// Regarnit immédiatement une place libre. N'augmente jamais le plafond.
    public func relais() async throws -> Loom {
        try await request(.post, "/v1/loom/relais")
    }

    // MARK: - Compte

    public func me() async throws -> Me {
        try await request(.get, "/v1/me")
    }

    public func updateWeavingHour(_ hour: Int) async throws {
        struct Corps: Encodable { let weavingHour: Int }
        let _: EmptyResponse = try await request(.patch, "/v1/me", encodable: Corps(weavingHour: hour))
    }

    public func entitlement() async throws -> Entitlement {
        try await request(.get, "/v1/billing/entitlement")
    }

    // MARK: - Appareils, Live Activity, montre

    public func registerDevice(_ device: DeviceRegistration) async throws {
        let _: EmptyResponse = try await request(.put, "/v1/devices", encodable: device)
    }

    public func registerActivity(vendorID: String, updateToken: String) async throws {
        let _: EmptyResponse = try await request(
            .post,
            "/v1/live-activity/sessions",
            body: ["vendorId": vendorID, "updateToken": updateToken]
        )
    }

    public func endActivity(updateToken: String) async throws {
        let _: EmptyResponse = try await request(
            .delete,
            "/v1/live-activity/sessions/\(updateToken)"
        )
    }

    public func liveActivityState() async throws -> WeaveActivityAttributes.ContentState {
        try await request(.get, "/v1/live-activity/state")
    }

    public func watchSummary() async throws -> WatchSummary {
        try await request(.get, "/v1/watch/summary")
    }

    // MARK: - Achats

    public func submitSubscription(signedTransaction: String) async throws {
        let _: EmptyResponse = try await request(
            .post,
            "/v1/billing/subscriptions",
            body: ["signedTransaction": signedTransaction]
        )
    }

    public func submitUnitPurchase(signedTransaction: String) async throws {
        let _: EmptyResponse = try await request(
            .post,
            "/v1/billing/units",
            body: ["signedTransaction": signedTransaction]
        )
    }

    // MARK: - Connexion

    public func requestCode(email: String) async throws {
        let _: EmptyResponse = try await requestWithoutAuth(
            .post,
            "/v1/auth/otp/request",
            body: ["email": email]
        )
    }

    public func verifyCode(
        email: String,
        code: String,
        displayName: String? = nil,
        birthDate: String? = nil,
        timezone: String = TimeZone.current.identifier
    ) async throws -> VerifyResult {
        var body: [String: String] = ["email": email, "code": code, "timezone": timezone]
        if let displayName { body["displayName"] = displayName }
        if let birthDate { body["birthDate"] = birthDate }
        let result: VerifyResult = try await requestWithoutAuth(.post, "/v1/auth/otp/verify", body: body)
        if let session = result.session {
            await store.save(session)
        }
        return result
    }

    public func logout() async throws {
        let refresh = await store.current?.refreshToken
        let _: EmptyResponse = try await request(
            .post,
            "/v1/auth/logout",
            body: refresh.map { ["refreshToken": $0] } ?? [:]
        )
        await store.clear()
    }

    // MARK: - Mécanique

    private enum Method: String {
        case get = "GET"
        case post = "POST"
        case put = "PUT"
        case patch = "PATCH"
        case delete = "DELETE"
    }

    private struct EmptyResponse: Decodable {
        init(from decoder: any Decoder) throws {}
        init() {}
    }

    private func request<T: Decodable>(
        _ method: Method,
        _ path: String,
        body: [String: String] = [:]
    ) async throws -> T {
        let token = try await validToken()
        return try await send(method, path, token: token, payload: encoder.encode(body))
    }

    private func request<T: Decodable>(
        _ method: Method,
        _ path: String,
        encodable: some Encodable
    ) async throws -> T {
        let token = try await validToken()
        return try await send(method, path, token: token, payload: encoder.encode(encodable))
    }

    private func requestWithoutAuth<T: Decodable>(
        _ method: Method,
        _ path: String,
        body: [String: String]
    ) async throws -> T {
        try await send(method, path, token: nil, payload: encoder.encode(body))
    }

    private func send<T: Decodable>(
        _ method: Method,
        _ path: String,
        token: String?,
        payload: Data
    ) async throws -> T {
        var request = URLRequest(url: baseURL.appending(path: path))
        request.httpMethod = method.rawValue
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.timeoutInterval = 15

        if let token {
            request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        }
        if method != .get, method != .delete {
            request.httpBody = payload
        }

        let data: Data
        let response: URLResponse
        do {
            (data, response) = try await session.data(for: request)
        } catch {
            throw WeaveAPIError.transport(error.localizedDescription)
        }

        guard let http = response as? HTTPURLResponse else {
            throw WeaveAPIError.transport("Réponse inattendue.")
        }

        guard (200..<300).contains(http.statusCode) else {
            throw Self.decodeError(status: http.statusCode, data: data, decoder: decoder)
        }

        if T.self == EmptyResponse.self { return EmptyResponse() as! T }

        do {
            return try decoder.decode(T.self, from: data)
        } catch {
            throw WeaveAPIError.server(status: http.statusCode, message: "Réponse illisible.")
        }
    }

    private static func decodeError(status: Int, data: Data, decoder: JSONDecoder) -> WeaveAPIError {
        guard let body = try? decoder.decode(APIErrorBody.self, from: data) else {
            return .server(status: status, message: "Erreur \(status).")
        }
        return switch body.error {
        case "unauthorized": .unauthorized
        case "forbidden": .forbidden
        case "not_found": .notFound
        case "validation": .validation(body.message)
        case "rate_limited": .rateLimited(body.message)
        case "loom_full": .loomFull
        case "thread_gone": .threadGone
        case "entitlement_required":
            .entitlementRequired(sku: body.details?.sku, productID: body.details?.unitProductId)
        default: .server(status: status, message: body.message)
        }
    }

    /// Jeton d'accès valide, renouvelé si nécessaire. Un seul renouvellement à
    /// la fois : les appelants concurrents attendent le même résultat.
    private func validToken() async throws -> String {
        guard let session = await store.current else { throw WeaveAPIError.unauthorized }
        guard session.needsRefresh else { return session.accessToken }

        // Un renouvellement déjà en vol : on attend son résultat plutôt que
        // d'en lancer un second, qui invaliderait le premier par rotation.
        if let refreshTask {
            return try await refreshTask.value.accessToken
        }

        let task = Task<Session, any Error> { [store, baseURL, urlSession = self.session] in
            try await Self.refresh(baseURL: baseURL, store: store, session: urlSession)
        }
        refreshTask = task

        do {
            let renewed = try await task.value
            refreshTask = nil
            return renewed.accessToken
        } catch {
            refreshTask = nil
            throw error
        }
    }

    private static func refresh(
        baseURL: URL,
        store: SessionStore,
        session urlSession: URLSession
    ) async throws -> Session {
        guard let current = await store.current else { throw WeaveAPIError.unauthorized }

        var request = URLRequest(url: baseURL.appending(path: "/v1/auth/refresh"))
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONEncoder().encode(["refreshToken": current.refreshToken])

        let (data, response) = try await urlSession.data(for: request)
        guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
            await store.clear()
            throw WeaveAPIError.unauthorized
        }

        struct Envelope: Decodable { let session: Session }
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        let renewed = try decoder.decode(Envelope.self, from: data).session
        await store.save(renewed)
        return renewed
    }
}

// MARK: - Charges utiles

public struct VerifyResult: Decodable, Sendable {
    public let session: Session?
    /// Vrai lorsque le compte n'existe pas encore et qu'il faut un nom et une
    /// date de naissance pour poursuivre.
    public let needsProfile: Bool
    public let created: Bool
}

public struct RespondResult: Decodable, Sendable {
    public let thread: ThreadCard
    public let conversationId: String?
}

public struct DeviceRegistration: Encodable, Sendable {
    public let vendorId: String
    public let platform: String
    public let model: String?
    public let osVersion: String?
    public let appVersion: String?
    public let apnsToken: String?
    /// Jeton « push-to-start » ActivityKit : autorise le serveur à démarrer la
    /// Live Activity à l'heure de tissage, application fermée.
    public let pushToStartToken: String?
    public let apnsEnvironment: String

    public init(
        vendorId: String,
        platform: String,
        model: String? = nil,
        osVersion: String? = nil,
        appVersion: String? = nil,
        apnsToken: String? = nil,
        pushToStartToken: String? = nil,
        apnsEnvironment: String = "sandbox"
    ) {
        self.vendorId = vendorId
        self.platform = platform
        self.model = model
        self.osVersion = osVersion
        self.appVersion = appVersion
        self.apnsToken = apnsToken
        self.pushToStartToken = pushToStartToken
        self.apnsEnvironment = apnsEnvironment
    }
}

extension ISO8601DateFormatter {
    static let weave: ISO8601DateFormatter = {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime]
        return formatter
    }()

    static let weaveWithFraction: ISO8601DateFormatter = {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return formatter
    }()
}
