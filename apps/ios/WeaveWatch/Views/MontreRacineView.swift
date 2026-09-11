import SwiftUI
import WeaveKit

struct MontreRacineView: View {
    @Environment(ModeleMontre.self) private var modele

    var body: some View {
        NavigationStack {
            List {
                if modele.resume.entries.isEmpty {
                    VidePlaceholder(chargement: modele.chargement, erreur: modele.erreur)
                } else {
                    Section {
                        ForEach(modele.resume.entries) { entree in
                            LigneFil(entree: entree)
                        }
                    } header: {
                        Text(entete)
                    }
                }
            }
            .navigationTitle("Métier")
            .refreshable { await modele.charger() }
        }
    }

    private var entete: String {
        let total = modele.resume.activeThreads
        let attente = modele.resume.awaitingYou
        return attente > 0 ? "\(total) fils · \(attente) à répondre" : "\(total) fils"
    }
}

private struct LigneFil: View {
    let entree: WatchSummary.Entry

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            HStack {
                Text(entree.name)
                    .font(.headline)
                Spacer()
                if entree.awaitingYou {
                    Circle()
                        .fill(Color.weaveCuivreMontre)
                        .frame(width: 7, height: 7)
                        .accessibilityLabel("Attend votre réponse")
                }
            }

            Text(timerInterval: Date.now...max(entree.expiresAt, .now), countsDown: true)
                .font(.caption2.monospacedDigit())
                .foregroundStyle(.secondary)
        }
        .padding(.vertical, 2)
        .accessibilityElement(children: .combine)
    }
}

private struct VidePlaceholder: View {
    let chargement: Bool
    let erreur: String?

    var body: some View {
        VStack(spacing: 8) {
            if chargement {
                ProgressView()
            } else if let erreur {
                Text(erreur)
                    .font(.caption)
                    .multilineTextAlignment(.center)
            } else {
                Text("Métier vide")
                    .font(.headline)
                Text("Vos prochains fils arrivent à votre heure de tissage.")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
            }
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 12)
    }
}
