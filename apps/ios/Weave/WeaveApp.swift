import SwiftUI
import WeaveKit
#if canImport(UIKit)
import UIKit
#endif

/// Le délégué n'existe que pour une chose : APNs ne remet le jeton d'appareil
/// nulle part ailleurs. SwiftUI seul ne donne pas accès à ce rappel.
final class DelegueApplication: NSObject, UIApplicationDelegate {
    /// Posé par `ModeleApplication` à sa création.
    @MainActor static var surJetonRecu: ((Data) async -> Void)?

    func application(
        _ application: UIApplication,
        didRegisterForRemoteNotificationsWithDeviceToken deviceToken: Data
    ) {
        Task { @MainActor in await Self.surJetonRecu?(deviceToken) }
    }

    func application(
        _ application: UIApplication,
        didFailToRegisterForRemoteNotificationsWithError error: any Error
    ) {
        // Sans jeton, pas d'alerte — mais tout le reste continue de marcher.
        // Échouer bruyamment ici empêcherait d'utiliser l'application pour un
        // confort dont elle peut se passer.
    }
}

@main
struct WeaveApp: App {
    @UIApplicationDelegateAdaptor(DelegueApplication.self) private var delegue
    @State private var modele = ModeleApplication()
    @Environment(\.scenePhase) private var scenePhase

    var body: some Scene {
        WindowGroup {
            RacineView()
                .environment(modele)
                .tint(.weaveCuivre)
                .task { await modele.demarrer() }
                .onChange(of: scenePhase) { _, phase in
                    // Au retour au premier plan, des plans ont pu passer et des
                    // demandes arriver : on revérifie plutôt que d'afficher un
                    // fil figé.
                    if phase == .active {
                        Task { await modele.reprendre() }
                    }
                }
        }
    }
}

/// État global : session, plans, Live Activity.
@MainActor
@Observable
final class ModeleApplication {
    let sessionStore: SessionStore
    let api: WeaveAPI
    let plans: PlansStore
    let conversations: ConversationsStore
    let activites: ActivityController
    let notifications: NotificationsController
    let boutique: BoutiqueController

    /// Le lien avec la montre. Ignoré par l'observation : c'est un tuyau,
    /// pas un état que les vues affichent.
    @ObservationIgnored private var lienMontre: LienMontre?

    private(set) var moi: Me?
    private(set) var connecte = false
    private(set) var pret = false

    init() {
        let store = SessionStore(accessGroup: WeaveEnvironment.keychainAccessGroup)
        let api = WeaveAPI(baseURL: WeaveEnvironment.apiBaseURL, store: store)
        self.sessionStore = store
        self.api = api
        self.plans = PlansStore(api: api)
        self.conversations = ConversationsStore(api: api)
        self.boutique = BoutiqueController(api: api)
        self.activites = ActivityController(api: api, vendorID: Self.vendorID)
        let notifications = NotificationsController(api: api, vendorID: Self.vendorID)
        self.notifications = notifications

        // APNs remet le jeton au délégué, pas ici : on lui dit où l'apporter.
        DelegueApplication.surJetonRecu = { [notifications] jeton in
            await notifications.deposer(jeton: jeton)
        }

        // L'écoute des transactions démarre AVANT toute connexion, et ne
        // s'arrête pas.
        //
        // Un achat peut aboutir sans passer par le bouton : autorisation
        // parentale accordée plus tard, paiement interrompu puis repris,
        // renouvellement d'abonnement, achat fait sur un autre appareil. Sans
        // écoute permanente, ces transactions-là ne seraient jamais transmises
        // au serveur — quelqu'un aurait payé sans rien recevoir.
        boutique.demarrer()

        // La montre demande sa session quand elle n'en a pas — à son premier
        // lancement, ou quand le serveur a révoqué la sienne.
        let lien = LienMontre { [weak self] message in
            await self?.recuDeLaMontre(message)
        }
        lien.demarrer()
        lienMontre = lien
    }

    func demarrer() async {
        connecte = await sessionStore.isAuthenticated
        pret = true
        guard connecte else { return }

        activites.start()
        await notifications.demanderAutorisation()
        await rafraichirMoi()
        await plans.refresh()
        await synchroniserActivite()
        await equiperLaMontre()
        await envoyerResumeALaMontre()
    }

    func reprendre() async {
        guard connecte else { return }
        plans.dropPast()
        await plans.refresh()
        await rafraichirMoi()
        await synchroniserActivite()
        // Au premier lancement, WatchConnectivity n'était peut-être pas encore
        // activé : on retente à chaque retour au premier plan, ce qui ne coûte
        // rien une fois la session donnée.
        await equiperLaMontre()
        await envoyerResumeALaMontre()
    }

