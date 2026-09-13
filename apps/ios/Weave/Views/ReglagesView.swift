import SwiftUI
import WeaveKit

struct ReglagesView: View {
    @Environment(ModeleApplication.self) private var modele
    @Environment(\.dismiss) private var dismiss

    @State private var distance = 25
    @State private var ageMin = 18
    @State private var ageMax = 32
    /// Les critères ne s'appliquent qu'une fois lus.
    ///
    /// Les trois valeurs ci-dessus sont des valeurs de départ, pas les
    /// réglages réels — l'application ne relisait jamais ses critères. Or les
    /// deux âges s'appliquent ensemble : toucher un curseur avant d'avoir lu
    /// aurait écrasé l'autre avec une valeur par défaut.
    @State private var criteresLus = false
    @State private var jours: Set<Int> = []
    @State private var escaleVille: String?
    @State private var escaleFin: Date?
    @State private var nouvelleEscale = false
    @State private var villeEscale = ""
    @State private var escaleEnCours = false
    @State private var bilan: Bilan?
    @State private var bilanEnCours = false

    @State private var exportEnCours = false
    @State private var fichierExporte: URL?
    @State private var demandeSuppression = false
    @State private var pauseEnCours = false
    @State private var fiche = false
    @State private var offres = false
    @State private var erreur: String?

