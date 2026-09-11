import ActivityKit
import SwiftUI
import WeaveKit
import WidgetKit

/// Live Activity « Métier ».
///
/// Règle de conception : ce qui apparaît ici est visible par quiconque regarde
/// l'écran verrouillé. On n'y met donc jamais de photo, jamais un message,
/// jamais un nom complet — un prénom, des compteurs, une échéance.
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
                    Compteur(valeur: contexte.state.activeThreads, libelle: "fils")
                }
                DynamicIslandExpandedRegion(.trailing) {
                    Compteur(valeur: contexte.state.awaitingYou, libelle: "à répondre")
                        .foregroundStyle(Color.weaveCuivreActivite)
                }
                DynamicIslandExpandedRegion(.center) {
                    Text(contexte.state.summary)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                DynamicIslandExpandedRegion(.bottom) {
                    if let intervalle = contexte.state.countdown {
                        HStack {
                            Text(contexte.state.soonestName ?? "Premier fil")
                                .font(.footnote.weight(.medium))
                            Spacer()
                            Text(timerInterval: intervalle, countsDown: true)
                                .font(.footnote.monospacedDigit())
                                .foregroundStyle(Color.weaveCuivreActivite)
                        }
                    } else if let regarnissage = contexte.state.nextRefillAt {
                        Text("Prochaine place garnie \(regarnissage, style: .relative)")
                            .font(.footnote)
                            .foregroundStyle(.secondary)
                    }
                }
            } compactLeading: {
                Image(systemName: "square.stack.3d.up")
                    .foregroundStyle(Color.weaveCuivreActivite)
            } compactTrailing: {
                if let intervalle = contexte.state.countdown {
                    Text(timerInterval: intervalle, countsDown: true)
                        .monospacedDigit()
                        .frame(maxWidth: 52)
                } else {
                    Text("\(contexte.state.activeThreads)")
                        .monospacedDigit()
                }
            } minimal: {
                Text("\(contexte.state.activeThreads)")
                    .monospacedDigit()
                    .foregroundStyle(Color.weaveCuivreActivite)
            }
            .keylineTint(Color.weaveCuivreActivite)
            .widgetURL(URL(string: "weave://metier"))
        }
    }
}

private struct EcranVerrouille: View {
    let etat: WeaveActivityAttributes.ContentState

    var body: some View {
        HStack(alignment: .center, spacing: 14) {
            Motif()

            VStack(alignment: .leading, spacing: 3) {
                Text(etat.activeThreads == 0 ? "Métier vide" : "\(etat.activeThreads) fils sur le métier")
                    .font(.headline)
                Text(etat.summary)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Spacer(minLength: 8)

            VStack(alignment: .trailing, spacing: 2) {
                if let intervalle = etat.countdown {
                    Text(timerInterval: intervalle, countsDown: true)
                        .font(.title3.monospacedDigit().weight(.semibold))
                        .foregroundStyle(Color.weaveCuivreActivite)
                    Text("avant dénouage")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                } else if let regarnissage = etat.nextRefillAt {
                    Text(regarnissage, style: .relative)
                        .font(.subheadline.monospacedDigit())
                        .foregroundStyle(.secondary)
                    Text("prochaine place")
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
    static let weaveCuivreActivite = Color(red: 0.902, green: 0.631, blue: 0.361)
    static let weaveEncreActivite = Color(red: 0.106, green: 0.090, blue: 0.078)
}

// MARK: - Aperçus

#Preview("Écran verrouillé", as: .content, using: WeaveActivityAttributes(accountHandle: "ines")) {
    WeaveLiveActivity()
} contentStates: {
    WeaveActivityAttributes.ContentState(
        activeThreads: 3,
        awaitingYou: 1,
        soonestExpiryAt: .now.addingTimeInterval(4 * 3600 + 12 * 60),
        soonestName: "Théo",
        nextRefillAt: nil
    )
    WeaveActivityAttributes.ContentState(
        activeThreads: 2,
        awaitingYou: 0,
        soonestExpiryAt: .now.addingTimeInterval(9 * 3600),
        soonestName: "Sofia",
        nextRefillAt: .now.addingTimeInterval(3600)
    )
    WeaveActivityAttributes.ContentState.idle
}