    func seConnecter() async {
        // Le compte est relu AVANT d'afficher quoi que ce soit.
        //
        // C'est lui qui porte le statut, et donc le choix entre les onglets et
        // la fiche à remplir. Basculer `connecte` d'abord montrait un instant
        // un fil vide et des onglets inertes à qui vient de s'inscrire, avant
        // que l'écran ne se ravise.
        await rafraichirMoi()
        connecte = true
        activites.start()
        // L'autorisation se demande ici, pas au premier lancement : avant
        // d'avoir un compte, il n'y a rien à notifier, et une demande posée
        // trop tôt se refuse par réflexe. iOS ne la repose jamais.
        await notifications.demanderAutorisation()
        await plans.refresh()
        await synchroniserActivite()
        // Un nouveau compte : la montre doit recevoir une session à lui.
        UserDefaults.standard.set(false, forKey: Self.cleSessionMontreDonnee)
        await equiperLaMontre()
        await envoyerResumeALaMontre()
    }

    func seDeconnecter() async {
        await activites.end()
        await notifications.oublier()
        // La montre oublie tout. Le serveur révoque de son côté la session de
        // la montre avec celle-ci : le message ne fait que lui éviter
        // d'afficher, en attendant, ce qu'elle n'a plus le droit de lire.
        lienMontre?.envoyer(.deconnexion)
        UserDefaults.standard.set(false, forKey: Self.cleSessionMontreDonnee)
        try? await api.logout()
        moi = nil
        connecte = false
    }

    // MARK: - La montre

    private static let cleSessionMontreDonnee = "montre.sessionDonnee"

    /// Donne à la montre SA session, si elle n'en a pas encore reçu.
    ///
    /// Jamais une copie de celle-ci : les jetons de renouvellement tournent,
    /// et deux appareils qui partagent le même se font prendre l'un pour le
    /// voleur de l'autre — le serveur coupe alors tout le compte. Le serveur
    /// ouvre donc une session propre à la montre (`watchSession`).
    ///
    /// Une fois donnée, on ne la redonne pas à chaque lancement : chaque
    /// session ouverte révoque la précédente, et en fabriquer une à chaque
    /// ouverture de l'iPhone ferait tourner la montre pour rien. C'est la
    /// montre qui en redemande une quand la sienne ne vaut plus.
    private func equiperLaMontre(forcer: Bool = false) async {
        guard connecte, let lienMontre, lienMontre.autreAppareilPresent else { return }
        if !forcer, UserDefaults.standard.bool(forKey: Self.cleSessionMontreDonnee) { return }
        guard let session = try? await api.watchSession() else { return }
        lienMontre.envoyer(.session(session))
        UserDefaults.standard.set(true, forKey: Self.cleSessionMontreDonnee)
    }

    /// Pousse le résumé à la montre : elle l'affiche sans attendre le réseau.
    ///
    /// Le résumé vient du serveur, comme celui que la montre demanderait
    /// elle-même : deux compositions différentes du même état finiraient par
    /// se contredire au poignet.
    private func envoyerResumeALaMontre() async {
        guard connecte, let lienMontre, lienMontre.autreAppareilPresent else { return }
        guard let resume = try? await api.watchSummary() else { return }
        lienMontre.envoyer(.resume(resume))
    }

    private func recuDeLaMontre(_ message: MessageMontre) async {
        if case .demandeDeSession = message {
            await equiperLaMontre(forcer: true)
        }
    }

    /// Recharge le résumé du compte.
    ///
    /// Accessible aux vues : c'est `moi` qui porte le statut, le palier et les
    /// crédits, et une action qui les change — une pause, un achat — doit
    /// pouvoir le redemander plutôt que d'attendre le prochain lancement.
    func rafraichirMoi() async {
        moi = try? await api.me()
    }

    /// Aligne la Live Activity sur l'état réel.
    ///
    /// Le serveur reste la source : on lui demande l'état plutôt que de le
    /// recomposer ici, sinon la bannière affichée localement et celle poussée
    /// par APNs finiraient par diverger.
    private func synchroniserActivite() async {
        guard let moi else { return }
        guard let etat = try? await api.liveActivityState() else { return }
        await activites.startLocally(handle: moi.handle, state: etat)
    }

    /// Identifiant stable de l'appareil, tel que fourni par le système.
    private static var vendorID: String {
        #if os(iOS)
        UIDevice.current.identifierForVendor?.uuidString ?? UUID().uuidString
        #else
        UUID().uuidString
        #endif
    }
}

extension Color {
    /// La couleur d'accent, reprise du site (`--fil-6`, iris).
    static let weaveCuivre = Color(red: 0.384, green: 0.192, blue: 0.780)
    /// Le fond, très légèrement chaud : lisible dehors, reposant dedans.
    static let weaveLin = Color(red: 1.0, green: 0.992, blue: 0.976)
    static let weaveEncre = Color(red: 0.086, green: 0.071, blue: 0.122)
}
