import SwiftUI
import WeaveKit

struct RacineView: View {
    @Environment(ModeleApplication.self) private var modele

    var body: some View {
        Group {
            if !modele.pret {
                ProgressView().controlSize(.large)
            } else if modele.connecte {
                // Un compte sans fiche ne peut rien faire : le fil est vide et
                // la publication refusée. On demande donc la fiche avant les
                // onglets, plutôt que de laisser quelqu'un devant une
                // application qui ne répond pas.
                if modele.moi?.status == .onboarding {
                    FicheView()
                } else {
                    Onglets()
                }
            } else {
                ConnexionView()
            }
        }
        .animation(.easeInOut(duration: 0.25), value: modele.connecte)
        .animation(.easeInOut(duration: 0.25), value: modele.moi?.status)
    }
}

/// Trois onglets, et c'est tout : ce qui se passe autour, ce qu'on propose, ce
/// qui en est sorti. Il n'y a pas d'onglet « découvrir ».
private struct Onglets: View {
    @Environment(ModeleApplication.self) private var modele
    @State private var reglages = false

    var body: some View {
        TabView {
            FilView()
                .tabItem { Label("Autour", systemImage: "calendar") }

            MesPlansView()
                .tabItem { Label("Mes plans", systemImage: "square.and.pencil") }
                .badge(modele.plans.pendingRequests)

            ConversationsView()
                .tabItem { Label("Suites", systemImage: "bubble.left.and.bubble.right") }
        }
        .overlay(alignment: .topTrailing) {
            Button {
                reglages = true
            } label: {
                Image(systemName: "person.crop.circle")
                    .font(.title3)
                    .padding(12)
            }
            .accessibilityLabel("Réglages")
        }
        .sheet(isPresented: $reglages) {
            ReglagesView().environment(modele)
        }
    }
}
