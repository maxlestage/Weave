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
                    // Au retour au premier plan, les fils ont pu se dénouer
                    // pendant l'absence : on revérifie plutôt que d'afficher
                    // un décompte figé.
                    if phase == .active {
                        Task { await modele.reprendre() }
                    }
                }
        }
    }
}

/// État global : session, métier, Live Activity.
@MainActor
@Observable
final class ModeleApplication {
    let sessionStore: SessionStore
    let api: WeaveAPI
    let loom: LoomStore
    let activites: ActivityController

    private(set) var moi: Me?
    private(set) var connecte = false
    private(set) var pret = false

    init() {
        let store = SessionStore(accessGroup: WeaveEnvironment.keychainAccessGroup)
        let api = WeaveAPI(baseURL: WeaveEnvironment.apiBaseURL, store: store)
        self.sessionStore = store
        self.api = api
        self.loom = LoomStore(api: api)
        self.activites = ActivityController(api: api, vendorID: Self.vendorID)
    }

    func demarrer() async {
        connecte = await sessionStore.isAuthenticated
        pret = true
        guard connecte else { return }

        activites.start()
        await chargerCompte()
        await loom.refresh()
        await synchroniserActivite()
    }

    func reprendre() async {
        guard connecte else { return }
        loom.dropExpired()
        await loom.refresh()
        await synchroniserActivite()
    }

    func seConnecter() async {
        connecte = true
        activites.start()
        await chargerCompte()
        await loom.refresh()
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

    /// Aligne la Live Activity sur l'état réel du métier.
    private func synchroniserActivite() async {
        guard let moi else { return }
        let etat = WeaveActivityAttributes.ContentState(
            activeThreads: loom.loom.threads.count,
            awaitingYou: loom.loom.awaitingYou.count,
            soonestExpiryAt: loom.loom.soonest?.expiresAt,
            soonestName: loom.loom.soonest?.displayName,
            nextRefillAt: loom.loom.nextRefillAt
        )
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
    static let weaveCuivre = Color(red: 0.706, green: 0.396, blue: 0.165)
    static let weaveLin = Color(red: 0.980, green: 0.969, blue: 0.949)
    static let weaveEncre = Color(red: 0.106, green: 0.090, blue: 0.078)
}
