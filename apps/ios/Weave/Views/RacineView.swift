import SwiftUI
import WeaveKit

struct RacineView: View {
    @Environment(ModeleApplication.self) private var modele

    var body: some View {
        Group {
            if !modele.pret {
                ProgressView().controlSize(.large)
            } else if modele.connecte {
                Onglets()
            } else {
                ConnexionView()
            }
        }
        .animation(.easeInOut(duration: 0.25), value: modele.connecte)
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
