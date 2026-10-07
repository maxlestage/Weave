import SwiftUI
import WeaveKit
import WidgetKit

@main
struct WeaveWatchApp: App {
    @State private var modele = ModeleMontre()
    @Environment(\.scenePhase) private var scenePhase

    var body: some Scene {
        WindowGroup {
            MontreRacineView()
                .environment(modele)
                .tint(.weaveCuivreMontre)
                .task { await modele.demarrer() }
                .onChange(of: scenePhase) { _, phase in
                    if phase == .active { Task { await modele.rafraichir() } }
                }
        }
    }
}

/// L'application montre : un magasin éprouvé ailleurs, et un lien vers
/// l'iPhone.
///
/// ## Ce qu'un poignet montre, et quand
///
/// Le cadran et la complication ne portent ni nom, ni photo, ni message : un
/// titre, une heure, deux compteurs. Un poignet est plus exposé au regard
/// d'autrui qu'un téléphone, et ce qui s'y affiche sans qu'on l'ait demandé
/// doit pouvoir être lu par quelqu'un d'autre.
///
/// Mais le site promet qu'on y accepte une demande « à la volée », et qu'on y
/// répond « par dictée ». Accepter quelqu'un sans lire ce qu'il a écrit ne
/// serait pas une décision ; répondre sans lire ce qu'on vous a dit non plus.
/// Les prénoms et les messages s'affichent donc — mais seulement APRÈS un
/// geste délibéré, jamais sur le cadran, et marqués `privacySensitive` : le
/// système les masque dès que le poignet retombe ou que l'écran passe en
/// veille permanente. Pas de photo : elle n'apporte rien à cette taille, et
/// c'est ce qui se reconnaît de plus loin.
@MainActor
@Observable
final class ModeleMontre {
    let magasin: MontreStore
    @ObservationIgnored private var lien: LienMontre?

    init() {
        let session = SessionStore(accessGroup: WeaveEnvironment.keychainAccessGroup)
        let api = WeaveAPI(baseURL: WeaveEnvironment.apiBaseURL, store: session)
        magasin = MontreStore(api: api, magasin: session)
    }

    func demarrer() async {
        let lien = LienMontre { [weak self] message in
            guard let self else { return }
            await self.magasin.recevoir(message)
            self.partagerAvecLaComplication()
        }
        lien.demarrer()
        self.lien = lien

        await magasin.demarrer()
        demanderUneSessionSiBesoin()
        partagerAvecLaComplication()
    }

    func rafraichir() async {
        await magasin.charger()
        demanderUneSessionSiBesoin()
        partagerAvecLaComplication()
    }

    /// Se connecte depuis la montre, puis met la complication à jour.
    func connecter(email: String, code: String) async -> MontreStore.IssueConnexion {
        let issue = await magasin.seConnecter(email: email, code: code)
        if issue == .connecte { partagerAvecLaComplication() }
        return issue
    }

    /// Sans session, l'iPhone peut en donner une : on la lui demande. La
    /// demande attend en file si l'iPhone n'est pas joignable ; en attendant,
    /// on peut aussi se connecter depuis la montre.
    private func demanderUneSessionSiBesoin() {
        guard !magasin.aUneSession else { return }
        lien?.envoyer(.demandeDeSession)
    }

    /// Dépose le résumé dans le trousseau partagé : la complication le lit de
    /// là plutôt que d'appeler le réseau à chaque relève de cadran.
    private func partagerAvecLaComplication() {
        ResumeComplication.deposer(magasin.resume, accessGroup: WeaveEnvironment.keychainAccessGroup)
        WidgetCenter.shared.reloadTimelines(ofKind: "app.weave.complication")
    }
}

extension Color {
    static let weaveCuivreMontre = Color(red: 0.690, green: 0.549, blue: 1.0)
}
