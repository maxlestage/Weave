import SwiftUI
import WeaveKit

/// Ses propres plans, et les demandes qu'ils ont suscitées.
struct MesPlansView: View {
    @Environment(ModeleApplication.self) private var modele
    @State private var publie = false
    @State private var planOuvert: MyPlan?

    var body: some View {
        NavigationStack {
            ScrollView {
                LazyVStack(spacing: 14) {
                    ForEach(modele.plans.myPlans) { plan in
                        MonPlanCarte(plan: plan)
                            .onTapGesture { planOuvert = plan }
                            .accessibilityAddTraits(.isButton)
                    }

                    if modele.plans.myPlans.isEmpty {
                        VStack(spacing: 12) {
                            Image(systemName: "square.and.pencil")
                                .font(.system(size: 36))
                                .foregroundStyle(Color.weaveCuivre)
                            Text("Aucun plan publié")
                                .font(.headline)
                            Text("Au plus \(MyPlan.maxOpen) plans ouverts à la fois : ce que vous comptez vraiment faire, pas une annonce permanente.")
                                .font(.subheadline)
                                .foregroundStyle(.secondary)
                                .multilineTextAlignment(.center)
                        }
                        .padding(.vertical, 48)
                        .padding(.horizontal, 24)
                    }
                }
                .padding(.horizontal, 16)
                .padding(.bottom, 32)
            }
            .background(Color.weaveLin.ignoresSafeArea())
            .navigationTitle("Mes plans")
            .refreshable { await modele.plans.refresh() }
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button {
                        publie = true
                    } label: {
                        Label("Publier", systemImage: "plus")
                    }
                    .disabled(modele.plans.myPlans.count >= MyPlan.maxOpen)
                }
            }
            .sheet(isPresented: $publie) {
                PublierView().environment(modele)
            }
            .sheet(item: $planOuvert) { plan in
                DemandesRecuesView(plan: plan).environment(modele)
            }
        }
    }
}

private struct MonPlanCarte: View {
    @Environment(ModeleApplication.self) private var modele
    let plan: MyPlan

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Label(plan.category.displayName, systemImage: plan.category.symbolName)
                    .font(.caption.weight(.bold))
                    .foregroundStyle(Color.weaveCuivre)
                Spacer()
                if plan.pendingRequests > 0 {
                    Text("\(plan.pendingRequests)")
                        .font(.caption.weight(.bold))
                        .padding(.horizontal, 8)
                        .padding(.vertical, 3)
                        .background(Color.weaveCuivre, in: .capsule)
                        .foregroundStyle(.white)
                        .accessibilityLabel("\(plan.pendingRequests) demandes à traiter")
                }
            }

            Text(plan.title).font(.headline)

            Text(plan.startsAt.weaveWhenLabel)
                .font(.subheadline.weight(.semibold))

            Text(etat)
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: .rect(cornerRadius: 18))
        .contextMenu {
            // Annuler un plan n'est pas anodin : les demandes en attente sont
            // closes. On ne le met donc pas à portée d'un balayage.
            Button("Annuler ce plan", role: .destructive) {
                Task { await modele.plans.cancel(plan) }
            }
        }
    }

    private var etat: String {
        switch plan.state {
        case .ouvert: plan.seatsLeft == 1 ? "1 place libre" : "\(plan.seatsLeft) places libres"
        case .complet: "Complet"
        case .passe: "Passé"
        case .annule: "Annulé"
        // Ajouté à `PlanState` pour que « Mes plans » cesse d'échouer au
        // décodage ; l'écran, lui, n'avait pas suivi. Seul un vrai
        // compilateur pouvait le dire — un `switch` incomplet ne se voit
        // qu'à la compilation de la cible, hors de portée de Linux.
        case .suspendu: "En pause"
        }
    }
}

/// Les demandes reçues sur un plan. Seul son auteur les voit : elles ne sont
/// pas publiques, et l'on n'affiche jamais « qui a regardé ».
struct DemandesRecuesView: View {
    let plan: MyPlan

    @Environment(ModeleApplication.self) private var modele
    @Environment(\.dismiss) private var dismiss

    @State private var demandes: [IncomingRequest] = []
    @State private var chargement = true

    var body: some View {
        NavigationStack {
            Group {
                if chargement {
                    ProgressView().controlSize(.large)
                } else if demandes.isEmpty {
                    ContentUnavailableView(
                        "Aucune demande",
                        systemImage: "envelope",
                        description: Text("Personne n'a encore demandé à venir. Ça n'enlève rien au plan.")
                    )
                } else {
                    List(demandes) { demande in
                        DemandeLigne(demande: demande) { acceptee in
                            Task { await repondre(demande, accepte: acceptee) }
                        }
                    }
                    .listStyle(.plain)
                }
            }
            .navigationTitle(plan.title)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Fermer") { dismiss() }
                }
            }
            .task {
                demandes = await modele.plans.incoming(for: plan)
                chargement = false
            }
        }
    }

    private func repondre(_ demande: IncomingRequest, accepte: Bool) async {
        if accepte {
            _ = await modele.plans.accept(demande)
        } else {
            await modele.plans.decline(demande)
        }
        demandes.removeAll { $0.id == demande.id }
    }
}

private struct DemandeLigne: View {
    let demande: IncomingRequest
    let repondre: (Bool) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 6) {
                Text("\(demande.author.displayName), \(demande.author.age) ans")
                    .font(.subheadline.weight(.semibold))
                if demande.author.verified {
                    Image(systemName: "checkmark.seal.fill")
                        .font(.caption2)
                        .foregroundStyle(Color.weaveCuivre)
                        .accessibilityLabel("Profil vérifié")
                }
            }

            Text(demande.message)
                .font(.body)

            // Deux boutons dans une même ligne de liste : sans un style
            // « borderless », toute la ligne devient un seul bouton et les deux
            // actions se déclenchent ensemble.
            HStack(spacing: 12) {
                Button("Accepter") { repondre(true) }
                    .buttonStyle(.borderedProminent)
                Button("Sans suite") { repondre(false) }
                    .buttonStyle(.bordered)
            }
            .controlSize(.small)
            .buttonStyle(.borderless)
        }
        .padding(.vertical, 8)
    }
}
