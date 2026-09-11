import SwiftUI
import WeaveKit
import WidgetKit

@main
struct WeaveWatchApp: App {
    @State private var modele = ModeleMontre()

    var body: some Scene {
        WindowGroup {
            MontreRacineView()
                .environment(modele)
                .tint(.weaveCuivreMontre)
                .task { await modele.charger() }
        }
    }
}

/// État de l'application montre.
///
/// La montre ne reçoit qu'un résumé compact : le prochain plan et deux
/// compteurs. Ni nom, ni photo, ni message — d'abord parce que c'est inutile
/// sur un écran de cette taille, ensuite parce qu'un poignet est plus exposé au
/// regard d'autrui qu'un téléphone.
@MainActor
@Observable
final class ModeleMontre {
    private(set) var resume: WatchSummary = .empty
    private(set) var chargement = false
    private(set) var erreur: String?

    private let api: WeaveAPI

    init() {
        let store = SessionStore(accessGroup: WeaveEnvironment.keychainAccessGroup)
        self.api = WeaveAPI(baseURL: WeaveEnvironment.apiBaseURL, store: store)
    }

    func charger() async {
        chargement = true
        defer { chargement = false }
        do {
            resume = try await api.watchSummary()
            erreur = nil
            partagerAvecLaComplication()
        } catch let probleme as WeaveAPIError {
            erreur = probleme.userMessage
        } catch {
            erreur = "Connexion impossible."
        }
    }

    /// Dépose le résumé dans les préférences du groupe : la complication le lit
    /// de là plutôt que d'appeler le réseau à chaque relève de cadran.
    private func partagerAvecLaComplication() {
        guard let defaults = UserDefaults(suiteName: WeaveEnvironment.appGroup) else { return }
        let encodeur = JSONEncoder()
        encodeur.dateEncodingStrategy = .iso8601
        guard let data = try? encodeur.encode(resume) else { return }
        defaults.set(data, forKey: "watchSummary")
        WidgetCenter.shared.reloadTimelines(ofKind: "app.weave.complication")
    }
}

extension Color {
    static let weaveCuivreMontre = Color(red: 0.690, green: 0.549, blue: 1.0)
}
