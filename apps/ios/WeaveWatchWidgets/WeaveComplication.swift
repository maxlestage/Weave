import SwiftUI
import WeaveKit
import WidgetKit

/// Complication de cadran : le prochain plan, et ce qui attend une réponse.
/// Rien d'autre ne tient — ni ne devrait tenir — sur un cadran de montre.
struct WeaveComplication: Widget {
    var body: some WidgetConfiguration {
        StaticConfiguration(kind: "app.weave.complication", provider: Fournisseur()) { entree in
            ComplicationVue(resume: entree.resume)
                .containerBackground(.fill.tertiary, for: .widget)
        }
        .configurationDisplayName("Prochain plan")
        .description("Votre prochain rendez-vous, et les demandes à traiter.")
        .supportedFamilies([
            .accessoryCircular,
            .accessoryCorner,
            .accessoryInline,
            .accessoryRectangular,
        ])
    }
}

struct Entree: TimelineEntry {
    let date: Date
    let resume: WatchSummary
}

/// Le résumé est déposé par l'application dans les préférences partagées du
/// groupe. La complication ne fait pas d'appel réseau : elle serait réveillée
/// bien trop souvent, pour une donnée qui bouge quelques fois par jour.
struct Fournisseur: TimelineProvider {
    func placeholder(in context: Context) -> Entree {
        Entree(date: .now, resume: .empty)
    }

    func getSnapshot(in context: Context, completion: @escaping (Entree) -> Void) {
        completion(Entree(date: .now, resume: Self.lire()))
    }

    func getTimeline(in context: Context, completion: @escaping (Timeline<Entree>) -> Void) {
        let resume = Self.lire()
        // Prochaine relève : à l'heure du prochain plan, ou dans une heure.
        let prochaine = resume.nextPlan?.startsAt ?? Date.now.addingTimeInterval(3600)
        completion(
            Timeline(
                entries: [Entree(date: .now, resume: resume)],
                policy: .after(min(prochaine, Date.now.addingTimeInterval(3600)))
            )
        )
    }

    private static func lire() -> WatchSummary {
        guard let defaults = UserDefaults(suiteName: WeaveEnvironment.appGroup),
              let data = defaults.data(forKey: "watchSummary")
        else { return .empty }

        let decodeur = JSONDecoder()
        decodeur.dateDecodingStrategy = .iso8601
        return (try? decodeur.decode(WatchSummary.self, from: data)) ?? .empty
    }
}

private struct ComplicationVue: View {
    @Environment(\.widgetFamily) private var famille
    let resume: WatchSummary

    /// Une ligne, la même partout : ce qui attend une action d'abord.
    private var ligne: String {
        if resume.pendingRequests > 0 {
            return resume.pendingRequests > 1
                ? "\(resume.pendingRequests) veulent venir"
                : "Quelqu'un veut venir"
        }
        if let plan = resume.nextPlan { return plan.city }
        if resume.awaitingReply > 0 { return "\(resume.awaitingReply) en attente" }
        return "Rien de prévu"
    }

    var body: some View {
        switch famille {
        case .accessoryInline:
            Text(ligne)

        case .accessoryCircular:
            VStack(spacing: 0) {
                Image(systemName: resume.pendingRequests > 0 ? "envelope.fill" : "calendar")
                    .font(.caption)
                if resume.pendingRequests > 0 {
                    Text("\(resume.pendingRequests)").font(.caption2.monospacedDigit())
                }
            }

        case .accessoryRectangular:
            VStack(alignment: .leading, spacing: 2) {
                Text(resume.nextPlan?.title ?? "Aucun plan")
                    .font(.headline)
                    .lineLimit(1)
                Text(ligne).font(.caption)
                if let depart = resume.nextPlan?.startsAt, depart > .now {
                    Text(timerInterval: Date.now...depart, countsDown: true)
                        .font(.caption2.monospacedDigit())
                        .foregroundStyle(.secondary)
                }
            }

        default:
            Image(systemName: resume.pendingRequests > 0 ? "envelope.fill" : "calendar")
        }
    }
}

@main
struct WeaveWatchWidgets: WidgetBundle {
    var body: some Widget {
        WeaveComplication()
    }
}