    var body: some View {
        NavigationStack {
            Form {
                if let moi = modele.moi {
                    Section("Vous") {
                        LabeledContent("Prénom", value: moi.displayName)
                        LabeledContent("Ville", value: moi.city)
                        LabeledContent("Offre", value: moi.tier.displayName)
                        // Rien ne permettait de corriger sa fiche : une ville
                        // mal saisie à l'inscription, ou un déménagement, et
                        // le fil restait composé autour du mauvais endroit.
                        Button("Modifier ma fiche") { fiche = true }
                        Button("Voir les offres") { offres = true }
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
                            guard criteresLus else { return }
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
                if let moi = modele.moi, moi.tier.filtreParJour {
                    Section {
                        ForEach(1...7, id: \.self) { jour in
                            Button {
                                basculerJour(jour)
                            } label: {
                                HStack {
                                    Text(Self.nomDuJour(jour)).foregroundStyle(.primary)
                                    Spacer()
                                    if jours.contains(jour) {
                                        Image(systemName: "checkmark").foregroundStyle(.tint)
                                    }
                                }
                            }
                        }
                    } header: {
                        Text("Jours")
                    } footer: {
                        Text(jours.isEmpty
                            ? "Aucun jour retenu : le fil montre tous les jours."
                            : "Le fil ne montre que les plans tombant ces jours-là.")
                    }
                }

                .onChange(of: ageMin) { _, _ in
                    guard criteresLus else { return }
                    Task { await appliquerAges() }
                }
                .onChange(of: ageMax) { _, _ in
                    guard criteresLus else { return }
                    Task { await appliquerAges() }
                }

                if let moi = modele.moi {
                    Section("Crédits") {
                        ForEach(UnitSku.allCases, id: \.self) { sku in
                            LabeledContent(sku.displayName, value: "\(moi.credits(for: sku))")
                        }
                    }
                }

                if let moi = modele.moi {
                    escaleSection(moi: moi)
                    bilanSection(moi: moi)
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
            .sheet(isPresented: $offres) {
                OffresView().environment(modele)
            }
            .sheet(isPresented: $fiche) {
                FicheView(
                    villeInitiale: modele.moi?.city ?? "",
                    bioInitiale: modele.moi?.bio ?? "",
                    permetDAnnuler: true
                )
                .environment(modele)
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
            .task { await chargerCriteres() }
            .sheet(item: $bilan) { rapport in
                BilanView(bilan: rapport)
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

    /// L'escale : la déclencher, ou dire celle qui court.
    @ViewBuilder
    private func escaleSection(moi: Me) -> some View {
        let credits = moi.credits(for: .escale)
        if escaleEnCours || credits > 0 {
            Section {
                if let ville = escaleVille, let fin = escaleFin, escaleEnCours {
                    LabeledContent("En escale à", value: ville)
                    LabeledContent("Jusqu'au", value: fin.formatted(date: .abbreviated, time: .shortened))
                    Button("Revenir chez moi", role: .destructive) {
                        Task { await fermerEscale() }
                    }
                } else {
                    Button("Ouvrir une escale") {
                        villeEscale = ""
                        nouvelleEscale = true
                    }
                    // L'alerte est portée par le bouton, pas par le
                    // formulaire. Trois alertes empilées sur une même vue ne
                    // se présentent pas toujours toutes : SwiftUI n'en retient
                    // qu'une, et laquelle ne se devine pas. Le formulaire en
                    // portait déjà deux.
                    .alert("Où allez-vous ?", isPresented: $nouvelleEscale) {
                        TextField("Ville", text: $villeEscale)
                        Button("Annuler", role: .cancel) { villeEscale = "" }
                        Button("Ouvrir l'escale") { Task { await ouvrirEscale() } }
                    } message: {
                        Text("Votre fil se composera autour de cette ville pendant sept jours, et vos plans y seront visibles. Cela consomme une « Escale ».")
                    }
                }
            } header: {
                Text("Escale")
            } footer: {
                Text(escaleEnCours
                    ? "Votre fil se compose autour de cette ville, et vos plans y sont visibles. Revenir n'est pas remboursé : l'escale a servi."
                    : "Publier depuis une autre ville pendant sept jours. Vous en avez \(credits).")
            }
        }
    }

    /// Le bilan : un rapport sur ses propres plans passés.
    @ViewBuilder
    private func bilanSection(moi: Me) -> some View {
        if moi.credits(for: .bilan) > 0 {
            Section {
                Button {
                    Task { await demanderBilan() }
                } label: {
                    HStack {
                        Text("Établir mon bilan")
                        if bilanEnCours {
                            Spacer()
                            ProgressView()
                        }
                    }
                }
                .disabled(bilanEnCours)
            } header: {
                Text("Bilan")
            } footer: {
                Text("Ce qui attire, ce qui tombe à plat, sur vos plans passés. Rien n'est comparé aux autres, et les messages reçus ne sont pas lus.")
            }
        }
    }

    /// Lit les critères réels, puis n'autorise qu'ensuite l'application des
    /// curseurs. Sans cela, toucher un âge écraserait l'autre.
    private func chargerCriteres() async {
        guard !criteresLus else { return }
        guard let criteres = try? await modele.api.preferences() else { return }

        distance = criteres.maxDistanceKm
        ageMin = criteres.minAge
        ageMax = criteres.maxAge
        escaleVille = criteres.escaleCity
        escaleFin = criteres.escaleUntil
        escaleEnCours = criteres.escaleEnCours
        jours = Set(criteres.days)
        criteresLus = true
    }

    /// Retient ou retire un jour, puis applique.
    ///
    /// Le serveur refuse ce critère aux paliers qui ne l'ont pas ; la section
    /// n'apparaît donc qu'à ceux qui y ont droit, et le refus reste le dernier
    /// mot plutôt que le premier.
    private func basculerJour(_ jour: Int) {
        if jours.contains(jour) {
            jours.remove(jour)
        } else {
            jours.insert(jour)
        }
        let choisis = jours.sorted()
        Task {
            do {
                try await modele.api.updatePreferences(PreferencesPatch(days: choisis))
                await modele.plans.refresh()
            } catch let souci as WeaveAPIError {
                erreur = souci.userMessage
                // Remettre l'affichage en accord avec ce que le serveur a
                // retenu : laisser la case cochée après un refus mentirait.
                await chargerCriteresDeForce()
            } catch {
                erreur = "Le réglage n'a pas abouti. Réessayez dans un moment."
            }
        }
    }

    /// Relit les critères même s'ils l'ont déjà été.
    private func chargerCriteresDeForce() async {
        criteresLus = false
        await chargerCriteres()
    }

    private func ouvrirEscale() async {
        let ville = villeEscale.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !ville.isEmpty else { return }
        do {
            let escale = try await modele.api.openEscale(city: ville)
            escaleVille = escale.escaleCity
            escaleFin = escale.escaleUntil
            escaleEnCours = true
            villeEscale = ""
            await modele.rafraichirMoi()
            await modele.plans.refresh()
        } catch let souci as WeaveAPIError {
            erreur = souci.userMessage
        } catch {
            erreur = "L'escale n'a pas pu être ouverte. Réessayez dans un moment."
        }
    }

    private func fermerEscale() async {
        do {
            try await modele.api.closeEscale()
            escaleVille = nil
            escaleFin = nil
            escaleEnCours = false
            await modele.plans.refresh()
        } catch {
            erreur = "La fermeture n'a pas abouti. Réessayez dans un moment."
        }
    }

    private func demanderBilan() async {
        bilanEnCours = true
        defer { bilanEnCours = false }
        do {
            bilan = try await modele.api.requestBilan()
            await modele.rafraichirMoi()
        } catch let souci as WeaveAPIError {
            // Le serveur refuse sans rien dépenser quand il n'y a pas assez de
            // plans passés, et son message le dit. Le reprendre tel quel plutôt
            // que d'en inventer un : c'est lui qui connaît le compte.
            erreur = souci.userMessage
        } catch {
            erreur = "Le bilan n'a pas pu être établi. Réessayez dans un moment."
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

    /// Le nom d'un jour, au sens ISO : 1 lundi, 7 dimanche.
    ///
    /// Rendu par le calendrier plutôt qu'écrit en dur : la langue de
    /// l'appareil décide, et une liste écrite ici resterait française partout.
    private static func nomDuJour(_ jour: Int) -> String {
        var calendrier = Calendar(identifier: .gregorian)
        calendrier.locale = .current
        // `weekdaySymbols` commence au dimanche : 1 (lundi) y est l'indice 1,
        // 7 (dimanche) l'indice 0.
        let symboles = calendrier.weekdaySymbols
        let indice = jour % 7
        let nom = symboles.indices.contains(indice) ? symboles[indice] : "\(jour)"
        return nom.prefix(1).uppercased() + nom.dropFirst()
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
