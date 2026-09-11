import SwiftUI
import WeaveKit
import WidgetKit

/// Complication de cadran : le nombre de fils en attente, et le temps qu'il
/// reste au plus pressé. Rien d'autre ne tient — ni ne devrait tenir — sur un
/// cadran de montre.
struct WeaveComplication: Widget {
    var body: some WidgetConfiguration {
        StaticConfiguration(kind: "app.weave.complication", provider: Fournisseur()) { entree in
            ComplicationVue(resume: entree.resume)
                .containerBackground(.fill.tertiary, for: .widget)
        }
        .configurationDisplayName("Métier")
        .description("Vos fils en cours et le temps qu'il leur reste.")
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
/// bien trop souvent, pour une donnée qui change à l'heure de tissage.
struct Fournisseur: TimelineProvider {
    func placeholder(in context: Context) -> Entree {
        Entree(date: .now, resume: .empty)
    }

    func getSnapshot(in context: Context, completion: @escaping (Entree) -> Void) {
        completion(Entree(date: .now, resume: Self.lire()))
    }

    func getTimeline(in context: Context, completion: @escaping (Timeline<Entree>) -> Void) {
        let resume = Self.lire()
        // Prochaine relève : à la première échéance connue, ou dans une heure.
        let prochaine = resume.soonestExpiryAt ?? Date.now.addingTimeInterval(3600)
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

    var body: some View {
        switch famille {
        case .accessoryInline:
            Text(resume.awaitingYou > 0 ? "\(resume.awaitingYou) à répondre" : "\(resume.activeThreads) fils")

        case .accessoryCircular:
            Gauge(value: Double(resume.activeThreads), in: 0...Double(Loom.maxActiveThreads)) {
                Image(systemName: "square.stack.3d.up")
            } currentValueLabel: {
                Text("\(resume.activeThreads)")
            }
            .gaugeStyle(.accessoryCircular)

        case .accessoryRectangular:
            VStack(alignment: .leading, spacing: 2) {
                Text("Métier").font(.headline)
                Text(resume.awaitingYou > 0
                     ? "\(resume.awaitingYou) fil\(resume.awaitingYou > 1 ? "s" : "") en attente"
                     : "\(resume.activeThreads) fil\(resume.activeThreads > 1 ? "s" : "") en cours")
                    .font(.caption)
                if let echeance = resume.soonestExpiryAt, echeance > .now {
                    Text(timerInterval: Date.now...echeance, countsDown: true)
                        .font(.caption2.monospacedDigit())
                        .foregroundStyle(.secondary)
                }
            }

        default:
            Text("\(resume.activeThreads)")
        }
    }
}

@main
struct WeaveWatchWidgets: WidgetBundle {
    var body: some Widget {
        WeaveComplication()
    }
}
