import SwiftUI
import WeaveKit

/// Demander à venir à un plan.
///
/// Il n'y a pas de bouton « je viens » : on écrit. Le seuil de caractères n'est
/// pas décoratif — c'est ce qui distingue une demande d'un réflexe, et c'est la
/// même règle côté serveur.
struct DemanderView: View {
    let plan: Plan

    @Environment(ModeleApplication.self) private var modele
    @Environment(\.dismiss) private var dismiss

    @State private var message = ""
    @State private var enCours = false
    @State private var erreur: String?
    @State private var offres = false
    /// Vrai quand le dernier refus portait sur une offre ou un crédit.
    @State private var manqueUneOffre = false
    @State private var renfortEnCours = false

    private var caracteres: Int { message.trimmingCharacters(in: .whitespacesAndNewlines).count }
    private var assezEcrit: Bool { caracteres >= JoinRequest.minimumMessageLength }

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    PlanCarte(plan: plan)

                    if plan.requested {
                        Text("Vous avez déjà demandé à venir. On ne redemande pas deux fois.")
                            .font(.subheadline)
                            .foregroundStyle(.secondary)
                    } else if plan.seatsLeft == 0 {
                        Text("Ce plan est complet.")
                            .font(.subheadline)
                            .foregroundStyle(.secondary)
                    } else {
                        redaction
                    }
                }
                .padding(16)
            }
            .background(Color.weaveLin.ignoresSafeArea())
            // Au retour des offres, un Renfort vient peut-être d'être acheté :
            // on relit les crédits pour pouvoir le proposer aussitôt.
            .sheet(isPresented: $offres, onDismiss: {
                Task { await modele.plans.refreshCredits() }
            }) {
                OffresView().environment(modele)
            }
            .task { await modele.plans.refreshCredits() }
            .navigationTitle("Demander à venir")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Fermer") { dismiss() }
                }
                // Le second endroit où l'on croise quelqu'un : sa fiche, vue
                // depuis le fil. Un profil qu'on ne veut pas signaler une fois
                // la conversation ouverte se signale d'ici — et c'est souvent
                // ici qu'on s'en aperçoit, avant d'avoir écrit.
                ToolbarItem(placement: .topBarTrailing) {
                    MenuDeProtection(
                        compteID: plan.author.id,
                        prenom: plan.author.displayName
                    ) {
                        dismiss()
                    }
                }
            }
        }
    }

    private var redaction: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Dites pourquoi ce plan-là.")
                .font(.headline)

            TextEditor(text: $message)
                .frame(minHeight: 160)
                .padding(8)
                .background(Color.white, in: .rect(cornerRadius: 14))
                .overlay(alignment: .topLeading) {
                    if message.isEmpty {
                        Text("Je viens de m'installer dans le quartier et je ne connais personne qui grimpe…")
                            .font(.subheadline)
                            .foregroundStyle(.tertiary)
                            .padding(.horizontal, 13)
                            .padding(.vertical, 16)
                            .allowsHitTesting(false)
                    }
                }

            HStack {
                Text(compteur)
                    .font(.caption)
                    .foregroundStyle(assezEcrit ? .secondary : Color.weaveCuivre)
                Spacer()
                Text(resteAujourdhui)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            if let erreur {
                VStack(alignment: .leading, spacing: 8) {
                    Text(erreur)
                        .font(.footnote)
                        .foregroundStyle(.red)
                    // Un refus d'offre sans moyen d'y accéder est une impasse :
                    // le message dit ce qui manque, et rien ne permet de
                    // l'obtenir.
                    if manqueUneOffre {
                        Button("Voir les offres") { offres = true }
                            .font(.footnote)
                    }
                }
            }

            if modele.plans.requestsLeftToday == 0 {
                plusDeDemandes
            }

            Button {
                Task { await envoyer() }
            } label: {
                if enCours {
                    ProgressView().frame(maxWidth: .infinity)
                } else {
                    Text("Envoyer la demande").frame(maxWidth: .infinity)
                }
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.large)
            .disabled(!assezEcrit || enCours || modele.plans.requestsLeftToday == 0)

            Text("Cette demande consomme une de vos demandes du jour. Retirée avant d'avoir été lue, elle vous est rendue.")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }

    /// Plus de demande aujourd'hui : ce qu'on peut faire, plutôt qu'un
    /// bouton grisé sans explication.
    ///
    /// Un Renfort détenu se propose en premier — il est déjà payé. Sinon, on
    /// montre où en acheter un, et l'on rappelle que le compteur revient à
    /// minuit : attendre reste une réponse gratuite, et la dire évite de
    /// laisser croire que payer est la seule.
    private var plusDeDemandes: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Plus de demande aujourd'hui. Le compteur revient à minuit.")
                .font(.footnote)
                .foregroundStyle(.secondary)

            if modele.plans.renfortsDisponibles > 0 {
                Button {
                    Task { await utiliserRenfort() }
                } label: {
                    HStack {
                        if renfortEnCours { ProgressView() }
                        Text(modele.plans.renfortsDisponibles == 1
                            ? String(localized: "Utiliser mon Renfort — 5 demandes de plus")
                            : String(localized: "Utiliser un Renfort (\(modele.plans.renfortsDisponibles) restants) — 5 demandes de plus"))
                    }
                }
                .buttonStyle(.bordered)
                .disabled(renfortEnCours)
            } else {
                Button("Un Renfort : 5 demandes de plus aujourd'hui") { offres = true }
                    .buttonStyle(.bordered)
            }
        }
    }

    private func utiliserRenfort() async {
        renfortEnCours = true
        defer { renfortEnCours = false }
        erreur = nil
        if !(await modele.plans.useRenfort()) {
            erreur = modele.plans.alert?.userMessage
        }
    }

    private var compteur: String {
        assezEcrit
            ? String(localized: "\(caracteres) caractères")
            : String(localized: "Encore \(JoinRequest.minimumMessageLength - caracteres) caractères")
    }

    private var resteAujourdhui: String {
        let reste = modele.plans.requestsLeftToday
        return reste == 1
            ? String(localized: "1 demande restante")
            : String(localized: "\(reste) demandes restantes")
    }

    private func envoyer() async {
        enCours = true
        erreur = nil
        let envoye = await modele.plans.join(
            plan,
            message: message.trimmingCharacters(in: .whitespacesAndNewlines)
        )
        enCours = false
        if envoye {
            dismiss()
        } else {
            erreur = modele.plans.alert?.userMessage
            manqueUneOffre = {
                if case .entitlementRequired = modele.plans.alert { return true }
                return false
            }()
        }
    }
}
