import ActivityKit
import SwiftUI
import WeaveKit
import WidgetKit

/// Live Activity « Prochain plan ».
///
/// Règle de conception : ce qui apparaît ici est visible par quiconque regarde
/// l'écran verrouillé. On n'y met donc jamais de nom, jamais de photo, jamais
/// un message — un titre de plan, une heure, des compteurs.
///
/// Les décomptes utilisent `Text(timerInterval:)` : le système les anime
/// lui-même, sans réveiller l'application ni consommer d'envoi APNs.
struct WeaveLiveActivity: Widget {
    var body: some WidgetConfiguration {
        ActivityConfiguration(for: WeaveActivityAttributes.self) { contexte in
            EcranVerrouille(etat: contexte.state)
                .activityBackgroundTint(Color.weaveEncreActivite)
                .activitySystemActionForegroundColor(Color.weaveCuivreActivite)
        } dynamicIsland: { contexte in
            DynamicIsland {
                DynamicIslandExpandedRegion(.leading) {
                    Compteur(valeur: contexte.state.pendingRequests, libelle: "veulent venir")
                        .foregroundStyle(Color.weaveCuivreActivite)
                }
                DynamicIslandExpandedRegion(.trailing) {
                    Compteur(valeur: contexte.state.awaitingReply, libelle: "en attente")
                }
                DynamicIslandExpandedRegion(.center) {
                    Text(contexte.state.summary)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                DynamicIslandExpandedRegion(.bottom) {
                    if let intervalle = contexte.state.countdown {
                        HStack {
                            Text(contexte.state.planTitle ?? "Prochain plan")
                                .font(.footnote.weight(.medium))
                                .lineLimit(1)
                            Spacer()
                            Text(timerInterval: intervalle, countsDown: true)
                                .font(.footnote.monospacedDigit())
                                .foregroundStyle(Color.weaveCuivreActivite)
                        }
                    }
                }
            } compactLeading: {
                Image(systemName: "calendar")
                    .foregroundStyle(Color.weaveCuivreActivite)
            } compactTrailing: {
                if let intervalle = contexte.state.countdown {
                    Text(timerInterval: intervalle, countsDown: true)
                        .monospacedDigit()
                        .frame(maxWidth: 52)
                } else if contexte.state.pendingRequests > 0 {
                    Text("\(contexte.state.pendingRequests)")
                        .monospacedDigit()
                }
            } minimal: {
                Image(systemName: contexte.state.pendingRequests > 0 ? "envelope.fill" : "calendar")
                    .foregroundStyle(Color.weaveCuivreActivite)
            }
            .keylineTint(Color.weaveCuivreActivite)
            .widgetURL(URL(string: "weave://plans"))
        }
    }
}

private struct EcranVerrouille: View {
    let etat: WeaveActivityAttributes.ContentState

    var body: some View {
        HStack(alignment: .center, spacing: 14) {
            Motif()

            VStack(alignment: .leading, spacing: 3) {
                Text(etat.planTitle ?? "Aucun plan à venir")
                    .font(.headline)
                    .lineLimit(2)
                Text(etat.summary)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }

            Spacer(minLength: 8)

            VStack(alignment: .trailing, spacing: 2) {
                if let intervalle = etat.countdown {
                    Text(timerInterval: intervalle, countsDown: true)
                        .font(.title3.monospacedDigit().weight(.semibold))
                        .foregroundStyle(Color.weaveCuivreActivite)
                    Text("avant le départ")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                } else if etat.pendingRequests > 0 {
                    Text("\(etat.pendingRequests)")
                        .font(.title3.monospacedDigit().weight(.semibold))
                        .foregroundStyle(Color.weaveCuivreActivite)
                    Text("à lire")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }
            }
        }
        .padding(16)
    }
}

/// Trois fils verticaux : la marque, dessinée plutôt qu'importée.
private struct Motif: View {
    var body: some View {
        HStack(spacing: 3) {
            ForEach(0..<3, id: \.self) { _ in
                Capsule()
                    .fill(Color.weaveCuivreActivite)
                    .frame(width: 3, height: 26)
            }
        }
        .accessibilityHidden(true)
    }
}

private struct Compteur: View {
    let valeur: Int
    let libelle: String

    var body: some View {
        VStack(spacing: 1) {
            Text("\(valeur)")
                .font(.title3.monospacedDigit().weight(.semibold))
            Text(libelle)
                .font(.caption2)
                .foregroundStyle(.secondary)
        }
    }
}

extension Color {
    static let weaveCuivreActivite = Color(red: 0.690, green: 0.549, blue: 1.0)
    static let weaveEncreActivite = Color(red: 0.086, green: 0.071, blue: 0.122)
}

// MARK: - Aperçus

#Preview("Écran verrouillé", as: .content, using: WeaveActivityAttributes(accountHandle: "ines")) {
    WeaveLiveActivity()
} contentStates: {
    WeaveActivityAttributes.ContentState(
        planTitle: "Marché puis brunch, sans se presser",
        planStartsAt: .now.addingTimeInterval(36 * 3600),
        pendingRequests: 2,
        awaitingReply: 0
    )
    WeaveActivityAttributes.ContentState(
        planTitle: "Bloc au mur de 19 h",
        planStartsAt: .now.addingTimeInterval(3 * 3600),
        pendingRequests: 0,
        awaitingReply: 1
    )
    WeaveActivityAttributes.ContentState.idle
}
