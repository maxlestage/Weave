import SwiftUI
import WeaveKit

struct ReglagesView: View {
    @Environment(ModeleApplication.self) private var modele
    @Environment(\.dismiss) private var dismiss

    /// Créneaux de tissage proposés, alignés sur `WEAVING_HOURS`.
    private let creneaux = [8, 12, 18, 21]

    @State private var heure = 18

    var body: some View {
        NavigationStack {
            Form {
                if let moi = modele.moi {
                    Section("Vous") {
                        LabeledContent("Prénom", value: moi.displayName)
                        LabeledContent("Motif", value: moi.motif.joined(separator: " · "))
                        LabeledContent("Offre", value: moi.plan.displayName)
                    }

                    Section {
                        Picker("Heure de tissage", selection: $heure) {
                            ForEach(creneaux, id: \.self) { Text("\($0) h").tag($0) }
                        }
                        .onChange(of: heure) { _, nouvelle in
                            Task { try? await modele.api.updateWeavingHour(nouvelle) }
                        }
                    } header: {
                        Text("Rendez-vous quotidien")
                    } footer: {
                        Text("C'est le seul moment où Weave vous sollicite. Vos fils arrivent à cette heure-là.")
                    }

                    Section("Crédits") {
                        ForEach(UnitSku.allCases, id: \.self) { sku in
                            LabeledContent(sku.displayName, value: "\(moi.credits(for: sku))")
                        }
                    }
                }

                Section {
                    LabeledContent(
                        "Live Activity",
                        value: modele.activites.isEnabled ? "Autorisée" : "Désactivée"
                    )
                } footer: {
                    Text("La bannière affiche le nombre de fils et le temps restant. Jamais de photo, jamais de message.")
                }

                Section {
                    Button("Se déconnecter", role: .destructive) {
                        Task {
                            await modele.seDeconnecter()
                            dismiss()
                        }
                    }
                }
            }
            .navigationTitle("Réglages")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Fermer") { dismiss() }
                }
            }
            .onAppear { heure = modele.moi?.weavingHour ?? 18 }
        }
    }
}
