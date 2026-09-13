import SwiftUI
import WeaveKit

/// Réglages › Confidentialité : donner et retirer son consentement.
///
/// La politique de confidentialité promet trois choses, et nomme cet écran :
/// le consentement aux données sensibles est demandé **séparément**, jamais par
/// une case unique valant acceptation de tout le reste ; il se retire **à tout
/// moment depuis l'application** ; et le service continue de fonctionner après,
/// avec un fil non filtré sur ce critère.
///
/// Rien de cela n'existait. Le critère de genre s'enregistrait sans qu'aucun
/// consentement n'ait jamais été demandé, et il n'y avait aucun chemin dans
/// l'application vers un retrait. « Réglages › Confidentialité » désignait un
/// écran qui n'était pas là.
struct ConfidentialiteView: View {
    @Environment(ModeleApplication.self) private var modele
    @Environment(\.dismiss) private var dismiss

    @State private var etat: Consentements?
    @State private var enCours: ConsentKind?
    @State private var confirmeRetrait: ConsentKind?
    @State private var erreur: String?

    var body: some View {
        NavigationStack {
            Form {
                if let etat {
                    ForEach(ConsentKind.allCases, id: \.self) { objet in
                        section(objet, actif: etat.estActif(objet), etat: etat.etat(objet))
                    }
                } else if erreur == nil {
                    Section { ProgressView().frame(maxWidth: .infinity) }
                }

                if let erreur {
                    Section {
                        Text(erreur)
                            .font(.footnote)
                            .foregroundStyle(.red)
                    }
                }

                Section {
                    Link("Lire la politique de confidentialité", destination: politique)
                } footer: {
                    Text("Le texte fait foi. Cet écran ne fait que vous permettre de donner et de retirer votre accord.")
                }
            }
            .navigationTitle("Confidentialité")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Fermer") { dismiss() }
                }
            }
            .task { await charger() }
            // Une alerte de confirmation, et une seule : deux alertes empilées
            // sur la même vue ne s'affichent pas de façon fiable.
            .alert(
                "Retirer votre accord ?",
                isPresented: Binding(
                    get: { confirmeRetrait != nil },
                    set: { if !$0 { confirmeRetrait = nil } }
                ),
                presenting: confirmeRetrait
            ) { objet in
                Button("Retirer", role: .destructive) {
                    Task { await retirer(objet) }
                }
                Button("Annuler", role: .cancel) {}
            } message: { _ in
                Text("Le critère est effacé et le fil cesse de filtrer dessus. Votre compte, vos plans et vos conversations ne changent pas.")
            }
        }
    }

    @ViewBuilder
    private func section(
        _ objet: ConsentKind,
        actif: Bool,
        etat: Consentement?
    ) -> some View {
        Section {
            Text(objet.explication)
                .font(.subheadline)
                .foregroundStyle(.secondary)

            if actif {
                Button("Retirer mon accord", role: .destructive) {
                    confirmeRetrait = objet
                }
                .disabled(enCours == objet)
            } else {
                Button {
                    Task { await donner(objet) }
                } label: {
                    HStack {
                        Text("Donner mon accord")
                        if enCours == objet {
                            Spacer()
                            ProgressView()
                        }
                    }
                }
                .disabled(enCours == objet)
            }
        } header: {
            HStack {
                Text(objet.titre)
                Spacer()
                Text(actif ? "Accordé" : "Non accordé")
                    .font(.caption)
                    .foregroundStyle(actif ? Color.weaveCuivre : .secondary)
            }
        } footer: {
            // Dire la date, c'est la moitié de la promesse : la politique
            // annonce que l'octroi et le retrait sont conservés avec la leur.
            if let etat, let date = actif ? etat.grantedAt : etat.revokedAt {
                Text(actif
                    ? "Accordé le \(date.formatted(date: .long, time: .omitted))."
                    : "Retiré le \(date.formatted(date: .long, time: .omitted)).")
            } else if !actif {
                Text("Sans cet accord, vous pouvez utiliser Weave normalement : le fil n'est simplement pas filtré par genre.")
            }
        }
    }

    private var politique: URL {
        modele.api.pagePublique("confidentialite/")
    }

    private func charger() async {
        do {
            etat = try await modele.api.consents()
            erreur = nil
        } catch let erreurAPI as WeaveAPIError {
            erreur = erreurAPI.userMessage
        } catch {
            erreur = error.localizedDescription
        }
    }

    private func donner(_ objet: ConsentKind) async {
        guard let version = etat?.policyVersion else { return }
        enCours = objet
        defer { enCours = nil }
        do {
            try await modele.api.grantConsent(objet, version: version)
            await charger()
            // Les critères viennent de changer de ce qu'ils autorisent.
            await modele.plans.refresh()
        } catch let erreurAPI as WeaveAPIError {
            erreur = erreurAPI.userMessage
        } catch {
            erreur = error.localizedDescription
        }
    }

    private func retirer(_ objet: ConsentKind) async {
        enCours = objet
        defer { enCours = nil }
        do {
            try await modele.api.revokeConsent(objet)
            await charger()
            // Le serveur vient d'effacer le critère : le fil affiché est
            // celui d'avant, et il filtrerait encore à l'écran.
            await modele.plans.refresh()
        } catch let erreurAPI as WeaveAPIError {
            erreur = erreurAPI.userMessage
        } catch {
            erreur = error.localizedDescription
        }
    }
}
