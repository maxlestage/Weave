import Foundation

/// Ce que l'iPhone et la montre se disent, par WatchConnectivity.
///
/// ## Pourquoi un type, et pas des dictionnaires
///
/// WatchConnectivity transporte des `[String: Any]`. Les écrire et les lire à
/// la main, des deux côtés, c'est deux orthographes d'une même clé qui
/// divergent un jour sans que rien ne le signale : la montre attendrait
/// « summary », l'iPhone enverrait « resume », et la montre resterait vide.
///
/// Ce type est le seul à savoir écrire et relire ces dictionnaires. Il vit
/// dans WeaveKit, compilé pour les deux appareils — et sous Linux, où un test
/// vérifie qu'un message relu est celui qui a été écrit. Le transport
/// lui-même, `WCSession`, n'existe que sur les appareils d'Apple.
///
/// Les valeurs sont des `Data` : WatchConnectivity n'accepte que des types de
/// liste de propriétés, et un `Data` en est un.
public enum MessageMontre: Sendable {
    /// iPhone → montre : la session propre à la montre.
    case session(Session)
    /// iPhone → montre : le résumé à afficher, sans attendre le réseau.
    case resume(WatchSummary)
    /// iPhone → montre : on s'est déconnecté, la montre oublie sa session.
    case deconnexion
    /// Montre → iPhone : je n'ai pas de session, donne-m'en une.
    case demandeDeSession

    /// Les dates partent dans la forme que le décodeur de l'API relit — avec
    /// millisecondes. Un second format, propre à ce canal, aurait été un
    /// second endroit où une date se perd en route.
    private static let encodeur: JSONEncoder = {
        let encodeur = JSONEncoder()
        encodeur.dateEncodingStrategy = .custom { date, sortie in
            var conteneur = sortie.singleValueContainer()
            try conteneur.encode(date.formatted(DateWeave.avecFractions))
        }
        return encodeur
    }()

    private static let cleGenre = "genre"
    private static let cleCharge = "charge"

    private var genre: String {
        switch self {
        case .session: "session"
        case .resume: "resume"
        case .deconnexion: "deconnexion"
        case .demandeDeSession: "demandeDeSession"
        }
    }

    /// Le dictionnaire à remettre à WatchConnectivity.
    public func dictionnaire() throws -> [String: Any] {
        var sortie: [String: Any] = [Self.cleGenre: genre]
        switch self {
        case .session(let session):
            sortie[Self.cleCharge] = try Self.encodeur.encode(session)
        case .resume(let resume):
            sortie[Self.cleCharge] = try Self.encodeur.encode(resume)
        case .deconnexion, .demandeDeSession:
            break
        }
        return sortie
    }

    /// Relit un dictionnaire reçu, ou `nil` s'il n'est pas l'un des nôtres.
    ///
    /// Un message illisible est ignoré plutôt que de faire tomber l'appareil
    /// qui le reçoit : une version plus récente de l'autre bout peut parler
    /// d'une chose que celle-ci ne connaît pas encore.
    public init?(_ dictionnaire: [String: Any]) {
        guard let genre = dictionnaire[Self.cleGenre] as? String else { return nil }
        let charge = dictionnaire[Self.cleCharge] as? Data
        switch genre {
        case "session":
            guard let charge,
                  let session = try? WeaveAPI.decoder.decode(Session.self, from: charge)
            else { return nil }
            self = .session(session)
        case "resume":
            guard let charge,
                  let resume = try? WeaveAPI.decoder.decode(WatchSummary.self, from: charge)
            else { return nil }
            self = .resume(resume)
        case "deconnexion":
            self = .deconnexion
        case "demandeDeSession":
            self = .demandeDeSession
        default:
            return nil
        }
    }
}
