import SwiftUI
import WeaveKit

/// Écran principal : les trois fils, et rien d'autre.
///
/// Il n'y a pas d'onglet « découvrir », pas de grille, pas de fil d'actualité.
/// Ce qui n'est pas sur cet écran n'existe pas dans le produit.
struct MetierView: View {
    @Environment(ModeleApplication.self) private var modele
    @State private var filPresente: ThreadCard?
    @State private var afficheReglages = false

    /// Cadence de rafraîchissement des décomptes : une fois par seconde suffit,
    /// et reste imperceptible pour la batterie.
    private let horloge = Timer.publish(every: 1, on: .main, in: .common).autoconnect()

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 16) {
                    EnTeteMetier(loom: modele.loom.loom)

                    ForEach(modele.loom.loom.threads) { fil in
                        FilCarte(fil: fil)
                            .onTapGesture { filPresente = fil }
                            .accessibilityAddTraits(.isButton)
                            .accessibilityHint("Ouvrir le fil de \(fil.displayName)")
                    }

                    // Une carte par place libre ne tient plus à douze : on en
                    // affiche une seule, qui porte le compte.
                    if modele.loom.loom.freeSlots > 0 {
                        PlacesLibres(
                            nombre: modele.loom.loom.freeSlots,
                            prochainRegarnissage: modele.loom.loom.nextRefillAt
                        )
                    }

                    if modele.loom.isEmpty, modele.loom.phase == .ready {
                        MetierVide(prochainTissage: modele.loom.loom.nextWeavingAt)
                    }
                }
                .padding(.horizontal, 16)
                .padding(.bottom, 32)
            }
            .background(Color.weaveLin.ignoresSafeArea())
            .navigationTitle("Métier")
            .navigationBarTitleDisplayMode(.large)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button {
                        afficheReglages = true
                    } label: {
                        Label("Réglages", systemImage: "person.crop.circle")
                    }
                }
            }
            .refreshable { await modele.loom.refresh() }
            .onReceive(horloge) { _ in modele.loom.dropExpired() }
            .sheet(item: $filPresente) { fil in
                FilDetailView(fil: fil)
            }
            .sheet(isPresented: $afficheReglages) {
                ReglagesView()
            }
            .alert(
                "Weave",
                isPresented: Binding(
                    get: { modele.loom.alert != nil },
                    set: { if !$0 { modele.loom.alert = nil } }
                ),
                presenting: modele.loom.alert
            ) { _ in
                Button("Entendu", role: .cancel) {}
            } message: { erreur in
                Text(erreur.userMessage)
            }
        }
    }
}

private struct EnTeteMetier: View {
    let loom: Loom

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(loom.threads.isEmpty ? "Aucun fil" : "\(loom.threads.count) sur \(Loom.maxActiveThreads)")
                .font(.system(.title3, design: .serif, weight: .semibold))

            if let prochain = loom.nextRefillAt, loom.freeSlots > 0 {
                Text("Prochaine place garnie \(prochain, style: .relative)")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            } else {
                Text("Prochain tissage \(loom.nextWeavingAt, style: .relative)")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.top, 4)
    }
}

/// Carte d'un fil : le motif d'abord, la photo floutée derrière, jamais l'inverse.
struct FilCarte: View {
    let fil: ThreadCard

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .firstTextBaseline) {
                Text(fil.displayName)
                    .font(.system(.title2, design: .serif, weight: .semibold))
                Text("\(fil.age)")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
                Spacer()
                Decompte(expiration: fil.expiresAt)
            }

            Text(fil.motif.joined(separator: " · "))
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .lineLimit(2)

            PhotoVoilee(fil: fil)
                .frame(height: 180)
                .clipShape(RoundedRectangle(cornerRadius: 14))

            if let premier = fil.fragments.first(where: { $0.kind == .question }) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(premier.prompt)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Text(premier.body)
                        .font(.callout)
                        .lineLimit(3)
                }
            }

            HStack(spacing: 8) {
                Etat(fil: fil)
                Spacer()
                Text("\(fil.distanceKm) km · \(fil.city)")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(16)
        .background(.background, in: RoundedRectangle(cornerRadius: 20))
        .overlay(
            RoundedRectangle(cornerRadius: 20)
                .stroke(fil.awaitingYou ? Color.weaveCuivre.opacity(0.5) : .clear, lineWidth: 1.5)
        )
        .accessibilityElement(children: .combine)
    }
}

