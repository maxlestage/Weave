import SwiftUI
import WeaveKit

struct ReglagesView: View {
    @Environment(ModeleApplication.self) private var modele
    @Environment(\.dismiss) private var dismiss

    @State private var distance = 25
    @State private var ageMin = 18
    @State private var ageMax = 32

    @State private var exportEnCours = false
    @State private var fichierExporte: URL?
    @State private var demandeSuppression = false
    @State private var pauseEnCours = false
    @State private var erreur: String?

    var body: some View {
        NavigationStack {
            Form {
                if let moi = modele.moi {
                    Section("Vous") {
                        LabeledContent("Prénom", value: moi.displayName)
                        LabeledContent("Ville", value: moi.city)
                        LabeledContent("Offre", value: moi.tier.displayName)
                    }

                    Section {
                        LabeledContent("Aujourd'hui", value: "\(moi.requestsLeftToday)")
                    } header: {
                        Text("Demandes restantes")
                    } footer: {
                        Text("Bornées à toutes les offres, socle gratuit compris. Elles reviennent à minuit. C'est ce qui empêche d'arroser — sur Weave, une demande vaut quelque chose.")
                    }
                }

                Section {
                    Stepper("Jusqu'à \(distance) km", value: $distance, in: 1...100, step: 5)
                        .onChange(of: distance) { _, valeur in
                            Task {
                                try? await modele.api.updatePreferences(
                                    PreferencesPatch(maxDistanceKm: valeur)
                                )
                                await modele.plans.refresh()
                            }
                        }
                    Stepper("À partir de \(ageMin) ans", value: $ageMin, in: 18...98)
                    Stepper("Jusqu'à \(ageMax) ans", value: $ageMax, in: 18...99)
                } header: {
                    Text("Critères du fil")
                } footer: {
                    Text("Ils filtrent ce que vous voyez ; ils ne changent jamais l'ordre. Le fil est trié par ce qui arrive le plus tôt, puis par ce qui est le plus près.")
                }
                .onChange(of: ageMin) { _, _ in Task { await appliquerAges() } }
                .onChange(of: ageMax) { _, _ in Task { await appliquerAges() } }

                if let moi = modele.moi {
                    Section("Crédits") {
                        ForEach(UnitSku.allCases, id: \.self) { sku in
                            LabeledContent(sku.displayName, value: "\(moi.credits(for: sku))")
                        }
                    }
                }

                Section {
                    LabeledContent(
                        "Live Activity",
                        value: modele.activites.isEnabled ? "Autorisée" : "Désactivée"
                    )
                    LabeledContent("Notifications", value: etatNotifications)
                } footer: {
                    Text("La bannière affiche votre prochain plan et ce qui attend une réponse. Une notification vous prévient qu'un message est arrivé. Ni l'une ni l'autre ne dit qui écrit, ni ce qui est écrit — elles s'affichent sur un écran verrouillé.")
                }

                // Les deux droits que la politique de confidentialité annonce :
                // obtenir ses données, et partir. Apple exige par ailleurs que
                // la suppression du compte soit possible depuis l'application.
                Section {
                    Button {
                        Task { await exporter() }
                    } label: {
                        HStack {
                            Text("Obtenir mes données")
                            if exportEnCours {
                                Spacer()
                                ProgressView()
                            }
                        }
                    }
                    .disabled(exportEnCours)

                    if let fichierExporte {
                        ShareLink(item: fichierExporte) {
                            Label("Enregistrer le fichier", systemImage: "square.and.arrow.up")
                        }
                    }
                } header: {
                    Text("Vos données")
                } footer: {
                    Text("Un fichier JSON contenant ce que vous avez écrit et ce que le service sait de vous. Il ne contient pas les messages écrits par d'autres, ni l'identité de qui vous aurait signalé : ce sont leurs données.")
                }

                // Souffler sans partir. La page publique « Supprimer votre
                // compte » renvoie ici : elle propose la pause à qui voulait
                // seulement s'absenter, et il faut donc qu'elle existe.
                if let moi = modele.moi {
                    Section {
                        Button {
                            Task { await basculerPause(vers: moi.status != .paused) }
                        } label: {
                            HStack {
                                Text(moi.status == .paused ? "Reprendre" : "Mettre en pause")
                                if pauseEnCours {
                                    Spacer()
                                    ProgressView()
                                }
                            }
                        }
                        .disabled(pauseEnCours)
                    } header: {
                        Text("Pause")
                    } footer: {
                        Text(moi.status == .paused
                            ? "Vous êtes en pause. Vos plans sont retirés du fil et personne ne peut vous écrire de demande. Reprendre les remet tels quels."
                            : "Vos plans sortent du fil et personne ne peut plus demander à venir. Rien n'est supprimé : vos conversations vous attendent, et vos plans reviennent à la reprise.")
                    }
                }

                Section {
                    Button("Se déconnecter", role: .destructive) {
                        Task {
                            await modele.seDeconnecter()
                            dismiss()
                        }
                    }
                    Button("Supprimer mon compte", role: .destructive) {
                        demandeSuppression = true
                    }
                } footer: {
                    Text("Vos plans ouverts disparaissent du fil immédiatement. Tout le reste est effacé sous \(accountPurgeDays) jours.")
                }
            }
            .alert("Supprimer votre compte ?", isPresented: $demandeSuppression) {
                Button("Annuler", role: .cancel) {}
                Button("Supprimer", role: .destructive) {
                    Task { await supprimer() }
                }
            } message: {
                Text("Vos plans, vos demandes et vos conversations seront effacés. Cette action ne s'annule pas.\n\nUn abonnement souscrit via l'App Store se résilie séparément, dans les réglages de votre compte Apple.")
            }
            .alert("Échec", isPresented: .constant(erreur != nil)) {
                Button("D'accord") { erreur = nil }
            } message: {
                Text(erreur ?? "")
            }
            .navigationTitle("Réglages")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Terminé") { dismiss() }
                }
            }
        }
    }

    /// Récupère l'export et le pose dans un fichier temporaire.
    ///
    /// Le passer par un fichier plutôt que par la mémoire donne au partage un
    /// nom lisible — « weave-mes-donnees.json » — au lieu d'un contenu anonyme
    /// que les applications de destination ne savent pas nommer.
    private func exporter() async {
        exportEnCours = true
        defer { exportEnCours = false }
        do {
            let donnees = try await modele.api.exportData()
            let cible = URL.temporaryDirectory.appending(path: "weave-mes-donnees.json")
            try donnees.write(to: cible, options: .atomic)
            fichierExporte = cible
        } catch {
            erreur = "L'export n'a pas abouti. Réessayez dans un moment."
        }
    }

    /// Met le compte en pause, ou le reprend.
    ///
    /// Le profil est rechargé ensuite : c'est lui qui porte le statut, et donc
    /// le libellé du bouton. Le fil aussi — ses propres plans viennent d'en
    /// sortir, ou d'y revenir.
    private func basculerPause(vers pause: Bool) async {
        pauseEnCours = true
        defer { pauseEnCours = false }
        do {
            try await modele.api.setPaused(pause)
            await modele.rafraichirMoi()
            await modele.plans.refresh()
        } catch {
            erreur = pause
                ? "La mise en pause n'a pas abouti. Réessayez dans un moment."
                : "La reprise n'a pas abouti. Réessayez dans un moment."
        }
    }

    private func supprimer() async {
        do {
            try await modele.api.deleteAccount()
            await modele.seDeconnecter()
            dismiss()
        } catch {
            erreur = "La suppression n'a pas abouti. Réessayez dans un moment."
        }
    }

    /// Ce que le système a répondu. Tant qu'on n'a rien demandé, il n'y a rien
    /// à annoncer : afficher « désactivées » laisserait croire à un refus.
    private var etatNotifications: String {
        switch modele.notifications.autorise {
        case .some(true): "Autorisées"
        case .some(false): "Refusées"
        case nil: "Pas encore demandées"
        }
    }

    private func appliquerAges() async {
        // L'API refuse un minimum supérieur au maximum : on l'évite ici plutôt
        // que d'afficher une erreur pour un réglage qu'on peut corriger seul.
        guard ageMin <= ageMax else { return }
        try? await modele.api.updatePreferences(
            PreferencesPatch(minAge: ageMin, maxAge: ageMax)
        )
        await modele.plans.refresh()
    }
}
