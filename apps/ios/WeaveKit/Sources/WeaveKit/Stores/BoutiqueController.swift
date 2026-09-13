#if canImport(StoreKit)
import Foundation
import StoreKit

/// Les achats : charger le catalogue, payer, et transmettre au serveur.
///
/// ## Ce qui manquait
///
/// `import StoreKit` n'apparaissait nulle part dans l'application.
/// `submitSubscription` et `submitUnitPurchase` existaient dans le client HTTP
/// et n'étaient appelées de nulle part. Il n'y avait pas d'écran d'offres.
/// **Rien n'était achetable** — ni un palier, ni un crédit.
///
/// ## Les deux endroits où l'on perd l'argent de quelqu'un
///
/// **`finish()` avant que le serveur ait enregistré.** Une transaction
/// terminée disparaît de la file d'Apple. Si l'appel au serveur échoue ensuite
/// — réseau coupé, application tuée —, la personne a payé et rien ne le sait
/// plus, nulle part. On ne termine donc qu'après un enregistrement réussi ;
/// une transaction non terminée revient d'elle-même au lancement suivant.
///
/// **Ne pas écouter les transactions hors bande.** Un achat peut aboutir sans
/// passer par `purchase()` : demande d'autorisation parentale, paiement
/// interrompu puis repris, renouvellement d'abonnement, achat fait sur un
/// autre appareil. Sans écoute permanente, ces transactions-là ne seraient
/// jamais transmises.
@MainActor
@Observable
public final class BoutiqueController {
    private let api: WeaveAPI

    /// Le catalogue, par identifiant de produit.
    public private(set) var produits: [String: Product] = [:]
    public private(set) var chargement = false
    /// Achat en cours, s'il y en a un.
    public private(set) var enCours: String?

    private var ecoute: Task<Void, Never>?

    public init(api: WeaveAPI) {
        self.api = api
    }

    /// Démarre l'écoute des transactions. À appeler au lancement.
    ///
    /// Avant le chargement du catalogue, et sans attendre une connexion : une
    /// transaction qui arrive pendant qu'on regarde ailleurs doit être reprise.
    public func demarrer() {
        guard ecoute == nil else { return }
        ecoute = Task { [weak self] in
            for await resultat in Transaction.updates {
                await self?.traiter(resultat)
            }
        }
    }

    public func arreter() {
        ecoute?.cancel()
        ecoute = nil
    }

    /// Charge le catalogue auprès d'Apple.
    ///
    /// Les prix ne sont jamais écrits dans l'application : ils viennent de
    /// l'App Store, dans la monnaie et au montant qui s'appliquent là où se
    /// trouve la personne. Afficher « 4,99 € » en dur mentirait partout
    /// ailleurs qu'en zone euro.
    public func charger() async {
        chargement = true
        defer { chargement = false }

        let identifiants = UnitSku.allCases.map(\.productID)
            + PlanTier.allCases.compactMap(\.productID)

        guard let charges = try? await Product.products(for: identifiants) else { return }
        produits = Dictionary(uniqueKeysWithValues: charges.map { ($0.id, $0) })
    }

    /// Achète un produit, puis transmet la transaction au serveur.
    ///
    /// Rend `true` quand l'achat a abouti ET a été enregistré. Un achat annulé
    /// rend `false` sans erreur : ce n'est pas un échec, c'est un refus.
    @discardableResult
    public func acheter(_ produit: Product) async throws -> Bool {
        enCours = produit.id
        defer { enCours = nil }

        switch try await produit.purchase() {
        case .success(let verification):
            return await traiter(verification)
        case .userCancelled:
            return false
        case .pending:
            // Attente d'une autorisation parentale. La transaction arrivera
            // par `Transaction.updates` si elle est accordée — c'est
            // exactement ce que l'écoute permanente sert à ne pas manquer.
            return false
        @unknown default:
            return false
        }
    }

    /// Rejoue les achats déjà faits, sur un nouvel appareil par exemple.
    public func restaurer() async {
        for await resultat in Transaction.currentEntitlements {
            await traiter(resultat)
        }
    }

    /// Transmet une transaction au serveur, puis la termine.
    ///
    /// L'ordre est l'essentiel de cette fonction.
    @discardableResult
    private func traiter(_ resultat: VerificationResult<Transaction>) async -> Bool {
        // Apple a déjà jugé la signature. Le serveur la revérifie de son côté
        // — il ne fait confiance à aucun client — mais transmettre ce qu'Apple
        // dit non vérifié serait transmettre du bruit.
        guard case .verified(let transaction) = resultat else { return false }

        let signee = resultat.jwsRepresentation
        do {
            if transaction.productType == .autoRenewable {
                try await api.submitSubscription(signedTransaction: signee)
            } else {
                try await api.submitUnitPurchase(signedTransaction: signee)
            }
        } catch {
            // Surtout ne pas terminer : la transaction reste dans la file
            // d'Apple et reviendra au prochain lancement par
            // `Transaction.updates`. C'est ce qui évite qu'un réseau coupé
            // fasse disparaître un achat payé.
            return false
        }

        await transaction.finish()
        return true
    }
}
#endif
