import SwiftUI
import WeaveKit

/// Détail d'un fil : la trame entière, et le seul geste qui engage — écrire.
struct FilDetailView: View {
    let fil: ThreadCard

    @Environment(ModeleApplication.self) private var modele
    @Environment(\.dismiss) private var dismiss

    @State private var fragmentChoisi: Fragment?
    @State private var reponse = ""
    @State private var envoiEnCours = false
    @State private var confirmeDenouage = false
    @FocusState private var champActif: Bool

    /// Longueur minimale d'une réponse, alignée sur `RESPONSE_MIN_CHARS`.
    private let minimum = 12

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    entete

                    ForEach(fil.fragments) { fragment in
                        FragmentCarte(
                            fragment: fragment,
                            choisi: fragmentChoisi?.id == fragment.id,
                            selectionnable: fil.awaitingYou && fragment.kind != .motif
                        ) {
                            fragmentChoisi = fragment
                            champActif = true
                        }
                    }

                    if fil.awaitingYou {
                        redaction
                    } else {
                        Text("Votre réponse est partie. C'est au tour de \(fil.displayName).")
                            .font(.subheadline)
                            .foregroundStyle(.secondary)
                    }
                }
                .padding(16)
            }
            .background(Color.weaveLin.ignoresSafeArea())
            .navigationTitle(fil.displayName)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    Button("Fermer") { dismiss() }
                }
                ToolbarItem(placement: .topBarTrailing) {
                    Menu {
                        Button("Prolonger de 24 h", systemImage: "clock.arrow.circlepath") {
                            Task { await modele.loom.extend(fil) }
                        }
                        Button("Dénouer ce fil", systemImage: "scissors", role: .destructive) {
                            confirmeDenouage = true
                        }
                    } label: {
                        Label("Options", systemImage: "ellipsis.circle")
                    }
                }
            }
            .confirmationDialog(
                "Dénouer ce fil ?",
                isPresented: $confirmeDenouage,
                titleVisibility: .visible
            ) {
                Button("Dénouer", role: .destructive) {
                    Task {
                        await modele.loom.release(fil, reason: nil)
                        dismiss()
                    }
                }
                Button("Annuler", role: .cancel) {}
            } message: {
                Text("Il disparaîtra définitivement, et \(fil.displayName) ne recevra aucun message.")
            }
        }
    }

    private var entete: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(fil.motif.joined(separator: " · "))
                .font(.subheadline)
                .foregroundStyle(.secondary)

            HStack(spacing: 12) {
                Label("\(fil.distanceKm) km", systemImage: "location")
                Label(fil.city, systemImage: "building.2")
                Spacer()
                Text(timerInterval: Date.now...max(fil.expiresAt, .now), countsDown: true)
                    .monospacedDigit()
            }
            .font(.caption)
            .foregroundStyle(.secondary)

            RevelationBarre(pourcentage: fil.revealPercent)
        }
    }

    private var redaction: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(
                fragmentChoisi == nil
                    ? "Choisissez un fragment auquel répondre."
                    : "Votre réponse à « \(fragmentChoisi!.prompt) »"
            )
            .font(.footnote)
            .foregroundStyle(.secondary)

            TextField("Écrivez votre réponse", text: $reponse, axis: .vertical)
                .lineLimit(3...8)
                .textFieldStyle(.plain)
                .padding(12)
                .background(.background, in: RoundedRectangle(cornerRadius: 12))
                .focused($champActif)
                .disabled(fragmentChoisi == nil)

            HStack {
                Text("\(reponse.count) / 480")
                    .font(.caption.monospacedDigit())
                    .foregroundStyle(reponse.count < minimum ? .secondary : Color.weaveCuivre)
                Spacer()
                Button {
                    Task { await envoyer() }
                } label: {
                    if envoiEnCours {
                        ProgressView()
                    } else {
                        Text("Répondre")
                    }
                }
                .buttonStyle(.borderedProminent)
                .disabled(!peutEnvoyer)
            }

            // Le geste est volontairement coûteux : répondre demande d'écrire,
            // pas de balayer. C'est la mécanique centrale du produit.
            Text("Répondre engage le fil. Ne rien faire le laisse se dénouer.")
                .font(.caption)
                .foregroundStyle(.tertiary)
        }
    }

    private var peutEnvoyer: Bool {
        fragmentChoisi != nil
            && reponse.trimmingCharacters(in: .whitespacesAndNewlines).count >= minimum
            && !envoiEnCours
    }

    private func envoyer() async {
        guard let fragment = fragmentChoisi else { return }
        envoiEnCours = true
        defer { envoiEnCours = false }

        let succes = await modele.loom.respond(
            to: fil,
            fragment: fragment,
            body: reponse.trimmingCharacters(in: .whitespacesAndNewlines)
        )
        if succes { dismiss() }
    }
}

private struct FragmentCarte: View {
    let fragment: Fragment
    let choisi: Bool
    let selectionnable: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            VStack(alignment: .leading, spacing: 6) {
                Text(fragment.kind == .motif ? "Motif" : fragment.prompt)
                    .font(.caption.weight(.medium))
                    .foregroundStyle(.secondary)

                if fragment.kind == .voix {
                    Label(
                        "Fragment vocal · \(fragment.durationSeconds ?? 8) s",
                        systemImage: "waveform"
                    )
                    .font(.callout)
                } else {
                    Text(fragment.body)
                        .font(.callout)
                        .multilineTextAlignment(.leading)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(14)
            .background(.background, in: RoundedRectangle(cornerRadius: 14))
            .overlay(
                RoundedRectangle(cornerRadius: 14)
                    .stroke(choisi ? Color.weaveCuivre : .clear, lineWidth: 2)
            )
        }
        .buttonStyle(.plain)
        .disabled(!selectionnable)
    }
}

struct RevelationBarre: View {
    let pourcentage: Int

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            GeometryReader { proxy in
                ZStack(alignment: .leading) {
                    Capsule().fill(.quaternary)
                    Capsule()
                        .fill(Color.weaveCuivre)
                        .frame(width: proxy.size.width * Double(pourcentage) / 100)
                }
            }
            .frame(height: 6)

            Text("Photo dévoilée à \(pourcentage) %")
                .font(.caption2)
                .foregroundStyle(.secondary)
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Photo dévoilée à \(pourcentage) pour cent")
    }
}
