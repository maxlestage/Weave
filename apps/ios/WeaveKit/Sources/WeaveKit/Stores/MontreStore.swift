import Foundation
import Observation

/// L'état de la montre : ce qu'elle affiche, et ce qu'elle permet de faire.
///
/// ## Ce que la montre ne faisait pas
///
/// Le site promet « de quoi accepter une demande à la volée, ou dicter deux
/// phrases avant de repartir », avec une « synchronisation par
/// WatchConnectivity, et repli sur le réseau ». La montre affichait deux
/// compteurs, et rien d'autre — et encore : elle n'avait aucune session, son
/// trousseau n'étant pas celui de l'iPhone. Son seul écran était une erreur.
///
/// ## Deux sources, une règle
///
/// Le résumé arrive par deux chemins : l'iPhone le pousse par
/// WatchConnectivity (immédiat, sans réseau sur la montre), et la montre le
/// demande elle-même au serveur quand elle le peut. Le PLUS RÉCENT l'emporte,
/// selon l'heure à laquelle le serveur l'a composé — pas selon l'ordre
/// d'arrivée : un résumé poussé par l'iPhone peut arriver après un résumé
/// réseau plus frais, et l'afficher ferait reculer l'écran.
///
/// La logique vit ici, dans WeaveKit, où un test l'éprouve. Le transport
/// (`WCSession`) et les écrans restent dans la cible de la montre.
@MainActor
@Observable
public final class MontreStore {
    /// Une demande reçue, avec le plan qu'elle vise.
    public struct DemandeATraiter: Identifiable, Hashable, Sendable {
        public let demande: IncomingRequest
        public let planTitle: String
        public let planStartsAt: Date
        public var id: String { demande.id }
    }

    public private(set) var resume: WatchSummary = .empty
    public private(set) var demandes: [DemandeATraiter] = []
    /// La montre a-t-elle sa session ? Sans elle, seul l'iPhone peut lui
    /// parler, et l'écran doit le dire.
    public private(set) var aUneSession = false
    public private(set) var chargement = false
    public var erreur: String?

    public let conversations: ConversationsStore

    private let api: WeaveAPI
    private let magasin: SessionStore

    public init(api: WeaveAPI, magasin: SessionStore) {
        self.api = api
        self.magasin = magasin
        self.conversations = ConversationsStore(api: api)
    }

    /// À l'ouverture : relit la session, et va au réseau si elle existe.
    public func demarrer() async {
        aUneSession = await magasin.isAuthenticated
        if aUneSession { await charger() }
    }

    /// Ce qui arrive de l'iPhone.
    public func recevoir(_ message: MessageMontre) async {
        switch message {
        case .session(let session):
            await magasin.save(session)
            aUneSession = true
            erreur = nil
            await charger()
        case .resume(let nouveau):
            retenir(nouveau)
        case .deconnexion:
            // L'iPhone s'est déconnecté : la montre ne garde rien. Un poignet
            // qui continuerait d'afficher les plans d'un compte dont on vient
            // de sortir serait une fuite, pas une commodité.
            await magasin.clear()
            aUneSession = false
            resume = .empty
            demandes = []
        case .demandeDeSession:
            // C'est la montre qui la pose, elle ne la reçoit pas.
            break
        }
    }

    /// Le repli sur le réseau.
    public func charger() async {
        guard aUneSession else { return }
        chargement = true
        defer { chargement = false }
        do {
            retenir(try await api.watchSummary())
            erreur = nil
        } catch WeaveAPIError.unauthorized {
            // Session révoquée — une autre montre l'a remplacée, ou le compte
            // s'est déconnecté partout. Seul l'iPhone peut en redonner une.
            aUneSession = false
        } catch let probleme as WeaveAPIError {
            erreur = probleme.userMessage
        } catch {
            erreur = NSLocalizedString("Connexion impossible.", comment: "")
        }
    }

    /// Les demandes en attente, sur tous ses plans, la plus ancienne d'abord :
    /// elle attend depuis le plus longtemps.
    public func chargerDemandes() async {
        guard aUneSession else { return }
        do {
            var trouvees: [DemandeATraiter] = []
            for plan in try await api.myPlans() where plan.pendingRequests > 0 {
                for demande in try await api.incomingRequests(planID: plan.id) {
                    trouvees.append(DemandeATraiter(
                        demande: demande,
                        planTitle: plan.title,
                        planStartsAt: plan.startsAt
                    ))
                }
            }
            demandes = trouvees.sorted { $0.demande.sentAt < $1.demande.sentAt }
        } catch let probleme as WeaveAPIError {
            erreur = probleme.userMessage
        } catch {
            erreur = NSLocalizedString("Connexion impossible.", comment: "")
        }
    }

    /// Accepte une demande. Rend la conversation ouverte, ou `nil`.
    @discardableResult
    public func accepter(_ demande: DemandeATraiter) async -> String? {
        do {
            let conversation = try await api.accept(requestID: demande.id)
            demandes.removeAll { $0.id == demande.id }
            await charger()
            return conversation
        } catch let probleme as WeaveAPIError {
            erreur = probleme.userMessage
            return nil
        } catch {
            erreur = NSLocalizedString("Connexion impossible.", comment: "")
            return nil
        }
    }

    /// Refuse une demande. Rien n'est dit à la personne, comme sur l'iPhone.
    public func refuser(_ demande: DemandeATraiter) async {
        do {
            try await api.decline(requestID: demande.id)
            demandes.removeAll { $0.id == demande.id }
            await charger()
        } catch let probleme as WeaveAPIError {
            erreur = probleme.userMessage
        } catch {
            erreur = NSLocalizedString("Connexion impossible.", comment: "")
        }
    }

    /// Garde le plus récent des deux résumés.
    private func retenir(_ nouveau: WatchSummary) {
        guard nouveau.generatedAt >= resume.generatedAt else { return }
        resume = nouveau
    }
}
