import SwiftUI
import WeaveKit

/// Ce que la montre montre : le prochain plan, et ce qui attend une réponse.
///
/// Elle ne reçoit ni nom, ni photo, ni message — un titre, une heure, deux
/// compteurs. Le résumé complet fait quelques centaines d'octets.
struct MontreRacineView: View {
    @Environment(ModeleMontre.self) private var modele

    var body: some View {
        NavigationStack {
            List {
                if modele.resume.nextPlan == nil, modele.resume.pendingRequests == 0 {
                    VidePlaceholder(chargement: modele.chargement, erreur: modele.erreur)
                } else {
                    if let plan = modele.resume.nextPlan {
                        Section("Prochain plan") {
                            ProchainPlan(plan: plan)
                        }
                    }

                    if modele.resume.pendingRequests > 0 || modele.resume.awaitingReply > 0 {
                        Section("En cours") {
                            if modele.resume.pendingRequests > 0 {
                                Compteur(
                                    valeur: modele.resume.pendingRequests,
                                    libelle: "veulent venir",
                                    accentue: true
                                )
                            }
                            if modele.resume.awaitingReply > 0 {
                                Compteur(
                                    valeur: modele.resume.awaitingReply,
                                    libelle: "demandes sans réponse",
                                    accentue: false
                                )
                            }
                        }
                    }
                }
            }
            .navigationTitle("Weave")
            .refreshable { await modele.charger() }
        }
    }
}

private struct ProchainPlan: View {
    let plan: WatchSummary.NextPlan

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(plan.title)
                .font(.headline)
                .lineLimit(2)
            Text(plan.city)
                .font(.caption2)
                .foregroundStyle(.secondary)
            Text(timerInterval: Date.now...max(plan.startsAt, .now), countsDown: true)
                .font(.caption2.monospacedDigit())
                .foregroundStyle(Color.weaveCuivreMontre)
        }
        .padding(.vertical, 2)
        .accessibilityElement(children: .combine)
    }
}

private struct Compteur: View {
    let valeur: Int
    let libelle: String
    let accentue: Bool

    var body: some View {
        HStack(spacing: 8) {
            Text("\(valeur)")
                .font(.title3.monospacedDigit().weight(.semibold))
                .foregroundStyle(accentue ? Color.weaveCuivreMontre : .primary)
            Text(libelle)
                .font(.caption2)
                .foregroundStyle(.secondary)
        }
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
                Text("Aucun plan à venir")
                    .font(.headline)
                Text("Publiez quelque chose, ou demandez à venir à un plan.")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
            }
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 12)
    }
}
