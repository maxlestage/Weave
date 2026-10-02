import SwiftUI
import WeaveKit

/// Les personnes qu'on a bloquées, et le moyen de lever un blocage.
///
/// La levée existait côté serveur ; aucun écran n'y menait, et rien ne
/// permettait même de retrouver qui l'on avait bloqué. Un blocage posé sous le
/// coup d'un agacement devenait définitif.
struct BloquesView: View {
    @Environment(ModeleApplication.self) private var modele

    @State private var bloques: [BlockedAccount] = []
    @State private var charge = false
    @State private var aLever: BlockedAccount?
    @State private var erreur: String?

    var body: some View {
        List {
            if charge && bloques.isEmpty {
                Text("Vous n'avez bloqué personne.")
                    .foregroundStyle(.secondary)
            }
            ForEach(bloques) { bloque in
                VStack(alignment: .leading, spacing: 2) {
                    Text(bloque.displayName ?? String(localized: "Compte supprimé"))
                        .foregroundStyle(bloque.displayName == nil ? .secondary : .primary)
                    Text(bloque.blockedAt.formatted(date: .abbreviated, time: .omitted))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                .swipeActions {
                    Button("Débloquer") { aLever = bloque }
                        .tint(.weaveCuivre)
                }
            }
            if let erreur {
                Text(erreur)
                    .font(.footnote)
                    .foregroundStyle(.red)
            }
        }
        .navigationTitle("Personnes bloquées")
        .task { await charger() }
        .confirmationDialog(
            "Débloquer cette personne ?",
            isPresented: Binding(get: { aLever != nil }, set: { if !$0 { aLever = nil } }),
            titleVisibility: .visible,
            presenting: aLever
        ) { bloque in
            Button("Débloquer") { Task { await lever(bloque) } }
            Button("Annuler", role: .cancel) {}
        } message: { _ in
            // Ce que le déblocage ne fait PAS, dit avant : il ne rouvre rien.
            Text("Ses plans reviennent dans votre fil, et les vôtres dans le sien. Rien n'est rouvert : ni les demandes closes, ni la conversation. Un signalement déjà envoyé reste traité.")
        }
    }

    private func charger() async {
        do {
            bloques = try await modele.api.blocks()
            erreur = nil
        } catch let souci as WeaveAPIError {
            erreur = souci.userMessage
        } catch {
            erreur = String(localized: "Connexion impossible.")
        }
        charge = true
    }

    private func lever(_ bloque: BlockedAccount) async {
        do {
            try await modele.api.unblock(accountID: bloque.accountId)
            bloques.removeAll { $0.id == bloque.id }
            await modele.plans.refresh()
        } catch let souci as WeaveAPIError {
            erreur = souci.userMessage
        } catch {
            erreur = String(localized: "Connexion impossible.")
        }
    }
}

/// Changer l'adresse de connexion, en deux temps : la nouvelle adresse, puis
/// le code qu'elle reçoit.
///
/// Les routes et le client existaient depuis la PR qui les a construits ;
/// l'écran manquait, et rien ne permettait de changer d'adresse.
struct AdresseEmailView: View {
    @Environment(ModeleApplication.self) private var modele
    @Environment(\.dismiss) private var dismiss

    @State private var adresse = ""
    @State private var code = ""
    @State private var codeEnvoye = false
    @State private var codeDeDeveloppement: String?
    @State private var enCours = false
    @State private var erreur: String?
    @State private var fait = false

    private var adresseNettoyee: String {
        adresse.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    var body: some View {
        NavigationStack {
            Form {
                if fait {
                    Section {
                        Label("Adresse changée", systemImage: "checkmark.circle")
                    } footer: {
                        Text("Les prochains codes de connexion partiront à cette adresse.")
                    }
                } else {
                    Section {
                        TextField("Nouvelle adresse", text: $adresse)
                            .keyboardType(.emailAddress)
                            .textContentType(.emailAddress)
                            .textInputAutocapitalization(.never)
                            .autocorrectionDisabled()
                            .disabled(codeEnvoye)
                    } footer: {
                        // Le code part à la NOUVELLE adresse : c'est elle qu'il
                        // faut prouver, l'ancienne l'a déjà été.
                        Text("Un code part à cette adresse. Tant qu'il n'est pas confirmé, rien ne change.")
                    }

                    if codeEnvoye {
                        Section {
                            TextField("Code à six chiffres", text: $code)
                                .keyboardType(.numberPad)
                                .textContentType(.oneTimeCode)
                            if let codeDeDeveloppement {
                                Text("Code (développement) : \(codeDeDeveloppement)")
                                    .font(.footnote.monospaced())
                                    .foregroundStyle(.secondary)
                            }
                        }
                    }

                    if let erreur {
                        Section {
                            Text(erreur)
                                .foregroundStyle(.red)
                        }
                    }

                    Section {
                        Button {
                            Task { await avancer() }
                        } label: {
                            HStack {
                                Text(codeEnvoye ? "Confirmer" : "Recevoir un code")
                                if enCours {
                                    Spacer()
                                    ProgressView()
                                }
                            }
                        }
                        .disabled(enCours || adresseNettoyee.isEmpty || (codeEnvoye && code.count < 6))
                    }
                }
            }
            .navigationTitle("Adresse e-mail")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(fait ? "Fermer" : "Annuler") { dismiss() }
                }
            }
        }
    }

    private func avancer() async {
        enCours = true
        defer { enCours = false }
        erreur = nil
        do {
            if codeEnvoye {
                try await modele.api.confirmEmailChange(adresseNettoyee, code: code)
                fait = true
            } else {
                codeDeDeveloppement = try await modele.api.requestEmailChange(adresseNettoyee)
                codeEnvoye = true
            }
        } catch let souci as WeaveAPIError {
            erreur = souci.userMessage
        } catch {
            erreur = String(localized: "Connexion impossible.")
        }
    }
}
