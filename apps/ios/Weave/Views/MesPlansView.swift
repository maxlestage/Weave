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
    @State private var modification = false

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
            // Corriger passe AVANT annuler, et sans rôle destructeur.
            //
            // Rien ne permettait de corriger un plan : une coquille dans le
            // titre, une heure décalée, et il fallait annuler puis republier.
            // Ce contournement consomme l'un des trois plans ouverts, perd les
            // personnes acceptées, et leur annonce « un plan est annulé » pour
            // une faute de frappe.
            if plan.state == .ouvert || plan.state == .complet {
                Button("Modifier ce plan") { modification = true }
            }
            // Annuler un plan n'est pas anodin : les demandes en attente sont
            // closes. On ne le met donc pas à portée d'un balayage.
            Button("Annuler ce plan", role: .destructive) {
                Task { await modele.plans.cancel(plan) }
            }
        }
        .sheet(isPresented: $modification) {
            ModifierPlanView(plan: plan)
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
                        } apresProtection: {
                            // Bloquer coupe tout des deux côtés, la demande
                            // comprise : la retirer de l'écran évite de laisser
                            // à l'affichage une ligne qui n'existe plus.
                            demandes.removeAll { $0.id == demande.id }
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
    let apresProtection: () -> Void

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
                Spacer()
                // C'est ICI qu'arrive le premier message d'un inconnu, et
                // c'était le seul endroit du produit où l'on en lisait un sans
                // pouvoir rien en faire.
                //
                // Il n'y avait que « Accepter » et « Sans suite ». Devant un
                // message déplacé, cela revenait à choisir entre ouvrir une
                // conversation avec son auteur, ou l'écarter en silence — et
                // le laisser recommencer sur le plan suivant. Le signalement
                // existait partout ailleurs : dans la conversation, et avant
                // même de demander à venir.
                MenuDeProtection(
                    compteID: demande.author.id,
                    prenom: demande.author.displayName,
                    apresCoupure: apresProtection
                )
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

/// Corriger un plan publié.
///
/// Ce qu'on peut changer, et rien de plus : le titre, la note, l'heure, le
/// nombre de places. Ni la catégorie ni la ville — les changer ne corrige pas
/// un plan, cela en fait un autre, auquel des gens ont dit oui sans le
/// connaître.
private struct ModifierPlanView: View {
    let plan: MyPlan

    @Environment(ModeleApplication.self) private var modele
    @Environment(\.dismiss) private var dismiss

    @State private var titre = ""
    @State private var note = ""
    @State private var debut = Date.now
    @State private var places = 1
    @State private var envoi = false

    /// Ce qui est réellement différent.
    ///
    /// Le champ absent vaut « ne change pas » côté serveur : envoyer un titre
    /// identique n'est pas faux, mais un changement d'HEURE identique
    /// préviendrait les personnes acceptées pour rien. On ne poste que l'écart.
    private var ecart: PlanEdit {
        PlanEdit(
            title: titre != plan.title ? titre : nil,
            note: note != plan.note ? note : nil,
            startsAt: debut != plan.startsAt ? debut : nil,
            capacity: places != plan.capacity ? places : nil
        )
    }

    var body: some View {
        NavigationStack {
            Form {
                Section("Le plan") {
                    TextField("Ce que vous comptez faire", text: $titre)
                    TextField("Une précision, si besoin", text: $note, axis: .vertical)
                        .lineLimit(1...4)
                }

                Section {
                    DatePicker("Quand", selection: $debut, in: Date.now...)
                } footer: {
                    if debut != plan.startsAt {
                        // Dit avant, pas après : quelqu'un qui corrige une
                        // coquille ne s'attend pas à faire sonner un téléphone.
                        Text("Les personnes que vous attendez seront prévenues du changement d'heure.")
                    }
                }

                Section {
                    Stepper("\(places) place\(places > 1 ? "s" : "")", value: $places, in: 1...6)
                } footer: {
                    if places < plan.seatsAccordees {
                        Text("Vous avez déjà accordé \(plan.seatsAccordees) place\(plan.seatsAccordees > 1 ? "s" : "") : les reprendre reviendrait à décommander quelqu'un.")
                            .foregroundStyle(.red)
                    }
                }
            }
            .navigationTitle("Modifier")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Annuler") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Enregistrer") {
                        envoi = true
                        Task {
                            if await modele.plans.edit(plan, ecart) { dismiss() }
                            envoi = false
                        }
                    }
                    .disabled(envoi || ecart.vide || places < plan.seatsAccordees)
                }
            }
            .task {
                titre = plan.title
                note = plan.note
                debut = plan.startsAt
                places = plan.capacity
            }
        }
    }
}
