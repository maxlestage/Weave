import Foundation

/// Erreurs remontées par l'API, alignées sur les codes stables du serveur
/// (`packages/contracts/src/errors.ts`).
public enum WeaveAPIError: Error, Sendable, Equatable {
    case unauthorized
    case forbidden
    case notFound
    case validation(String)
    case rateLimited(String)
    /// Le quota de demandes du jour est épuisé. C'est l'invariant central :
    /// on ne peut pas arroser, et aucun achat ne lève cette limite du jour.
    case noRequestsLeft(String)
    /// On a déjà le maximum de plans ouverts.
    case tooManyPlans(String)
    /// Le plan n'accepte plus de demandes : complet, annulé ou passé.
    case planClosed(String)
    /// On a déjà demandé à venir. On ne redemande pas deux fois.
    case alreadyRequested
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
        case .noRequestsLeft(let message): message
        case .tooManyPlans(let message): message
        case .planClosed(let message): message
        case .alreadyRequested: "Vous avez déjà demandé à venir."
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

    // MARK: - Les plans

    /// Le fil : les plans à venir autour de soi, du plus imminent au plus
    /// lointain. L'ordre vient du serveur et n'est jamais retouché ici.
    public func feed() async throws -> Feed {
        try await request(.get, "/v1/plans")
    }

    public func publish(_ plan: PlanDraft) async throws -> PublishedPlan {
        try await request(.post, "/v1/plans", encodable: plan)
    }

    public func myPlans() async throws -> [MyPlan] {
        try await request(.get, "/v1/plans/mine")
    }

    /// Les demandes reçues sur un de ses plans. Seul l'auteur peut les lire.
    public func incomingRequests(planID: String) async throws -> [IncomingRequest] {
        try await request(.get, "/v1/plans/\(planID)/requests")
    }

    public func cancelPlan(id: String) async throws {
        let _: EmptyResponse = try await request(.delete, "/v1/plans/\(id)")
    }

    // MARK: - Les demandes

    /// Demande à venir. Le message est obligatoire : c'est l'écriture qui
    /// engage. Chaque envoi consomme une unité du quota du jour.
    public func join(planID: String, message: String) async throws -> SentRequest {
        try await request(.post, "/v1/requests", body: ["planId": planID, "message": message])
    }

    public func sentRequests() async throws -> SentRequests {
        try await request(.get, "/v1/requests/sent")
    }

    /// Retire une demande. L'unité de quota est rendue si elle n'a pas été lue.
    public func withdraw(requestID: String) async throws {
        let _: EmptyResponse = try await request(.delete, "/v1/requests/\(requestID)")
    }

    public func accept(requestID: String) async throws -> String {
        struct Result: Decodable { let conversationId: String }
        let result: Result = try await request(.post, "/v1/requests/\(requestID)/accept")
        return result.conversationId
    }

    public func decline(requestID: String) async throws {
        let _: EmptyResponse = try await request(.post, "/v1/requests/\(requestID)/decline")
    }

    /// Applique un « Renfort » : quelques demandes de plus pour la journée en
    /// cours. Le nombre de renforts applicables dans une journée est lui-même
    /// borné — l'argent ne lève pas l'invariant, il l'assouplit une fois ou deux.
    public func applyRenfort() async throws -> RenfortResult {
        try await request(.post, "/v1/requests/renfort")
    }

    // MARK: - Conversations

    public func conversations() async throws -> [Conversation] {
        try await request(.get, "/v1/conversations")
    }

    public func messages(conversationID: String) async throws -> [Message] {
        struct Page: Decodable { let messages: [Message] }
        let page: Page = try await request(.get, "/v1/conversations/\(conversationID)/messages")
        return page.messages
    }

    public func send(conversationID: String, body: String) async throws -> Message {
        try await request(.post, "/v1/conversations/\(conversationID)/messages", body: ["body": body])
    }

    public func close(conversationID: String) async throws {
        let _: EmptyResponse = try await request(.delete, "/v1/conversations/\(conversationID)")
    }

    // MARK: - Compte

    public func me() async throws -> Me {
        try await request(.get, "/v1/me")
    }

