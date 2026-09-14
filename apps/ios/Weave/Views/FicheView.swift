import CoreLocation
import MapKit
import PhotosUI
import SwiftUI
import UIKit
import WeaveKit

/// La fiche : une ville, un genre, une phrase.
///
/// ## L'étape qui manquait
///
/// L'inscription ne demandait que le prénom et la date de naissance. Le compte
/// naissait donc au statut « onboarding », sans fiche — et sans fiche, le
/// serveur rend un fil vide et refuse toute publication avec « Renseignez
/// d'abord votre ville. »
///
/// Autrement dit : tout le monde arrivait sur une application morte, et rien
/// ne le disait. Le serveur, lui, attendait cette étape depuis le début — le
/// dépôt de la fiche est ce qui fait passer le compte à « active ».
///
/// ## Pourquoi la ville, et pas la position
///
/// Weave n'enregistre jamais de position plus précise que le kilomètre, pas
/// même la sienne. Demander l'accès à la position pour ensuite l'arrondir
/// serait réclamer beaucoup pour n'en garder presque rien — et une demande
/// d'autorisation au premier lancement se refuse par réflexe.
///
/// La ville est donc saisie, puis convertie en coordonnées par le géocodeur du
/// système. Aucune autorisation, aucune position exacte, et un champ que l'on
/// comprend en le lisant.
struct FicheView: View {
    /// Ce qui s'affiche déjà, quand on revient modifier sa fiche.
    var villeInitiale: String = ""
    var bioInitiale: String = ""
    /// À l'inscription, il n'y a pas d'échappatoire : sans fiche, rien ne
    /// marche. Depuis les réglages, on peut refermer.
    var permetDAnnuler: Bool = false

    @Environment(ModeleApplication.self) private var modele
    @Environment(\.dismiss) private var dismiss

