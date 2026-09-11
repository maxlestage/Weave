import SwiftUI
import WeaveKit

struct ReglagesView: View {
    @Environment(ModeleApplication.self) private var modele
    @Environment(\.dismiss) private var dismiss

    @State private var distance = 25
    @State private var ageMin = 18
    @State private var ageMax = 32

    var body: some View {
        NavigationStack {
            Form {
                if let moi = modele.moi {
                    Section("Vous") {
                        LabeledContent("Prénom", value: moi.displayName)
                        LabeledContent("Ville", value: moi.city)
                        LabeledContent("Offre", value: moi.tier.displayName)
                    }

                    Section {
                        LabeledContent("Aujourd'hui", value: "\(moi.requestsLeftToday)")
                    } header: {
                        Text("Demandes restantes")
                    } footer: {
                        Text("Bornées à toutes les offres, socle gratuit compris. Elles reviennent à minuit. C'est ce qui empêche d'arroser — sur Weave, une demande vaut quelque chose.")
                    }
                }

                Section {
                    Stepper("Jusqu'à \(distance) km", value: $distance, in: 1...100, step: 5)
                        .onChange(of: distance) { _, valeur in
                            Task {
                                try? await modele.api.updatePreferences(
                                    PreferencesPatch(maxDistanceKm: valeur)
                                )
                                await modele.plans.refresh()
                            }
                        }
                    Stepper("À partir de \(ageMin) ans", value: $ageMin, in: 18...98)
                    Stepper("Jusqu'à \(ageMax) ans", value: $ageMax, in: 18...99)
                } header: {
                    Text("Critères du fil")
                } footer: {
                    Text("Ils filtrent ce que vous voyez ; ils ne changent jamais l'ordre. Le fil est trié par ce qui arrive le plus tôt, puis par ce qui est le plus près.")
                }
                .onChange(of: ageMin) { _, _ in Task { await appliquerAges() } }
                .onChange(of: ageMax) { _, _ in Task { await appliquerAges() } }

                if let moi = modele.moi {
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
                    Text("La bannière affiche votre prochain plan et ce qui attend une réponse. Jamais de nom, jamais de photo, jamais de message.")
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
                ToolbarItem(placement: .confirmationAction) {
                    Button("Terminé") { dismiss() }
                }
            }
        }
    }

    private func appliquerAges() async {
        // L'API refuse un minimum supérieur au maximum : on l'évite ici plutôt
        // que d'afficher une erreur pour un réglage qu'on peut corriger seul.
        guard ageMin <= ageMax else { return }
        try? await modele.api.updatePreferences(
            PreferencesPatch(minAge: ageMin, maxAge: ageMax)
        )
        await modele.plans.refresh()
    }
}
