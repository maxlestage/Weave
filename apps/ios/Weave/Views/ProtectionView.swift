import SwiftUI
import WeaveKit

/// Bloquer et signaler, depuis partout où quelqu'un en croise un autre.
///
/// ## Pourquoi ces gestes existent ici
///
/// L'API les servait depuis le début — `POST /v1/blocks`, `POST /v1/reports` —
/// et l'application ne les appelait nulle part. Un service où l'on écrit à des
/// inconnus sans pouvoir les bloquer ni les signaler n'est pas seulement
/// incomplet : Apple le refuse. La règle 1.2 des directives de l'App Store
/// exige, pour toute application portant du contenu écrit par ses
/// utilisateurs, un moyen de signaler un contenu et un moyen de bloquer son
/// auteur.
///
/// ## Ce que l'écran ne dit pas
///
/// Rien ne revient vers la personne visée. Ni au blocage, ni au signalement :
/// prévenir quelqu'un qu'il vient d'être bloqué n'apaise rien et expose celui
/// qui s'est protégé. C'est la règle tenue côté serveur, rappelée ici parce
/// que c'est ici qu'on serait tenté d'afficher « demande envoyée à… ».
///
/// Un signalement bloque d'office : inutile de faire les deux.
struct MenuDeProtection: View {
    let compteID: String
    let prenom: String

    /// Appelé quand le lien est coupé — la vue appelante décide quoi fermer.
    var apresCoupure: () -> Void = {}

    @Environment(ModeleApplication.self) private var modele
    @State private var demandeBlocage = false
    @State private var signalement = false
    @State private var erreur: String?

    var body: some View {
        Menu {
            Button(role: .destructive) {
                signalement = true
            } label: {
                Label("Signaler \(prenom)", systemImage: "flag")
            }
            Button(role: .destructive) {
                demandeBlocage = true
            } label: {
                Label("Bloquer \(prenom)", systemImage: "hand.raised")
            }
        } label: {
            Image(systemName: "ellipsis.circle")
                .accessibilityLabel("Signaler ou bloquer \(prenom)")
        }
        .alert("Bloquer \(prenom) ?", isPresented: $demandeBlocage) {
            Button("Annuler", role: .cancel) {}
            Button("Bloquer", role: .destructive) { Task { await bloquer() } }
        } message: {
            Text("Vous ne verrez plus ses plans, et il ne verra plus les vôtres. Votre conversation se ferme et les demandes en attente entre vous expirent.\n\n\(prenom) n'en est pas informé.")
        }
        .sheet(isPresented: $signalement) {
            SignalementView(compteID: compteID, prenom: prenom, apresEnvoi: apresCoupure)
        }
        .alert("Échec", isPresented: .constant(erreur != nil)) {
            Button("D'accord") { erreur = nil }
        } message: {
            Text(erreur ?? "")
        }
    }

    private func bloquer() async {
        do {
            try await modele.api.block(accountID: compteID)
            await modele.plans.refresh()
            apresCoupure()
        } catch {
            erreur = "Le blocage n'a pas abouti. Réessayez dans un moment."
        }
    }
}

/// Le formulaire de signalement.
///
/// Le motif est obligatoire et pris dans la liste que le serveur accepte : un
/// motif libre serait refusé côté serveur, et il n'y a pas de raison de
/// laisser écrire un texte pour le rejeter ensuite.
struct SignalementView: View {
    let compteID: String
    let prenom: String
    var apresEnvoi: () -> Void = {}

    @Environment(ModeleApplication.self) private var modele
    @Environment(\.dismiss) private var dismiss

    @State private var motif: ReportReason?
    @State private var precisions = ""
    @State private var envoi = false
    @State private var erreur: String?

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    ForEach(ReportReason.allCases) { candidat in
                        Button {
                            motif = candidat
                        } label: {
                            HStack {
                                Text(candidat.libelle).foregroundStyle(.primary)
                                Spacer()
                                if motif == candidat {
                                    Image(systemName: "checkmark").foregroundStyle(.tint)
                                }
                            }
                        }
                    }
                } header: {
                    Text("Que se passe-t-il ?")
                }

                Section {
                    TextField("Précisions", text: $precisions, axis: .vertical)
                        .lineLimit(3...8)
                } header: {
                    Text(motif?.exigeDesPrecisions == true ? "Précisions (obligatoires)" : "Précisions (facultatives)")
                } footer: {
                    Text("\(precisions.count) / \(reportDetailsMaxChars) caractères. N'écrivez que ce qui aide à comprendre : ces précisions sont lues par une personne.")
                }
            }
            .navigationTitle("Signaler \(prenom)")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Annuler") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Envoyer") { Task { await envoyer() } }
                        .disabled(!estEnvoyable || envoi)
                }
            }
            .alert("Échec", isPresented: .constant(erreur != nil)) {
                Button("D'accord") { erreur = nil }
            } message: {
                Text(erreur ?? "")
            }
        }
    }

    /// Un signalement part quand il dit quelque chose : un motif, et le texte
    /// que ce motif réclame. « Autre », seul, ne renseigne personne.
    private var estEnvoyable: Bool {
        guard let motif else { return false }
        let texte = precisions.trimmingCharacters(in: .whitespacesAndNewlines)
        guard texte.count <= reportDetailsMaxChars else { return false }
        return motif.exigeDesPrecisions ? !texte.isEmpty : true
    }

    private func envoyer() async {
        guard let motif else { return }
        envoi = true
        defer { envoi = false }
        do {
            try await modele.api.report(
                accountID: compteID, reason: motif, details: precisions
            )
            // Le signalement bloque d'office côté serveur : le fil vient de
            // changer, et la conversation de se fermer.
            await modele.plans.refresh()
            apresEnvoi()
            dismiss()
        } catch {
            erreur = "Le signalement n'a pas abouti. Réessayez dans un moment."
        }
    }
}
