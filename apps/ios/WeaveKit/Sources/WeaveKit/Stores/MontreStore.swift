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

    // MARK: - Se connecter depuis la montre

    /// L'issue d'une connexion faite au poignet.
    public enum IssueConnexion: Equatable, Sendable {
        case connecte
        /// Le code est bon, mais aucun compte ne porte cette adresse : il se
        /// crée sur l'iPhone, où l'on choisit son prénom et sa photo.
        case compteAbsent
        case echec(String)
    }

    /// Demande un code à six chiffres pour cette adresse. Rend le message à
    /// afficher si la demande échoue, `nil` sinon.
    ///
    /// La montre attendait que l'iPhone lui transmette une session. Si ce
    /// relais échouait — application iPhone jamais rouverte, montre jumelée
    /// après coup, Bluetooth coupé — elle restait bloquée sur « Ouvrez Weave
    /// sur votre iPhone », sans autre issue. Elle se connecte désormais comme
    /// l'iPhone : une adresse, un code reçu par e-mail.
    public func demanderCode(email: String) async -> String? {
        do {
            try await api.requestCode(email: email.trimmingCharacters(in: .whitespaces))
            return nil
        } catch let probleme as WeaveAPIError {
            return probleme.userMessage
        } catch {
            return NSLocalizedString("Envoi impossible. Réessayez.", comment: "")
        }
    }

    /// Vérifie le code et ouvre la session de la montre.
    public func seConnecter(email: String, code: String) async -> IssueConnexion {
        do {
            let resultat = try await api.verifyCode(
                email: email.trimmingCharacters(in: .whitespaces),
                code: code.trimmingCharacters(in: .whitespaces)
            )
            if resultat.needsProfile || resultat.session == nil {
                return .compteAbsent
            }
            aUneSession = true
            erreur = nil
            await charger()
            return .connecte
        } catch let probleme as WeaveAPIError {
            return .echec(probleme.userMessage)
        } catch {
            return .echec(NSLocalizedString("Vérification impossible. Réessayez.", comment: ""))
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
            // s'est déconnecté partout. L'iPhone en redonnera une, ou l'on se
            // reconnecte depuis la montre.
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
