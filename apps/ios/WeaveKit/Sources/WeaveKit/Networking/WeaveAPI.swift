import Foundation

/// Erreurs remontées par l'API, alignées sur les codes stables du serveur
/// (`packages/contracts/src/errors.ts`).
public enum WeaveAPIError: Error, Sendable, Equatable {
    case unauthorized
    /// Refus qui ne vient ni du palier ni des crédits : une condition que le
    /// serveur seul connaît, et qu'il explique.
    ///
    /// Le message était jeté au profit d'un « Cette action ne vous est pas
    /// permise » générique. Le refus de chercher par genre sans consentement
    /// dit où le donner — Réglages › Confidentialité — et cette phrase
    /// n'atteignait personne.
    case forbidden(String)
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
    /// Le serveur refuse faute de crédit ou faute d'offre.
    ///
    /// `message` est celui du serveur, et il est rendu tel quel : c'est lui
    /// qui sait si le refus porte sur un crédit — « Renfort » n'est pas
    /// compris dans votre offre — ou sur un palier — filtrer par catégorie
    /// demande une offre supérieure. Le client répondait « Cette action
    /// demande un crédit » dans les deux cas, ce qui est faux dans le second
    /// et envoie acheter ce qu'on n'a pas besoin d'acheter.
    case entitlementRequired(message: String, sku: String?, productID: String?)
    case server(status: Int, message: String)
    case transport(String)

