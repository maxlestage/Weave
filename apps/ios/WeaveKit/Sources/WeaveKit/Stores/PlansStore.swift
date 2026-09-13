import Foundation
import Observation

/// État observable des plans, partagé par toutes les vues de l'application.
///
/// Le magasin ne persiste rien sur disque : le fil est recomposé à la demande,
/// exactement comme côté serveur où il ne vit que quelques minutes en cache.
/// L'ordre reçu n'est jamais retrié ici — imminence puis proximité, décidées
/// par le serveur et annoncées comme telles.
@MainActor
@Observable
public final class PlansStore {
    public enum Phase: Equatable {
        case idle
        case loading
        case ready
        case failed(String)
    }

    public private(set) var feed: Feed = .empty
    public private(set) var myPlans: [MyPlan] = []
    public private(set) var sent: [JoinRequest] = []
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

    public var isEmpty: Bool { feed.plans.isEmpty }

    /// Demandes restantes aujourd'hui. Affiché en permanence : c'est la seule
    /// ressource rare du produit, et la cacher reviendrait à la rendre injuste.
    public var requestsLeftToday: Int { feed.requestsLeftToday }

    /// Demandes reçues sur ses propres plans, en attente de décision.
    public var pendingRequests: Int { myPlans.reduce(0) { $0 + $1.pendingRequests } }

    public func refresh() async {
        if phase == .idle { phase = .loading }
        do {
            async let fil = api.feed()
            async let miens = api.myPlans()
            feed = try await fil
            myPlans = try await miens
            lastUpdated = .now
            phase = .ready
        } catch let error as WeaveAPIError {
            handle(error)
            phase = feed.plans.isEmpty ? .failed(error.userMessage) : .ready
        } catch {
            phase = .failed(error.localizedDescription)
        }
    }

    public func refreshSent() async {
        do {
            sent = try await api.sentRequests().requests
        } catch let error as WeaveAPIError {
            handle(error)
        } catch {
            alert = .transport(error.localizedDescription)
        }
    }

    /// Publie un plan. Le fil des autres est invalidé côté serveur ; ici on
    /// recharge seulement les siens.
    public func publish(_ draft: PlanDraft) async -> Bool {
        do {
            _ = try await api.publish(draft)
            myPlans = try await api.myPlans()
            return true
        } catch let error as WeaveAPIError {
            handle(error)
            return false
        } catch {
            alert = .transport(error.localizedDescription)
            return false
        }
    }

    public func cancel(_ plan: MyPlan) async {
        myPlans.removeAll { $0.id == plan.id }
        do {
            try await api.cancelPlan(id: plan.id)
        } catch let error as WeaveAPIError {
            // Le plan est déjà retiré de l'affichage : inutile d'alarmer si le
            // serveur répond qu'il n'existait plus.
            if case .notFound = error { return }
            handle(error)
        } catch {
            alert = .transport(error.localizedDescription)
        }
    }

    /// Demande à venir. Marque le plan comme demandé sur place et décrémente le
    /// compteur : la personne vient d'écrire, elle doit voir ce que ça coûte.
    public func join(_ plan: Plan, message: String) async -> Bool {
        do {
            let envoyee = try await api.join(planID: plan.id, message: message)
            markRequested(plan.id, requestsLeft: envoyee.requestsLeftToday)
            return true
        } catch let error as WeaveAPIError {
            handle(error)
            // Un plan qui vient de se remplir ou d'être annulé n'a plus à
            // figurer dans le fil.
            if case .planClosed = error { remove(plan.id) }
            return false
        } catch {
            alert = .transport(error.localizedDescription)
            return false
        }
    }

    public func withdraw(_ request: JoinRequest) async {
        do {
            try await api.withdraw(requestID: request.id)
            await refreshSent()
            feed = try await api.feed()
        } catch let error as WeaveAPIError {
            handle(error)
        } catch {
            alert = .transport(error.localizedDescription)
        }
    }

    // MARK: - Côté auteur

    public func incoming(for plan: MyPlan) async -> [IncomingRequest] {
        do {
            return try await api.incomingRequests(planID: plan.id)
        } catch let error as WeaveAPIError {
            handle(error)
            return []
        } catch {
            alert = .transport(error.localizedDescription)
            return []
        }
    }

    /// Accepte une demande et renvoie la conversation ouverte.
    public func accept(_ request: IncomingRequest) async -> String? {
        do {
            let conversationID = try await api.accept(requestID: request.id)
            myPlans = try await api.myPlans()
            return conversationID
        } catch let error as WeaveAPIError {
            handle(error)
            return nil
        } catch {
            alert = .transport(error.localizedDescription)
            return nil
        }
    }

    public func decline(_ request: IncomingRequest) async {
        do {
            try await api.decline(requestID: request.id)
            myPlans = try await api.myPlans()
        } catch let error as WeaveAPIError {
            handle(error)
        } catch {
            alert = .transport(error.localizedDescription)
        }
    }

    /// Retire du fil les plans dont l'heure est passée, sans aller au réseau.
    /// La vérité est de toute façon revue au prochain rafraîchissement.
    public func dropPast() {
        let vivants = feed.plans.filter { $0.startsAt > .now }
        guard vivants.count != feed.plans.count else { return }
        feed = Feed(
            plans: vivants,
            requestsLeftToday: feed.requestsLeftToday,
            fromCache: true,
            generatedAt: feed.generatedAt
        )
    }

    // MARK: - Interne

    private func handle(_ error: WeaveAPIError) {
        if case .entitlementRequired(_, let sku, _) = error {
            // `sku` est absent quand le refus porte sur un palier plutôt que
            // sur un crédit : il n'y a alors rien de précis à proposer, et
            // suggérer un achat au hasard vaudrait moins que rien.
            suggestedPurchase = sku.flatMap(UnitSku.init(rawValue:))
        }
        alert = error
    }

    private func markRequested(_ planID: String, requestsLeft: Int) {
        let plans = feed.plans.map { plan -> Plan in
            guard plan.id == planID else { return plan }
            return Plan(
                id: plan.id,
                author: plan.author,
                title: plan.title,
                note: plan.note,
                category: plan.category,
                startsAt: plan.startsAt,
                city: plan.city,
                distanceKm: plan.distanceKm,
                capacity: plan.capacity,
                seatsLeft: plan.seatsLeft,
                state: plan.state,
                requested: true,
                createdAt: plan.createdAt
            )
        }
        feed = Feed(
            plans: plans,
            requestsLeftToday: requestsLeft,
            fromCache: true,
            generatedAt: feed.generatedAt
        )
    }

    private func remove(_ planID: String) {
        feed = Feed(
            plans: feed.plans.filter { $0.id != planID },
            requestsLeftToday: feed.requestsLeftToday,
            fromCache: true,
            generatedAt: feed.generatedAt
        )
    }
}
