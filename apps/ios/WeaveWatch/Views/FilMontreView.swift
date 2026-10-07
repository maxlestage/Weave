import SwiftUI
import WeaveKit

/// Les plans autour de soi, au poignet : le même fil que sur l'iPhone.
///
/// La liste ne montre que le plan — un titre, une heure, une distance. Le
/// prénom de qui le propose n'apparaît qu'un geste plus loin, et se masque dès
/// que le poignet retombe : un poignet est plus exposé au regard qu'un
/// téléphone. Pas de photo : elle n'apporte rien à cette taille, et c'est ce
/// qui se reconnaît de plus loin.
struct FilMontreView: View {
    @Environment(ModeleMontre.self) private var modele

    private var magasin: MontreStore { modele.magasin }

    var body: some View {
        List {
            if magasin.fil.isEmpty {
                Text("Aucun plan")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else {
                Text("\(magasin.demandesRestantes) demandes restantes")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
            ForEach(magasin.fil) { plan in
                NavigationLink {
                    PlanMontreView(plan: plan)
                } label: {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(plan.title)
                            .font(.headline)
                            .lineLimit(2)
                        Text(plan.startsAt.weaveWhenLabel)
                            .font(.caption2)
                            .foregroundStyle(Color.weaveCuivreMontre)
                        Text("à \(plan.distanceKm) km")
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                    }
                }
            }
            if let erreur = magasin.erreur {
                Text(erreur)
                    .font(.caption2)
                    .foregroundStyle(.red)
            }
        }
        .navigationTitle("Autour de vous")
        .task { await magasin.chargerFil() }
        .refreshable { await magasin.chargerFil() }
    }
}

/// Un plan, et la demande pour le rejoindre — dictée, le plus souvent.
private struct PlanMontreView: View {
    let plan: Plan

    @Environment(ModeleMontre.self) private var modele
    @State private var message = ""
    @State private var enCours = false
    @State private var refus: String?

    /// L'état le plus frais : le plan a pu être demandé depuis l'ouverture.
    private var actuel: Plan {
        modele.magasin.fil.first { $0.id == plan.id } ?? plan
    }

    private var manque: Int {
        JoinRequest.minimumMessageLength
            - message.trimmingCharacters(in: .whitespacesAndNewlines).count
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 8) {
                Text(actuel.title)
                    .font(.headline)
                Text(actuel.startsAt.weaveWhenLabel)
                    .font(.caption2)
                    .foregroundStyle(Color.weaveCuivreMontre)
                Text(actuel.city)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                Text(actuel.author.displayName)
                    .font(.body)
                    .privacySensitive()
                if !actuel.note.isEmpty {
                    Text(actuel.note)
                        .font(.caption)
                        .privacySensitive()
                }

                if actuel.requested {
                    Label("Demande envoyée", systemImage: "checkmark")
                        .font(.caption)
                        .foregroundStyle(Color.weaveCuivreMontre)
                } else if actuel.seatsLeft == 0 {
                    Text("Complet")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                } else if actuel.isJoinable {
                    // Le champ ouvre la dictée, le griffonnage ou le clavier.
                    TextField("Demander à venir", text: $message, axis: .vertical)
                    if manque > 0 {
                        Text("Encore \(manque) caractères")
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                    }
                    Button {
                        Task { await envoyer() }
                    } label: {
                        Label("Envoyer", systemImage: "paperplane")
                            .frame(maxWidth: .infinity)
                    }
                    .tint(.weaveCuivreMontre)
                    .disabled(manque > 0 || enCours || modele.magasin.demandesRestantes == 0)
                }

                if let refus {
                    Text(refus)
                        .font(.caption2)
                        .foregroundStyle(.red)
                }
            }
        }
        .navigationTitle(actuel.category.displayName)
    }

    private func envoyer() async {
        enCours = true
        defer { enCours = false }
        refus = await modele.magasin.demanderAVenir(actuel, message: message)
        if refus == nil { message = "" }
    }
}
