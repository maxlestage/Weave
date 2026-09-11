import SwiftUI
import WeaveKit

/// Écran principal : les plans à venir autour de soi.
///
/// Il n'y a pas de pile à balayer, pas de grille de visages, pas de fil
/// d'actualité. On lit ce que les autres comptent faire, on demande à venir, et
/// c'est tout. Ce qui n'est pas sur cet écran n'existe pas dans le produit.
struct FilView: View {
    @Environment(ModeleApplication.self) private var modele
    @State private var planPresente: Plan?

    /// Les plans dont l'heure est passée sortent tout seuls : une minute suffit,
    /// un rendez-vous ne se périme pas à la seconde.
    private let horloge = Timer.publish(every: 60, on: .main, in: .common).autoconnect()

    var body: some View {
        NavigationStack {
            ScrollView {
                LazyVStack(spacing: 14) {
                    CompteurDemandes(restantes: modele.plans.requestsLeftToday)

                    ForEach(modele.plans.feed.plans) { plan in
                        PlanCarte(plan: plan)
                            .onTapGesture { planPresente = plan }
                            .accessibilityAddTraits(.isButton)
                            .accessibilityHint("Ouvrir le plan de \(plan.author.displayName)")
                    }

                    if modele.plans.isEmpty, modele.plans.phase == .ready {
                        FilVide()
                    }
                }
                .padding(.horizontal, 16)
                .padding(.bottom, 32)
            }
            .background(Color.weaveLin.ignoresSafeArea())
            .navigationTitle("Autour de vous")
            .refreshable { await modele.plans.refresh() }
            .onReceive(horloge) { _ in modele.plans.dropPast() }
            .sheet(item: $planPresente) { plan in
                DemanderView(plan: plan)
                    .environment(modele)
            }
        }
    }
}

/// La seule ressource rare du produit, affichée en permanence : on ne peut pas
/// arroser, et le cacher rendrait la limite arbitraire au lieu d'être une règle.
private struct CompteurDemandes: View {
    let restantes: Int

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: restantes > 0 ? "envelope" : "envelope.badge.shield.half.filled")
                .foregroundStyle(Color.weaveCuivre)
            Text(texte)
                .font(.subheadline)
                .foregroundStyle(.secondary)
            Spacer()
        }
        .padding(.vertical, 10)
        .padding(.horizontal, 14)
        .background(Color.white, in: .rect(cornerRadius: 14))
        .accessibilityElement(children: .combine)
    }

    private var texte: String {
        switch restantes {
        case 0: "Plus de demandes aujourd'hui. Elles reviennent à minuit."
        case 1: "Une demande restante aujourd'hui."
        default: "\(restantes) demandes restantes aujourd'hui."
        }
    }
}

struct PlanCarte: View {
    let plan: Plan

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 8) {
                Label(plan.category.displayName, systemImage: plan.category.symbolName)
                    .font(.caption.weight(.bold))
                    .foregroundStyle(Color.weaveCuivre)
                Spacer()
                Text("à \(plan.distanceKm) km")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Text(plan.title)
                .font(.headline)
                .multilineTextAlignment(.leading)

            if !plan.note.isEmpty {
                Text(plan.note)
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
                    .lineLimit(3)
            }

            HStack(spacing: 8) {
                Text(plan.startsAt.weaveWhenLabel)
                    .font(.subheadline.weight(.semibold))
                Spacer()
                Text(placesTexte)
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(plan.seatsLeft > 0 ? .secondary : Color.weaveCuivre)
            }

            HStack(spacing: 6) {
                Text("\(plan.author.displayName), \(plan.author.age) ans")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                if plan.author.verified {
                    Image(systemName: "checkmark.seal.fill")
                        .font(.caption2)
                        .foregroundStyle(Color.weaveCuivre)
                        .accessibilityLabel("Profil vérifié")
                }
                Spacer()
                if plan.requested {
                    Text("Demande envoyée")
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(Color.weaveCuivre)
                }
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: .rect(cornerRadius: 18))
    }

    private var placesTexte: String {
        switch plan.seatsLeft {
        case 0: "Complet"
        case 1: "1 place"
        default: "\(plan.seatsLeft) places"
        }
    }
}

private struct FilVide: View {
    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: "calendar.badge.plus")
                .font(.system(size: 40))
                .foregroundStyle(Color.weaveCuivre)
            Text("Rien autour de vous pour l'instant")
                .font(.headline)
            Text("C'est le moment de publier quelque chose. Un plan sans réponse reste un plan : vous ferez ce que vous aviez prévu.")
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
        }
        .padding(.vertical, 48)
        .padding(.horizontal, 24)
    }
}