    public func updatePreferences(_ preferences: PreferencesPatch) async throws {
        let _: EmptyResponse = try await request(.patch, "/v1/me/preferences", encodable: preferences)
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

    // MARK: - Se protéger

    /// Bloque quelqu'un.
    ///
    /// Le serveur coupe tout dans les deux sens : les demandes en attente
    /// expirent, les conversations se closent, et aucun des deux ne reverra
    /// les plans de l'autre. L'autre partie n'est pas prévenue — prévenir
    /// qu'on vient d'être bloqué n'apaise rien et expose la personne qui
    /// s'est protégée.
    public func block(accountID: String) async throws {
        let _: EmptyResponse = try await request(
            .post, "/v1/blocks", body: ["accountId": accountID]
        )
    }

    /// Lève un blocage posé plus tôt.
    ///
    /// Ne rouvre rien de ce que le blocage a coupé : les conversations closes
    /// le restent, les demandes expirées aussi. Seule la visibilité revient.
    public func unblock(accountID: String) async throws {
        let _: EmptyResponse = try await request(.delete, "/v1/blocks/\(accountID)")
    }

    /// Signale quelqu'un.
    ///
    /// Le signalement bloque d'office : personne n'a à revoir les plans de qui
    /// il vient de signaler pendant que le dossier est examiné. Inutile donc
    /// d'appeler `block` dans la foulée.
    public func report(
        accountID: String,
        reason: ReportReason,
        details: String? = nil
    ) async throws {
        struct Body: Encodable {
            let accountId: String
            let reason: String
            let details: String?
        }
        // Des précisions vides et des précisions absentes sont la même chose :
        // le serveur accepte les deux, autant n'en envoyer qu'une.
        let precisions = details?.trimmingCharacters(in: .whitespacesAndNewlines)
        let _: EmptyResponse = try await request(
            .post,
            "/v1/reports",
            encodable: Body(
                accountId: accountID,
                reason: reason.rawValue,
                details: (precisions?.isEmpty ?? true) ? nil : precisions
            )
        )
    }

    // MARK: - Souffler

    /// Met le compte en pause, ou le reprend.
    ///
    /// En pause : les plans ouverts sortent du fil des autres et personne ne
    /// peut plus demander à venir. Rien n'est supprimé — les conversations
    /// attendent, et les plans reviennent tels quels à la reprise.
    public func setPaused(_ paused: Bool) async throws {
        struct Body: Encodable { let paused: Bool }
        let _: EmptyResponse = try await request(
            .post, "/v1/me/pause", encodable: Body(paused: paused)
        )
    }

    // MARK: - Ses données

    /// Récupère l'export de ses données — article 20 du RGPD.
    ///
    /// Rendu tel quel, sans décodage : c'est un fichier qu'on emporte, pas une
    /// structure que l'application exploite. Le décoder pour le réencoder
    /// risquerait d'en perdre une partie au premier champ ajouté côté serveur.
    public func exportData() async throws -> Data {
        let token = try await validToken()
        return try await sendRaw(.get, "/v1/me/export", token: token)
    }

    /// Demande la suppression de son compte.
    ///
    /// En deux temps côté serveur : le compte sort du fil immédiatement, les
    /// données sont effacées à l'issue du délai légal. La session locale est
    /// vidée ici — il n'y a plus rien à rouvrir.
    public func deleteAccount() async throws {
        let _: EmptyResponse = try await request(.delete, "/v1/auth/account")
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

    /// Même chemin que `send`, mais rend les octets au lieu de les décoder.
    ///
    /// La gestion d'erreur est identique : un export refusé doit produire la
    /// même erreur typée qu'un appel ordinaire, sinon l'écran qui l'appelle
    /// devrait traiter deux vocabulaires d'échec.
    private func sendRaw(_ method: Method, _ path: String, token: String?) async throws -> Data {
        var request = URLRequest(url: baseURL.appending(path: path))
        request.httpMethod = method.rawValue
        request.timeoutInterval = 60

        if let token {
            request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
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
        return data
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
        case "no_requests_left": .noRequestsLeft(body.message)
        case "too_many_plans": .tooManyPlans(body.message)
        case "plan_closed": .planClosed(body.message)
        case "already_requested": .alreadyRequested
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

/// Un plan à publier.
public struct PlanDraft: Encodable, Sendable {
    public let title: String
    public let note: String?
    public let category: PlanCategory
    /// Date et heure du rendez-vous, ISO 8601.
    public let startsAt: String
    public let city: String?
    /// Personnes attendues en plus de soi. Au-delà d'une, il faut le droit aux
    /// plans de groupe — par le palier ou par un crédit « Tablée ».
    public let capacity: Int?

    public init(
        title: String,
        note: String? = nil,
        category: PlanCategory,
        startsAt: Date,
        city: String? = nil,
        capacity: Int? = nil
    ) {
        self.title = title
        self.note = note
        self.category = category
        self.startsAt = ISO8601DateFormatter.weave.string(from: startsAt)
        self.city = city
        self.capacity = capacity
    }
}

public struct PublishedPlan: Decodable, Sendable {
    public let id: String
    public let title: String
    public let startsAt: Date
}

public struct SentRequest: Decodable, Sendable {
    public let id: String
    public let sentAt: Date
    /// Ce qu'il reste après cet envoi : affiché tout de suite, sans second appel.
    public let requestsLeftToday: Int
}

public struct RenfortResult: Decodable, Sendable {
    public let granted: Int
    public let requestsLeftToday: Int
}

public struct SentRequests: Decodable, Sendable {
    public let requests: [JoinRequest]
    public let requestsLeftToday: Int
}

/// Critères du fil. Tous les champs sont facultatifs : on n'envoie que ce qui
/// change.
public struct PreferencesPatch: Encodable, Sendable {
    public let minAge: Int?
    public let maxAge: Int?
    public let maxDistanceKm: Int?
    public let categories: [PlanCategory]?

    public init(
        minAge: Int? = nil,
        maxAge: Int? = nil,
        maxDistanceKm: Int? = nil,
        categories: [PlanCategory]? = nil
    ) {
        self.minAge = minAge
        self.maxAge = maxAge
        self.maxDistanceKm = maxDistanceKm
        self.categories = categories
    }
}

public struct DeviceRegistration: Encodable, Sendable {
    public let vendorId: String
    public let platform: String
    public let model: String?
    public let osVersion: String?
    public let appVersion: String?
    public let apnsToken: String?
    /// Jeton « push-to-start » ActivityKit : autorise le serveur à démarrer la
    /// Live Activity quand quelqu'un demande à venir, application fermée.
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
