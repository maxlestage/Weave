import SwiftUI
import WeaveKit

/// L'écran d'accueil de la montre : le prochain plan, et ce qui attend.
///
/// Rien de nominatif ici : un titre, une heure, des compteurs. Les prénoms et
/// les messages n'apparaissent qu'un geste plus loin.
struct MontreRacineView: View {
    @Environment(ModeleMontre.self) private var modele

    private var magasin: MontreStore { modele.magasin }

    var body: some View {
        NavigationStack {
            List {
                if !magasin.aUneSession {
                    ConnexionMontreView()
                } else {
                    // Les plans autour de soi : toujours accessibles, même
                    // quand rien ne nous concerne encore.
                    Section {
                        NavigationLink {
                            FilMontreView()
                        } label: {
                            Label("Autour de vous", systemImage: "mappin.and.ellipse")
                        }
                    }

                    if magasin.resume.nextPlan == nil,
                       magasin.resume.pendingRequests == 0,
                       magasin.resume.awaitingReply == 0
                    {
                        VidePlaceholder(chargement: magasin.chargement, erreur: magasin.erreur)
                    } else {
                        if let plan = magasin.resume.nextPlan {
                            Section("Prochain plan") {
                                ProchainPlan(plan: plan)
                            }
                        }

                        Section("En cours") {
                            if magasin.resume.pendingRequests > 0 {
                                NavigationLink {
                                    DemandesMontreView()
                                } label: {
                                    Compteur(
                                        valeur: magasin.resume.pendingRequests,
                                        libelle: "veulent venir",
                                        accentue: true
                                    )
                                }
                            }
                            NavigationLink {
                                ConversationsMontreView()
                            } label: {
                                Label("Conversations", systemImage: "bubble.left.and.bubble.right")
                            }
                            if magasin.resume.awaitingReply > 0 {
                                Compteur(
                                    valeur: magasin.resume.awaitingReply,
                                    libelle: "demandes sans réponse",
                                    accentue: false
                                )
                            }
                        }
                    }
                }
            }
            .navigationTitle("Weave")
            .refreshable { await modele.rafraichir() }
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
    let libelle: LocalizedStringKey
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
