import SwiftUI
import WeaveKit

/// Les demandes reçues, à accepter ou refuser du poignet.
///
/// Le prénom et le message s'affichent — on ne décide pas de faire venir
/// quelqu'un sans lire ce qu'il a écrit — mais seulement ici, après un geste,
/// et masqués par le système dès que le poignet retombe.
struct DemandesMontreView: View {
    @Environment(ModeleMontre.self) private var modele

    var body: some View {
        List {
            if modele.magasin.demandes.isEmpty {
                Text("Aucune demande en attente.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            ForEach(modele.magasin.demandes) { demande in
                NavigationLink {
                    DemandeMontreView(demande: demande)
                } label: {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(demande.planTitle)
                            .font(.caption2)
                            .foregroundStyle(Color.weaveCuivreMontre)
                            .lineLimit(1)
                        Text(demande.demande.author.displayName)
                            .font(.headline)
                            .privacySensitive()
                    }
                }
            }
        }
        .navigationTitle("Demandes")
        .task { await modele.magasin.chargerDemandes() }
    }
}

private struct DemandeMontreView: View {
    let demande: MontreStore.DemandeATraiter

    @Environment(ModeleMontre.self) private var modele
    @Environment(\.dismiss) private var dismiss
    @State private var enCours = false

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 10) {
                Text(demande.planTitle)
                    .font(.caption2)
                    .foregroundStyle(Color.weaveCuivreMontre)
                Text(demande.demande.author.displayName)
                    .font(.headline)
                    .privacySensitive()
                Text(demande.demande.message)
                    .font(.body)
                    .privacySensitive()

                if let erreur = modele.magasin.erreur {
                    Text(erreur)
                        .font(.caption2)
                        .foregroundStyle(.red)
                }

                Button {
                    Task { await decider(accepter: true) }
                } label: {
                    Label("Accepter", systemImage: "checkmark")
                        .frame(maxWidth: .infinity)
                }
                .tint(.weaveCuivreMontre)
                .disabled(enCours)

                // Un refus ne se commente pas et ne prévient de rien
                // d'accusateur — pas plus au poignet qu'au téléphone.
                Button(role: .destructive) {
                    Task { await decider(accepter: false) }
                } label: {
                    Label("Refuser", systemImage: "xmark")
                        .frame(maxWidth: .infinity)
                }
                .disabled(enCours)
            }
        }
        .navigationTitle("Demande")
    }

    private func decider(accepter: Bool) async {
        enCours = true
        defer { enCours = false }
        modele.magasin.erreur = nil
        if accepter {
            await modele.magasin.accepter(demande)
        } else {
            await modele.magasin.refuser(demande)
        }
        if modele.magasin.erreur == nil { dismiss() }
    }
}
