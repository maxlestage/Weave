import SwiftUI
import WeaveKit
#if canImport(UIKit)
import UIKit
#endif

@main
struct WeaveApp: App {
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
    let activites: ActivityController

    private(set) var moi: Me?
    private(set) var connecte = false
    private(set) var pret = false

    init() {
        let store = SessionStore(accessGroup: WeaveEnvironment.keychainAccessGroup)
        let api = WeaveAPI(baseURL: WeaveEnvironment.apiBaseURL, store: store)
        self.sessionStore = store
        self.api = api
        self.plans = PlansStore(api: api)
        self.activites = ActivityController(api: api, vendorID: Self.vendorID)
    }

    func demarrer() async {
        connecte = await sessionStore.isAuthenticated
        pret = true
        guard connecte else { return }

        activites.start()
        await chargerCompte()
        await plans.refresh()
        await synchroniserActivite()
    }

    func reprendre() async {
        guard connecte else { return }
        plans.dropPast()
        await plans.refresh()
        await chargerCompte()
        await synchroniserActivite()
    }

    func seConnecter() async {
        connecte = true
        activites.start()
        await chargerCompte()
        await plans.refresh()
        await synchroniserActivite()
    }

    func seDeconnecter() async {
        await activites.end()
        try? await api.logout()
        moi = nil
        connecte = false
    }

    private func chargerCompte() async {
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