    @State private var ville = ""
    @State private var genre: Gender?
    @State private var bio = ""
    @State private var choixPhoto: PhotosPickerItem?
    @State private var photo: UIImage?
    @State private var photoEnCours = false
    @State private var enCours = false
    @State private var erreur: String?

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField("Ville", text: $ville)
                        .textContentType(.addressCity)
                        .autocorrectionDisabled()
                } header: {
                    Text("Où vous êtes")
                } footer: {
                    Text("Elle sert à composer votre fil et à situer vos plans. Weave n'enregistre jamais de position plus précise que le kilomètre — pas même la vôtre.")
                }

                Section {
                    ForEach(Gender.allCases) { candidat in
                        Button {
                            genre = candidat
                        } label: {
                            HStack {
                                Text(candidat.libelle).foregroundStyle(.primary)
                                Spacer()
                                if genre == candidat {
                                    Image(systemName: "checkmark").foregroundStyle(.tint)
                                }
                            }
                        }
                    }
                } header: {
                    Text("Vous êtes")
                } footer: {
                    Text("Affiché sur vos plans, et utilisé par les critères des autres.")
                }

                Section {
                    // L'aperçu et l'indicateur vivent HORS du sélecteur.
                    //
                    // L'étiquette de `PhotosPicker` est une fermeture
                    // `@Sendable` : elle n'hérite pas de l'isolation au fil
                    // principal, et aucune lecture de `@State` ne peut y
                    // figurer. Les trois qui s'y trouvaient empêchaient la
                    // compilation de l'écran entier.
                    HStack(spacing: 14) {
                        if let photo {
                            Image(uiImage: photo)
                                .resizable()
                                .scaledToFill()
                                .frame(width: 56, height: 56)
                                .clipShape(.circle)
                        } else {
                            Image(systemName: "person.crop.circle.badge.plus")
                                .font(.largeTitle)
                                .foregroundStyle(.tint)
                        }
                        PhotosPicker(
                            photo == nil ? "Ajouter une photo" : "Changer de photo",
                            selection: $choixPhoto,
                            matching: .images
                        )
                        Spacer()
                        if photoEnCours { ProgressView() }
                    }
                    .disabled(photoEnCours)
                } header: {
                    Text("Votre photo")
                } footer: {
                    Text("Facultative. Elle n'est jamais servie par une adresse devinable : chaque affichage passe par un lien signé qui expire.")
                }

                Section {
                    TextField("Une phrase", text: $bio, axis: .vertical)
                        .lineLimit(2...4)
                } header: {
                    Text("En une phrase")
                } footer: {
                    Text("\(bio.count) / \(bioMaxChars) caractères. Facultatif. Une phrase, pas une biographie : ce sont les plans qui parlent pour vous.")
                }
            }
            .navigationTitle(permetDAnnuler ? "Votre fiche" : "Encore une chose")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                if permetDAnnuler {
                    ToolbarItem(placement: .cancellationAction) {
                        Button("Annuler") { dismiss() }
                    }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button(permetDAnnuler ? "Enregistrer" : "Commencer") {
                        Task { await enregistrer() }
                    }
                    .disabled(!estComplet || enCours)
                }
            }
            .overlay {
                if enCours {
                    ProgressView().controlSize(.large)
                }
            }
            .alert("Échec", isPresented: .constant(erreur != nil)) {
                Button("D'accord") { erreur = nil }
            } message: {
                Text(erreur ?? "")
            }
            .onAppear {
                if ville.isEmpty { ville = villeInitiale }
                if bio.isEmpty { bio = bioInitiale }
            }
            .onChange(of: choixPhoto) { _, choix in
                guard let choix else { return }
                Task { await envoyerPhoto(choix) }
            }
        }
        // Pas de geste de fermeture à l'inscription : sans fiche, l'écran
        // d'après est vide et la publication refusée. Mieux vaut un écran dont
        // on ne sort qu'en le remplissant qu'une application qui ne marche pas
        // sans dire pourquoi.
        .interactiveDismissDisabled(!permetDAnnuler)
    }

    private var estComplet: Bool {
        !ville.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && genre != nil
            && bio.count <= bioMaxChars
    }

    private func enregistrer() async {
        guard let genre else { return }
        let nom = ville.trimmingCharacters(in: .whitespacesAndNewlines)
        enCours = true
        defer { enCours = false }

        let position: CLLocationCoordinate2D
        do {
            position = try await Self.situer(nom)
        } catch {
            erreur = "Cette ville n'a pas été trouvée. Vérifiez l'orthographe, ou essayez la grande ville la plus proche."
            return
        }

        do {
            try await modele.api.submitProfile(
                city: nom,
                latitude: position.latitude,
                longitude: position.longitude,
                gender: genre,
                bio: bio
            )
            // Le dépôt fait passer le compte de « onboarding » à « active » :
            // il faut relire le compte pour que l'application s'en aperçoive.
            // La photo choisie avant que la fiche n'existe part maintenant :
            // le serveur la rattache à la fiche, et refusait donc jusqu'ici.
            if let photo, !permetDAnnuler {
                await televerser(photo)
            }
            await modele.rafraichirMoi()
            await modele.plans.refresh()
            if permetDAnnuler { dismiss() }
        } catch {
            erreur = "L'enregistrement n'a pas abouti. Réessayez dans un moment."
        }
    }

    /// Recompresse la photo choisie, puis l'envoie.
    ///
    /// La fiche doit exister d'abord — le serveur refuse une photo sans ville,
    /// puisqu'elle s'attache à la fiche. À l'inscription, l'envoi est donc
    /// différé jusqu'au dépôt de la fiche ; ailleurs, il part tout de suite.
    private func envoyerPhoto(_ choix: PhotosPickerItem) async {
        photoEnCours = true
        defer { photoEnCours = false }

        guard let brut = try? await choix.loadTransferable(type: Data.self),
              let image = UIImage(data: brut)
        else {
            erreur = "Cette image n'a pas pu être lue."
            return
        }

        let reduite = Self.reduire(image)
        photo = reduite

        // Sans fiche encore déposée, il n'y a rien à quoi rattacher la photo :
        // elle partira juste après, dans `enregistrer`.
        guard permetDAnnuler else { return }
        await televerser(reduite)
    }

    private func televerser(_ image: UIImage) async {
        guard let octets = image.jpegData(compressionQuality: 0.82) else {
            erreur = "Cette image n'a pas pu être préparée."
            return
        }
        guard octets.count <= photoMaxBytes else {
            erreur = "Cette image est trop lourde, même après réduction."
            return
        }
        do {
            _ = try await modele.api.submitPhoto(octets)
            await modele.rafraichirMoi()
        } catch {
            erreur = "L'envoi de la photo n'a pas abouti. Réessayez dans un moment."
        }
    }

    /// Ramène le côté le plus long à `photoMaxCote`, en gardant les
    /// proportions. Une photo déjà plus petite n'est pas agrandie.
    private static func reduire(_ image: UIImage) -> UIImage {
        // Les conversions sont écrites plutôt que laissées à l'implicite :
        // `CGFloat` et `Double` se convertissent seuls depuis Swift 5.5, mais
        // le lecteur ne sait alors plus dans quel type le calcul se fait.
        let largeur = Double(image.size.width)
        let hauteur = Double(image.size.height)
        let cote = max(largeur, hauteur)
        guard cote > photoMaxCote else { return image }

        let facteur = photoMaxCote / cote
        let cible = CGSize(width: largeur * facteur, height: hauteur * facteur)
        let rendu = UIGraphicsImageRenderer(size: cible)
        return rendu.image { _ in image.draw(in: CGRect(origin: .zero, size: cible)) }
    }

    /// Convertit un nom de ville en coordonnées.
    ///
    /// Le géocodeur du système, plutôt qu'un service tiers : rien ne sort de
    /// l'appareil vers nous, et il n'y a pas de clé d'API à porter.
    ///
    /// `CLGeocoder` est retiré depuis iOS 26 au profit de `MKGeocodingRequest`.
    /// Le dépôt compilant avec `SWIFT_TREAT_WARNINGS_AS_ERRORS`, l'obsolescence
    /// n'était pas un avertissement : l'application ne se construisait pas.
    private static func situer(_ ville: String) async throws -> CLLocationCoordinate2D {
        guard let requete = MKGeocodingRequest(addressString: ville) else {
            throw VilleIntrouvable()
        }
        let trouvees = try await requete.mapItems
        guard let position = trouvees.first?.location.coordinate else {
            throw VilleIntrouvable()
        }
        return position
    }

    /// Le géocodeur n'a rien rendu pour ce nom.
    private struct VilleIntrouvable: Error {}
}