private struct PhotoVoilee: View {
    let fil: ThreadCard

    var body: some View {
        ZStack {
            if let url = fil.photoURL {
                AsyncImage(url: url) { phase in
                    switch phase {
                    case .success(let image):
                        image.resizable().scaledToFill()
                    default:
                        Rectangle().fill(.quaternary)
                    }
                }
                // Le serveur a déjà flouté l'image ; ce flou n'est qu'un raccord
                // visuel avec le niveau de révélation en cours.
                .blur(radius: fil.blurRadius)
                .clipped()
            } else {
                Rectangle().fill(.quaternary)
            }

            if fil.revealPercent < 100 {
                Text("\(fil.revealPercent) %")
                    .font(.caption2.weight(.semibold))
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(.ultraThinMaterial, in: Capsule())
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomTrailing)
                    .padding(8)
            }
        }
        .accessibilityLabel(
            fil.revealPercent == 100
                ? "Photo de \(fil.displayName)"
                : "Photo de \(fil.displayName), voilée à \(100 - fil.revealPercent) pour cent"
        )
    }
}

private struct Etat: View {
    let fil: ThreadCard

    var body: some View {
        let texte: String = switch fil.state {
        case .propose: "À vous de répondre"
        case .engage: fil.awaitingYou ? "À vous de répondre" : "Réponse envoyée"
        case .tisse: "Fil tissé"
        case .denoue: "Dénoué"
        }

        Text(texte)
            .font(.caption.weight(.medium))
            .foregroundStyle(fil.awaitingYou ? Color.weaveCuivre : .secondary)
    }
}

private struct Decompte: View {
    let expiration: Date

    var body: some View {
        Text(timerInterval: Date.now...max(expiration, .now), countsDown: true)
            .font(.subheadline.monospacedDigit())
            .foregroundStyle(expiration.timeIntervalSinceNow < 3600 ? Color.weaveCuivre : .secondary)
            .accessibilityLabel("Se dénoue \(expiration.formatted(.relative(presentation: .named)))")
    }
}

private struct PlacesLibres: View {
    let nombre: Int
    let prochainRegarnissage: Date?

    var body: some View {
        VStack(spacing: 6) {
            Image(systemName: "circle.dashed")
                .font(.title2)
                .foregroundStyle(.tertiary)
            Text(nombre == 1 ? "1 place libre" : "\(nombre) places libres")
                .font(.subheadline.weight(.medium))
                .foregroundStyle(.secondary)
            if let prochainRegarnissage {
                Text("prochaine garnie \(prochainRegarnissage, style: .relative)")
                    .font(.caption)
                    .foregroundStyle(.tertiary)
            }
        }
        .frame(maxWidth: .infinity)
        .frame(height: 120)
        .background(
            RoundedRectangle(cornerRadius: 20)
                .strokeBorder(style: StrokeStyle(lineWidth: 1.5, dash: [6, 5]))
                .foregroundStyle(.quaternary)
        )
        .accessibilityElement(children: .combine)
    }
}

private struct MetierVide: View {
    let prochainTissage: Date

    var body: some View {
        VStack(spacing: 10) {
            Text("Le métier est vide")
                .font(.system(.title3, design: .serif, weight: .semibold))
            Text("Vos prochains fils arrivent \(prochainTissage, style: .relative).")
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
        }
        .padding(.vertical, 40)
    }
}
