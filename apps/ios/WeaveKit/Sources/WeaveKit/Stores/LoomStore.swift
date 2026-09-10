import Foundation
import Observation

/// État observable du métier, partagé par toutes les vues de l'application.
///
/// Le magasin ne persiste rien sur disque : les fils vivent en mémoire, le
/// temps de la session. C'est la transposition côté client de la règle serveur
/// « les profils proposés n'existent qu'en cache ».
@MainActor
@Observable
public final class LoomStore {
    public enum Phase: Equatable {
        case idle
        case loading
        case ready
        case failed(String)
    }

    public private(set) var loom: Loom = .empty
    public private(set) var phase: Phase = .idle
    public private(set) var lastUpdated: Date?

    /// Erreur à présenter, effacée dès qu'elle a été affichée.
    public var alert: WeaveAPIError?

    /// Achat suggéré à la suite d'un refus pour crédit manquant.
    public var suggestedPurchase: UnitSku?

    private let api: WeaveAPI

    public init(api: WeaveAPI) {
        self.api = api
    }

    public var isEmpty: Bool { loom.threads.isEmpty }

    public func refresh() async {
        if phase == .idle { phase = .loading }
        do {
            loom = try await api.loom()
            lastUpdated = .now
            phase = .ready
        } catch let error as WeaveAPIError {
            handle(error)
            phase = loom.threads.isEmpty ? .failed(error.userMessage) : .ready
        } catch {
            phase = .failed(error.localizedDescription)
        }
    }

    /// Répond à un fragment. Met à jour le fil localement sans recharger tout
    /// le métier : la réponse vient d'être écrite, l'utilisateur doit la voir
    /// prise en compte immédiatement.
    public func respond(to thread: ThreadCard, fragment: Fragment, body: String) async -> Bool {
        do {
            let result = try await api.respond(
                threadID: thread.id,
                fragmentID: fragment.id,
                body: body
            )
            replace(result.thread)
            return true
        } catch let error as WeaveAPIError {
            handle(error)
            // Un fil dénoué pendant la saisie ne peut plus être affiché.
            if case .threadGone = error { remove(thread.id) }
            return false
        } catch {
            alert = .transport(error.localizedDescription)
            return false
        }
    }

    public func release(_ thread: ThreadCard, reason: String?) async {
        remove(thread.id)
        do {
            try await api.release(threadID: thread.id, reason: reason)
            await refresh()
        } catch let error as WeaveAPIError {
            // Le fil est déjà retiré de l'affichage : inutile d'alarmer si le
            // serveur répond qu'il n'existait plus.
            if case .threadGone = error { return }
            handle(error)
        } catch {
            alert = .transport(error.localizedDescription)
        }
    }

    public func extend(_ thread: ThreadCard) async {
        do {
            _ = try await api.extend(threadID: thread.id)
            await refresh()
        } catch let error as WeaveAPIError {
            handle(error)
        } catch {
            alert = .transport(error.localizedDescription)
        }
    }

    public func relais() async {
        do {
            loom = try await api.relais()
            lastUpdated = .now
        } catch let error as WeaveAPIError {
            handle(error)
        } catch {
            alert = .transport(error.localizedDescription)
        }
    }

    /// Purge les fils échus sans aller au réseau : le décompte est local, la
    /// vérité est de toute façon vérifiée au prochain rafraîchissement.
    public func dropExpired() {
        let living = loom.threads.filter { !$0.isExpired }
        guard living.count != loom.threads.count else { return }
        loom = Loom(
            threads: living,
            nextWeavingAt: loom.nextWeavingAt,
            freeSlots: Loom.maxActiveThreads - living.count,
            nextRefillAt: loom.nextRefillAt,
            fromCache: true
        )
    }

    // MARK: - Interne

    private func handle(_ error: WeaveAPIError) {
        if case .entitlementRequired(let sku, _) = error {
            suggestedPurchase = sku.flatMap(UnitSku.init(rawValue:))
        }
        alert = error
    }

    private func replace(_ thread: ThreadCard) {
        var threads = loom.threads
        guard let index = threads.firstIndex(where: { $0.id == thread.id }) else { return }
        threads[index] = thread
        loom = Loom(
            threads: threads,
            nextWeavingAt: loom.nextWeavingAt,
            freeSlots: loom.freeSlots,
            nextRefillAt: loom.nextRefillAt,
            fromCache: true
        )
    }

    private func remove(_ threadID: String) {
        let threads = loom.threads.filter { $0.id != threadID }
        loom = Loom(
            threads: threads,
            nextWeavingAt: loom.nextWeavingAt,
            freeSlots: Loom.maxActiveThreads - threads.count,
            nextRefillAt: loom.nextRefillAt,
            fromCache: true
        )
    }
}
