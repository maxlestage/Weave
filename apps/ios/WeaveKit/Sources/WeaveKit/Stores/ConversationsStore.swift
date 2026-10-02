import Foundation
import Observation

/// Les conversations, partagées par l'iPhone et la montre.
///
/// ## Pourquoi un magasin, et pourquoi ici
///
/// Les conversations se lisaient et s'écrivaient directement depuis l'écran
/// de l'iPhone, avec des `try?` qui avalaient les refus : un message refusé
/// disparaissait sans un mot, et rien ne permettait de fermer une
/// conversation — alors que le site promet qu'« elle se ferme quand vous
/// voulez ». La route existait, le client aussi ; aucun écran ne les reliait.
///
/// La montre doit maintenant répondre par dictée. Écrire une seconde fois la
/// même logique dans une seconde vue, c'était s'assurer que les deux
/// divergent : elle vit donc dans WeaveKit, où les deux la partagent et où un
/// test peut l'éprouver contre un serveur simulé.
@MainActor
@Observable
public final class ConversationsStore {
    public private(set) var conversations: [Conversation] = []
    /// Les messages chargés, par conversation.
    public private(set) var messages: [String: [Message]] = [:]
    /// Erreur à présenter, effacée dès qu'elle a été affichée.
    public var alert: WeaveAPIError?

    private let api: WeaveAPI

    public init(api: WeaveAPI) {
        self.api = api
    }

    /// Messages non lus, toutes conversations confondues.
    public var nonLus: Int { conversations.reduce(0) { $0 + $1.unread } }

    /// Les conversations où l'on peut encore écrire.
    public var ouvertes: [Conversation] { conversations.filter { !$0.closed } }

    public func refresh() async {
        do {
            conversations = try await api.conversations()
        } catch let erreur as WeaveAPIError {
            alert = erreur
        } catch {
            alert = .transport(error.localizedDescription)
        }
    }

    public func load(_ conversationID: String) async {
        do {
            messages[conversationID] = try await api.messages(conversationID: conversationID)
        } catch let erreur as WeaveAPIError {
            alert = erreur
        } catch {
            alert = .transport(error.localizedDescription)
        }
    }

    /// Ce qui partirait vraiment, ou `nil` si rien ne doit partir.
    ///
    /// Les espaces de bord ne comptent pas, et un message vide ne part pas. Un
    /// message trop long ne part pas non plus : le couper en silence enverrait
    /// une phrase que personne n'a écrite — c'est à l'écran de borner la
    /// saisie, et à ce garde-fou de refuser ce qui l'aurait débordée.
    public static func texteEnvoyable(_ brouillon: String) -> String? {
        let texte = brouillon.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !texte.isEmpty, texte.count <= conversationMaxChars else { return nil }
        return texte
    }

    /// Envoie un message. Vrai s'il est parti.
    @discardableResult
    public func send(_ brouillon: String, in conversationID: String) async -> Bool {
        guard let texte = Self.texteEnvoyable(brouillon) else { return false }
        do {
            let message = try await api.send(conversationID: conversationID, body: texte)
            messages[conversationID, default: []].append(message)
            return true
        } catch let erreur as WeaveAPIError {
            alert = erreur
            return false
        } catch {
            alert = .transport(error.localizedDescription)
            return false
        }
    }

    /// Ferme une conversation. Vrai si elle l'est.
    ///
    /// La liste est RELUE plutôt que retouchée : c'est le serveur qui sait
    /// qu'elle est close, et qui a posé la date de purge de ses messages.
    @discardableResult
    public func close(_ conversationID: String) async -> Bool {
        do {
            try await api.close(conversationID: conversationID)
            conversations = try await api.conversations()
            return true
        } catch let erreur as WeaveAPIError {
            alert = erreur
            return false
        } catch {
            alert = .transport(error.localizedDescription)
            return false
        }
    }
}
