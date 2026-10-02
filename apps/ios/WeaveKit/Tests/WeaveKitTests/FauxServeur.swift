import Foundation
#if canImport(FoundationNetworking)
import FoundationNetworking
#endif
@testable import WeaveKit

/// Un serveur simulé, pour éprouver les magasins de bout en bout.
///
/// Les tests des modèles décodent du JSON écrit à la main ; ils ne disent rien
/// de ce que fait un magasin quand le serveur répond — ni s'il appelle la bonne
/// route. C'est pourtant là que se cachait le Renfort vendu sans effet : la
/// route existait, le client aussi, et rien ne les reliait. Un test qui passe
/// par un vrai `URLSession` voit ce genre de trou ; un test de modèle, non.
///
/// Chaque test construit SON serveur : les réponses vivent dans l'instance, et
/// l'instance est retrouvée par un en-tête que le client ne pose pas — l'hôte
/// de l'adresse de base, unique par test. Des tests lancés en parallèle ne se
/// marchent donc pas dessus.
final class FauxServeur: @unchecked Sendable {
    struct Reponse {
        let statut: Int
        let corps: String
    }

    private let verrou = NSLock()
    private var reponses: [String: Reponse] = [:]
    private var recues: [String] = []
    private var corps: [String: String] = [:]
    let hote: String

    init() {
        hote = "faux-\(UUID().uuidString.lowercased()).test"
        Registre.inscrire(self)
    }

    deinit { Registre.retirer(hote) }

    /// Ce que le serveur répond à « MÉTHODE /chemin ».
    func repondre(_ methode: String, _ chemin: String, statut: Int = 200, _ corps: String) {
        verrou.withLock { reponses["\(methode) \(chemin)"] = Reponse(statut: statut, corps: corps) }
    }

    /// Les requêtes reçues, dans l'ordre : « MÉTHODE /chemin ».
    var requetes: [String] { verrou.withLock { recues } }

    /// Le dernier corps reçu pour « MÉTHODE /chemin ».
    func corpsRecu(_ methode: String, _ chemin: String) -> String? {
        verrou.withLock { corps["\(methode) \(chemin)"] }
    }

    fileprivate func traiter(_ requete: URLRequest) -> Reponse {
        let cle = "\(requete.httpMethod ?? "GET") \(requete.url?.path ?? "")"
        let charge = Self.lireCorps(requete)
        return verrou.withLock {
            recues.append(cle)
            if let charge { corps[cle] = charge }
            // Une route que le test n'a pas prévue est une faute du test, ou une
            // requête que le code n'aurait pas dû faire : 599 se remarque.
            return reponses[cle] ?? Reponse(statut: 599, corps: #"{"error":"route non prévue : \#(cle)"}"#)
        }
    }

    /// Le corps d'une requête interceptée.
    ///
    /// Un `URLProtocol` reçoit souvent le corps sous forme de flux plutôt que
    /// de `httpBody` : il faut lire les deux.
    private static func lireCorps(_ requete: URLRequest) -> String? {
        if let donnees = requete.httpBody { return String(decoding: donnees, as: UTF8.self) }
        guard let flux = requete.httpBodyStream else { return nil }
        flux.open()
        defer { flux.close() }
        var donnees = Data()
        var tampon = [UInt8](repeating: 0, count: 4096)
        while flux.hasBytesAvailable {
            let lus = flux.read(&tampon, maxLength: tampon.count)
            if lus <= 0 { break }
            donnees.append(tampon, count: lus)
        }
        return String(decoding: donnees, as: UTF8.self)
    }

    /// Un client d'API branché sur ce serveur, et le magasin de session qu'il
    /// lit — vide si `avecSession` est faux, comme sur une montre neuve.
    func apiEtMagasin(avecSession: Bool) async -> (WeaveAPI, SessionStore) {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [Protocole.self]
        let magasin = SessionStore(service: "test.\(hote)")
        if avecSession {
            await magasin.save(Session(
                accessToken: "jeton",
                refreshToken: "renouvellement",
                expiresAt: .now.addingTimeInterval(3600)
            ))
        }
        let api = WeaveAPI(
            baseURL: URL(string: "https://\(hote)")!,
            store: magasin,
            session: URLSession(configuration: configuration)
        )
        return (api, magasin)
    }

    /// Un client d'API branché sur ce serveur, avec une session valide.
    ///
    /// `expireDans` permet de partir d'une session qui doit d'abord être
    /// renouvelée.
    func api(
        expireDans: TimeInterval = 3600,
        renouvellement: String = "renouvellement"
    ) async -> WeaveAPI {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [Protocole.self]
        let magasin = SessionStore(service: "test.\(hote)")
        await magasin.save(Session(
            accessToken: "jeton",
            refreshToken: renouvellement,
            expiresAt: .now.addingTimeInterval(expireDans)
        ))
        return WeaveAPI(
            baseURL: URL(string: "https://\(hote)")!,
            store: magasin,
            session: URLSession(configuration: configuration)
        )
    }

    /// Le registre des serveurs vivants, par hôte.
    private enum Registre {
        nonisolated(unsafe) static var serveurs: [String: FauxServeur] = [:]
        static let verrou = NSLock()

        static func inscrire(_ serveur: FauxServeur) {
            verrou.withLock { serveurs[serveur.hote] = serveur }
        }

        static func retirer(_ hote: String) {
            verrou.withLock { serveurs[hote] = nil }
        }

        static func trouver(_ hote: String?) -> FauxServeur? {
            guard let hote else { return nil }
            return verrou.withLock { serveurs[hote] }
        }
    }

    /// Le protocole qui détourne les requêtes vers le serveur simulé.
    final class Protocole: URLProtocol {
        override class func canInit(with request: URLRequest) -> Bool { true }
        override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }

        override func startLoading() {
            guard let serveur = Registre.trouver(request.url?.host),
                  let url = request.url
            else {
                client?.urlProtocol(self, didFailWithError: URLError(.cannotFindHost))
                return
            }
            let reponse = serveur.traiter(request)
            let entete = HTTPURLResponse(
                url: url,
                statusCode: reponse.statut,
                httpVersion: "HTTP/1.1",
                headerFields: ["Content-Type": "application/json"]
            )!
            client?.urlProtocol(self, didReceive: entete, cacheStoragePolicy: .notAllowed)
            client?.urlProtocol(self, didLoad: Data(reponse.corps.utf8))
            client?.urlProtocolDidFinishLoading(self)
        }

        override func stopLoading() {}
    }
}
