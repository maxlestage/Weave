import SwiftUI
import WeaveKit

/// Le bilan de ses plans passés.
///
/// ## Ce que cet écran ne montre pas
///
/// Aucune comparaison aux autres. Weave ne classe pas les gens, et « vous
/// recevez moins que la moyenne » ferait exactement cela — sur un service de
/// rencontre, c'est la dernière chose dont quelqu'un a besoin.
///
/// Aucun message reçu non plus. Leur nombre suffit à dire ce qui attire ;
/// leur contenu appartient à qui les a écrits.
///
/// Le délai de publication est rendu en deux chiffres côte à côte plutôt qu'en
/// conseil. Le chiffre se lit et se juge ; un conseil s'impose.
struct BilanView: View {
    let bilan: Bilan

    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            List {
                Section {
                    LabeledContent("Plans passés", value: "\(bilan.plansPasses)")
                    LabeledContent("Demandes reçues", value: "\(bilan.demandesRecues)")
                    LabeledContent("Dont acceptées", value: "\(bilan.demandesAcceptees)")
                    LabeledContent("Sans aucune demande", value: "\(bilan.plansSansAucuneDemande)")
                } header: {
                    Text("Sur la période")
                } footer: {
                    Text("Du \(bilan.periode.duPremierPlan.formatted(date: .abbreviated, time: .omitted)) au \(bilan.periode.auDernier.formatted(date: .abbreviated, time: .omitted)).")
                }

                if !bilan.cequiAttire.isEmpty {
                    Section("Ce qui attire") {
                        ForEach(bilan.cequiAttire) { plan in
                            LignePlan(plan: plan)
                        }
                    }
                }

                if !bilan.ceQuiTombeAPlat.isEmpty {
                    Section {
                        ForEach(bilan.ceQuiTombeAPlat) { plan in
                            LignePlan(plan: plan)
                        }
                    } header: {
                        Text("Ce qui tombe à plat")
                    } footer: {
                        Text("Un plan sans demande n'est pas un échec personnel : le titre, l'heure et la catégorie y sont souvent pour plus que vous.")
                    }
                }

                if !bilan.parCategorie.isEmpty {
                    Section {
                        ForEach(bilan.parCategorie) { rendement in
                            LabeledContent(rendement.categorie.displayName) {
                                Text("\(rendement.demandesParPlan, format: .number.precision(.fractionLength(1))) / plan")
                                    .monospacedDigit()
                            }
                        }
                    } header: {
                        Text("Par catégorie")
                    } footer: {
                        Text("Demandes reçues par plan publié, de la plus sollicitée à la moins.")
                    }
                }

                if let prend = bilan.delai.joursAvantQuandCaPrend {
                    Section {
                        LabeledContent("Quand ça prend") {
                            Text("\(prend, format: .number.precision(.fractionLength(1))) jours avant")
                                .monospacedDigit()
                        }
                        if let pas = bilan.delai.joursAvantQuandCaNePrendPas {
                            LabeledContent("Quand ça ne prend pas") {
                                Text("\(pas, format: .number.precision(.fractionLength(1))) jours avant")
                                    .monospacedDigit()
                            }
                        }
                    } header: {
                        Text("Délai de publication")
                    } footer: {
                        Text("Combien de temps à l'avance vos plans étaient publiés. À vous d'en tirer ce que vous voulez.")
                    }
                }
            }
            .navigationTitle("Votre bilan")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Terminé") { dismiss() }
                }
            }
        }
    }
}

private struct LignePlan: View {
    let plan: Bilan.PlanCite

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(plan.titre)
            Text("\(plan.categorie.displayName) · \(plan.demandes) demande\(plan.demandes > 1 ? "s" : "") · publié \(plan.publieJoursAvant) jour\(plan.publieJoursAvant > 1 ? "s" : "") avant")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }
}