    public var userMessage: String {
        switch self {
        case .unauthorized: "Votre session a expiré. Reconnectez-vous."
        case .forbidden(let message): message
        case .notFound: "Introuvable."
        case .validation(let message): message
        case .rateLimited(let message): message
        case .noRequestsLeft(let message): message
        case .tooManyPlans(let message): message
        case .planClosed(let message): message
        case .alreadyRequested: "Vous avez déjà demandé à venir."
        case .entitlementRequired(let message, _, _): message
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

    /// L'adresse d'une page publique servie par le même hôte que l'API.
    ///
    /// Le service sert le site vitrine sous la même origine : les pages
    /// juridiques s'atteignent donc depuis l'application sans coder de domaine
    /// en dur — un domaine écrit en dur serait faux en développement, et faux
    /// le jour où il change.
    public func pagePublique(_ chemin: String) -> URL {
        baseURL.appending(path: chemin)
    }
    private let session: URLSession
    private let store: SessionStore

    private let encoder: JSONEncoder = {
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        return encoder
    }()

    /// Le décodeur, partagé par TOUT le client — renouvellement de session
    /// compris.
    ///
    /// Il était propre à l'instance, et le renouvellement, qui est statique,
    /// s'en fabriquait un second en `.iso8601`. Or `.iso8601` s'appuie sur
    /// `.withInternetDateTime`, qui REFUSE les fractions de seconde — et l'API
    /// en met toujours : `expiresAt` vaut « …T16:30:00.000Z ».
    ///
    /// Chaque renouvellement échouait donc à décoder sa réponse, et la session
    /// tombait au bout du quart d'heure du jeton d'accès. Aucun compilateur
    /// n'aurait rien dit : les deux décodeurs sont parfaitement valides, ils ne
    /// lisent simplement pas le même format.
    private static let decoder: JSONDecoder = {
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

    /// Relit ses critères.
    ///
    /// Cette lecture manquait : `PreferencesPatch` n'est qu'`Encodable`, et
    /// l'écran des réglages affichait donc des valeurs écrites en dur quels
    /// que soient les réglages réels.
    public func preferences() async throws -> Preferences {
        try await request(.get, "/v1/me/preferences")
    }

    /// Ouvre une « Escale » : le fil se compose autour d'une autre ville
    /// pendant sept jours. Dépense un crédit « Escale ».
    public func openEscale(city: String) async throws -> Escale {
        try await request(
            .post, "/v1/me/escale",
            body: ["city": city.trimmingCharacters(in: .whitespacesAndNewlines)]
        )
    }

    /// Ferme une escale avant son terme.
    ///
    /// Le crédit n'est pas rendu : il a été dépensé, et l'escale a servi.
    /// Fermer sert à revenir chez soi plus tôt, pas à annuler un achat.
    public func closeEscale() async throws {
        let _: EmptyResponse = try await request(.delete, "/v1/me/escale")
    }

    /// Établit un « Bilan » sur ses plans passés. Dépense un crédit « Bilan ».
    ///
    /// Le serveur refuse — sans rien dépenser — quand il n'y a pas assez de
    /// plans passés pour conclure quoi que ce soit. L'erreur porte alors le
    /// message à afficher.
    public func requestBilan() async throws -> Bilan {
        try await request(.post, "/v1/me/bilan")
    }

    public func updatePreferences(_ preferences: PreferencesPatch) async throws {
        let _: EmptyResponse = try await request(.patch, "/v1/me/preferences", encodable: preferences)
    }

    // MARK: - Vérification de profil

    /// Où en est ma demande, et le badge est-il posé ?
    public func verificationState() async throws -> EtatDeVerification {
        try await request(.get, "/v1/me/verification")
    }

    /// Demande la vérification de son profil.
    ///
    /// `note` est un mot libre et facultatif. Aucune pièce d'identité ne
    /// transite par l'application : la vérification se poursuit par courrier,
    /// et une réserve de papiers d'identité serait une responsabilité que ce
    /// service n'a aucune raison de prendre.
    public func requestVerification(note: String?) async throws {
        let _: EmptyResponse = try await request(
            .post, "/v1/me/verification",
            body: ["note": note ?? ""]
        )
    }

    // MARK: - Consentements

    /// L'état de chaque consentement, y compris ceux jamais donnés.
    public func consents() async throws -> Consentements {
        try await request(.get, "/v1/me/consents")
    }

    /// Donne un consentement, sur la version du texte que le serveur annonce.
    ///
    /// La version vient de `consents()` plutôt que d'une constante compilée
    /// ici : une application pas encore mise à jour consentirait sinon à un
    /// texte qu'elle n'affiche pas.
    public func grantConsent(_ kind: ConsentKind, version: String) async throws {
        let _: EmptyResponse = try await request(
            .post, "/v1/me/consents",
            body: ["kind": kind.rawValue, "version": version]
        )
    }

    /// Retire un consentement. Le critère qu'il couvrait est effacé côté
    /// serveur : le fil cesse de filtrer dessus, et le service continue.
    public func revokeConsent(_ kind: ConsentKind) async throws {
        let _: EmptyResponse = try await request(
            .post, "/v1/me/consents/revoke",
            body: ["kind": kind.rawValue]
        )
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

    /// Dépose ou met à jour sa fiche : une ville, un genre, une phrase.
    ///
    /// C'est l'étape qui manquait. L'inscription ne demandait que le prénom et
    /// la date de naissance ; le compte restait donc « onboarding », sans
    /// fiche — et sans fiche, le serveur rend un fil vide et refuse toute
    /// publication avec « Renseignez d'abord votre ville. »
    ///
    /// La position est arrondie au kilomètre par le serveur : Weave ne
    /// conserve jamais de position plus précise, pas même pour soi.
    public func submitProfile(
        city: String,
        latitude: Double,
        longitude: Double,
        gender: Gender,
        bio: String? = nil
    ) async throws {
        struct Body: Encodable {
            let city: String
            let latitude: Double
            let longitude: Double
            let gender: String
            let bio: String?
        }
        let texte = bio?.trimmingCharacters(in: .whitespacesAndNewlines)
        let _: EmptyResponse = try await request(
            .put,
            "/v1/me/profile",
            encodable: Body(
                city: city.trimmingCharacters(in: .whitespacesAndNewlines),
                latitude: latitude,
                longitude: longitude,
                gender: gender.rawValue,
                bio: (texte?.isEmpty ?? true) ? nil : texte
            )
        )
    }

    /// Dépose sa photo de profil.
    ///
    /// Le corps est l'image elle-même, brute : il n'y a qu'un fichier et aucun
    /// champ qui l'accompagne. Le serveur déduit le format des octets de tête
    /// plutôt que de l'en-tête annoncé — celui-ci est écrit par l'appelant, et
    /// resservir plus tard un « image/jpeg » qui n'en est pas laisserait le
    /// navigateur du destinataire décider quoi en faire.
    ///
    /// Rend l'URL signée de la photo déposée.
    public func submitPhoto(_ image: Data) async throws -> URL? {
        struct Result: Decodable { let photoUrl: URL? }
        let token = try await validToken()
        let brut = try await sendRawUpload(.put, "/v1/me/photo", token: token, payload: image)
        return try Self.decoder.decode(Result.self, from: brut).photoUrl
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

    /// L'en-tête `Accept-Language`, tiré des langues préférées du système.
    ///
    /// Sans lui, le service répond en français : c'est sa langue par défaut, et
    /// il n'a aucun autre moyen de connaître la nôtre. Or ses messages sont
    /// rendus TELS QUELS — voir `WeaveAPIError.message` plus haut : une
    /// application en anglais affichait « Ce plan est complet. » au milieu de
    /// son propre texte.
    ///
    /// Les poids décroissent dans l'ordre de préférence du système, comme la
    /// norme le prévoit. Trois suffisent : au-delà, le service ne parle de
    /// toute façon aucune des suivantes.
    private static let acceptLanguage: String = {
        let preferees = Locale.preferredLanguages.prefix(3)
        guard !preferees.isEmpty else { return "fr" }
        return preferees
            .enumerated()
            .map { rang, etiquette in
                rang == 0 ? etiquette : "\(etiquette);q=\(String(format: "%.1f", 1.0 - Double(rang) / 10.0))"
            }
            .joined(separator: ",")
    }()

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
        request.setValue(Self.acceptLanguage, forHTTPHeaderField: "Accept-Language")
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
            throw Self.decodeError(status: http.statusCode, data: data, decoder: Self.decoder)
        }

        if T.self == EmptyResponse.self { return EmptyResponse() as! T }

        do {
            return try Self.decoder.decode(T.self, from: data)
        } catch {
            throw WeaveAPIError.server(status: http.statusCode, message: "Réponse illisible.")
        }
    }

    /// Même chemin que `send`, mais rend les octets au lieu de les décoder.
    ///
    /// La gestion d'erreur est identique : un export refusé doit produire la
    /// même erreur typée qu'un appel ordinaire, sinon l'écran qui l'appelle
    /// devrait traiter deux vocabulaires d'échec.
    /// Envoie des octets bruts et rend la réponse brute.
    ///
    /// Distincte de `send` : celle-ci encode du JSON, et une image encodée en
    /// JSON ferait un tiers de taille en plus pour rien.
    private func sendRawUpload(
        _ method: Method,
        _ path: String,
        token: String,
        payload: Data
    ) async throws -> Data {
        var request = URLRequest(url: baseURL.appending(path: path))
        request.httpMethod = method.rawValue
        // Une photo part plus lentement qu'une requête ordinaire, et souvent
        // sur un réseau mobile : la minute par défaut y suffit rarement.
        request.timeoutInterval = 120
        request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        request.setValue("application/octet-stream", forHTTPHeaderField: "Content-Type")
        request.setValue(Self.acceptLanguage, forHTTPHeaderField: "Accept-Language")

        let data: Data
        let response: URLResponse
        do {
            (data, response) = try await session.upload(for: request, from: payload)
        } catch {
            throw WeaveAPIError.transport(error.localizedDescription)
        }

        guard let http = response as? HTTPURLResponse else {
            throw WeaveAPIError.transport("Réponse inattendue.")
        }
        guard (200..<300).contains(http.statusCode) else {
            throw Self.decodeError(status: http.statusCode, data: data, decoder: Self.decoder)
        }
        return data
    }

    private func sendRaw(_ method: Method, _ path: String, token: String?) async throws -> Data {
        var request = URLRequest(url: baseURL.appending(path: path))
        request.httpMethod = method.rawValue
        request.setValue(Self.acceptLanguage, forHTTPHeaderField: "Accept-Language")
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
            throw Self.decodeError(status: http.statusCode, data: data, decoder: Self.decoder)
        }
        return data
    }

    private static func decodeError(status: Int, data: Data, decoder: JSONDecoder) -> WeaveAPIError {
        guard let body = try? decoder.decode(APIErrorBody.self, from: data) else {
            return .server(status: status, message: "Erreur \(status).")
        }
        return switch body.error {
        case "unauthorized": .unauthorized
        case "forbidden": .forbidden(body.message)
        case "not_found": .notFound
        case "validation": .validation(body.message)
        case "rate_limited": .rateLimited(body.message)
        case "no_requests_left": .noRequestsLeft(body.message)
        case "too_many_plans": .tooManyPlans(body.message)
        case "plan_closed": .planClosed(body.message)
        case "already_requested": .alreadyRequested
        case "entitlement_required":
            .entitlementRequired(
                message: body.message,
                sku: body.details?.sku,
                productID: body.details?.unitProductId
            )
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
        request.setValue(Self.acceptLanguage, forHTTPHeaderField: "Accept-Language")
        request.httpBody = try JSONEncoder().encode(["refreshToken": current.refreshToken])

        let (data, response) = try await urlSession.data(for: request)
        guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
            await store.clear()
            throw WeaveAPIError.unauthorized
        }

        struct Envelope: Decodable { let session: Session }
        // Le décodeur partagé, et pas un second : c'est en s'en fabriquant un
        // que ce chemin avait cessé de lire les dates de l'API.
        let renewed = try Self.decoder.decode(Envelope.self, from: data).session
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
    /// Les genres recherchés. Le champ existait côté serveur et manquait ici :
    /// l'application ne pouvait pas dire qui l'on cherche, et la
    /// correspondance par genre restait donc inatteignable.
    public let seeking: [Gender]?
    /// Jours retenus, au sens ISO : 1 lundi, 7 dimanche. Vide = tous.
    public let days: [Int]?

    public init(
        minAge: Int? = nil,
        maxAge: Int? = nil,
        maxDistanceKm: Int? = nil,
        categories: [PlanCategory]? = nil,
        seeking: [Gender]? = nil,
        days: [Int]? = nil
    ) {
        self.minAge = minAge
        self.maxAge = maxAge
        self.maxDistanceKm = maxDistanceKm
        self.seeking = seeking
        self.days = days
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
