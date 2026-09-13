import Foundation

/// Les objets sur lesquels un consentement distinct est demandé.
///
/// Un seul aujourd'hui, et c'est celui qui compte : les personnes que l'on
/// cherche, rapprochées de son propre genre, peuvent révéler l'orientation
/// sexuelle. Le règlement européen range cette information parmi les
/// catégories particulières de l'article 9 — elle ne peut être traitée que sur
/// un consentement explicite et distinct.
///
/// `packages/contracts` fait foi : `CONSENT_KINDS`.
public enum ConsentKind: String, Codable, Sendable, CaseIterable {
    case donneesSensibles = "donnees_sensibles"

    /// Ce que l'on demande, dit à la personne qui doit répondre.
    public var titre: String {
        switch self {
        case .donneesSensibles: "Chercher par genre"
        }
    }

    public var explication: String {
        switch self {
        case .donneesSensibles:
            """
            Les personnes que vous cherchez, rapprochées de votre genre, peuvent \
            révéler votre orientation sexuelle. La loi range cette information à \
            part : elle ne peut être traitée qu'avec votre accord explicite.

            Vous pouvez le retirer quand vous voulez. Le service continue de \
            fonctionner — le fil cesse simplement de filtrer sur ce critère, et \
            le critère est effacé.
            """
        }
    }
}

/// L'état d'un consentement, tel que le serveur le tient.
public struct Consentement: Decodable, Sendable, Identifiable {
    public let kind: ConsentKind
    /// La seule valeur dont dépend un traitement.
    ///
    /// Elle est fausse dans trois cas que l'application n'a pas à distinguer :
    /// jamais donné, retiré, ou donné sur une version du texte qui n'est plus
    /// en vigueur. Dans les trois, il faut redemander.
    public let active: Bool
    public let version: String?
    public let grantedAt: Date?
    public let revokedAt: Date?

    public var id: String { kind.rawValue }
}

public struct Consentements: Decodable, Sendable {
    public let consents: [Consentement]
    /// La version des textes en vigueur, à renvoyer avec un consentement.
    ///
    /// Elle vient du serveur plutôt que d'une constante locale : une
    /// application pas encore mise à jour consentirait sinon à un texte qu'elle
    /// n'affiche pas.
    public let policyVersion: String

    public func etat(_ kind: ConsentKind) -> Consentement? {
        consents.first { $0.kind == kind }
    }

    public func estActif(_ kind: ConsentKind) -> Bool {
        etat(kind)?.active ?? false
    }
}
