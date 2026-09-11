import SwiftUI
import WeaveKit

/// Publier un plan : ce qu'on compte faire, quand, et combien de personnes
/// peuvent venir.
///
/// Rien ici ne demande de se décrire. Le formulaire est court exprès : un plan
/// qu'on met dix minutes à rédiger ne sera pas publié.
struct PublierView: View {
    @Environment(ModeleApplication.self) private var modele
    @Environment(\.dismiss) private var dismiss

    @State private var titre = ""
    @State private var note = ""
    @State private var categorie: PlanCategory = .sortie
    @State private var quand = Date.now.addingTimeInterval(24 * 60 * 60)
    @State private var places = 1
    @State private var enCours = false
    @State private var erreur: String?

    /// Préavis minimal, identique côté serveur (`PLAN_MIN_LEAD_MINUTES`).
    private static let preavisMinutes = 60

    private var plusTot: Date { .now.addingTimeInterval(Double(Self.preavisMinutes) * 60) }
    private var titreValide: Bool {
        titre.trimmingCharacters(in: .whitespacesAndNewlines).count >= 8
    }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField("Bloc au mur de 19 h, niveau débutant", text: $titre, axis: .vertical)
                        .lineLimit(1...3)
                } header: {
                    Text("Ce que vous faites")
                } footer: {
                    Text("Écrivez-le comme vous le diriez à quelqu'un. Pas une annonce.")
                }

                Section {
                    TextField("Je grimpe depuis six mois, très mal.", text: $note, axis: .vertical)
                        .lineLimit(2...5)
                } header: {
                    Text("Une précision, si besoin")
                } footer: {
                    Text("Le prix d'entrée, le niveau, l'heure de fin : ce qui évite un malentendu.")
                }

                Section("Quand") {
                    DatePicker("Rendez-vous", selection: $quand, in: plusTot...)
                    Picker("Catégorie", selection: $categorie) {
                        ForEach(PlanCategory.allCases, id: \.self) { categorie in
                            Label(categorie.displayName, systemImage: categorie.symbolName)
                                .tag(categorie)
                        }
                    }
                }

                Section {
                    Stepper("\(places) personne\(places > 1 ? "s" : "")", value: $places, in: 1...4)
                } header: {
                    Text("Combien de personnes peuvent venir")
                } footer: {
                    Text("Au-delà d'une, il faut les plans de groupe — compris à partir d'Escapade, ou à l'unité avec une « Tablée ».")
                }

                if let erreur {
                    Section { Text(erreur).font(.footnote).foregroundStyle(.red) }
                }
            }
            .navigationTitle("Publier un plan")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Annuler") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Publier") { Task { await publier() } }
                        .disabled(!titreValide || enCours)
                }
            }
        }
    }

    private func publier() async {
        enCours = true
        erreur = nil
        let brouillon = PlanDraft(
            title: titre.trimmingCharacters(in: .whitespacesAndNewlines),
            note: note.trimmingCharacters(in: .whitespacesAndNewlines),
            category: categorie,
            startsAt: quand,
            capacity: places
        )
        let publie = await modele.plans.publish(brouillon)
        enCours = false
        if publie {
            dismiss()
        } else {
            erreur = modele.plans.alert?.userMessage
        }
    }
}
