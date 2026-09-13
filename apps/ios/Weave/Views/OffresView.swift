import StoreKit
import SwiftUI
import WeaveKit

/// Les offres : paliers d'abonnement et achats à l'unité.
///
/// ## Ce qu'on ne vend pas, et pourquoi c'est dit ici
///
/// Aucun palier ne fait remonter un plan. Il n'existe aucun produit de
/// « mise en avant », et le plafond de trois plans ouverts ne se desserre
/// avec aucune somme. Ce qui se paie, c'est l'horizon de publication, la
/// finesse des critères et les plans de groupe.
///
/// C'est écrit sur l'écran plutôt que dans un document : c'est ici qu'on se
/// demande ce que l'argent change, et une promesse qui n'est pas là où on se
/// pose la question ne sert à rien.
///
/// ## Les prix viennent d'Apple
///
/// Jamais écrits dans l'application : `Product.displayPrice` les rend dans la
/// monnaie et au montant qui s'appliquent là où se trouve la personne.
/// Afficher « 4,99 € » en dur mentirait partout ailleurs qu'en zone euro.
struct OffresView: View {
    @Environment(ModeleApplication.self) private var modele
    @Environment(\.dismiss) private var dismiss

    @State private var annuel = false
    @State private var erreur: String?

    var body: some View {
        NavigationStack {
            List {
                Section {
                    Picker("Rythme", selection: $annuel) {
                        Text("Mensuel").tag(false)
                        Text("Annuel").tag(true)
                    }
                    .pickerStyle(.segmented)
                } footer: {
                    Text("Un abonnement se résilie dans les réglages de votre compte Apple, pas ici — c'est Apple qui l'encaisse.")
                }

                Section {
                    ForEach(PlanTier.allCases.filter { $0 != .depart }, id: \.self) { palier in
                        ligneAbonnement(palier)
                    }
                } header: {
                    Text("Abonnements")
                } footer: {
                    Text("Aucun palier ne fait remonter vos plans, et le plafond de trois plans ouverts ne se desserre pour aucune somme. Ce qui se paie, c'est l'horizon de publication, la finesse des critères et les plans de groupe.")
                }

                Section {
                    ForEach(UnitSku.allCases, id: \.self) { sku in
                        ligneUnite(sku)
                    }
                } header: {
                    Text("À l'unité")
                } footer: {
                    Text("Achetés une fois, utilisés quand vous voulez. Ils ne se périment pas.")
                }

                if modele.boutique.produits.isEmpty && !modele.boutique.chargement {
                    Section {
                        Text("Le catalogue n'a pas pu être chargé. Vérifiez votre connexion.")
                            .foregroundStyle(.secondary)
                    }
                }

                Section {
                    Button("Restaurer mes achats") {
                        Task { await modele.boutique.restaurer(); await modele.rafraichirMoi() }
                    }
                } footer: {
                    Text("Sur un nouvel appareil, cela retrouve ce que vous avez déjà payé.")
                }
            }
            .navigationTitle("Offres")
            .navigationBarTitleDisplayMode(.inline)
            .overlay {
                if modele.boutique.chargement && modele.boutique.produits.isEmpty {
                    ProgressView().controlSize(.large)
                }
            }
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Terminé") { dismiss() }
                }
            }
            .alert("Échec", isPresented: .constant(erreur != nil)) {
                Button("D'accord") { erreur = nil }
            } message: {
                Text(erreur ?? "")
            }
            .task { await modele.boutique.charger() }
        }
    }

    @ViewBuilder
    private func ligneAbonnement(_ palier: PlanTier) -> some View {
        let identifiant = annuel ? palier.productIDs?.yearly : palier.productIDs?.monthly
        let produit = identifiant.flatMap { modele.boutique.produits[$0] }
        let actuel = modele.moi?.tier == palier

        Button {
            guard let produit else { return }
            Task { await acheter(produit) }
        } label: {
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text(palier.displayName)
                        .foregroundStyle(.primary)
                    if actuel {
                        Text("Votre offre actuelle")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
                Spacer()
                prix(produit, enCours: modele.boutique.enCours == produit?.id)
            }
        }
        .disabled(produit == nil || actuel || modele.boutique.enCours != nil)
    }

    @ViewBuilder
    private func ligneUnite(_ sku: UnitSku) -> some View {
        let produit = modele.boutique.produits[sku.productID]
        let possedes = modele.moi?.credits(for: sku) ?? 0

        Button {
            guard let produit else { return }
            Task { await acheter(produit) }
        } label: {
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text(sku.displayName).foregroundStyle(.primary)
                    if possedes > 0 {
                        Text("Vous en avez \(possedes)")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
                Spacer()
                prix(produit, enCours: modele.boutique.enCours == produit?.id)
            }
        }
        .disabled(produit == nil || modele.boutique.enCours != nil)
    }

    /// Le prix tel qu'Apple le rend, ou une attente.
    @ViewBuilder
    private func prix(_ produit: Product?, enCours: Bool) -> some View {
        if enCours {
            ProgressView()
        } else if let produit {
            Text(produit.displayPrice).monospacedDigit()
        } else {
            Text("—").foregroundStyle(.tertiary)
        }
    }

    private func acheter(_ produit: Product) async {
        do {
            if try await modele.boutique.acheter(produit) {
                // Le palier et les crédits vivent dans le résumé du compte :
                // sans cette relecture, l'écran annoncerait encore l'ancien.
                await modele.rafraichirMoi()
            }
        } catch {
            erreur = "L'achat n'a pas abouti. Si vous avez été débité, il sera retrouvé au prochain lancement."
        }
    }
}
