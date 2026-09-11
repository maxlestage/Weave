import SwiftUI
import WeaveKit

/// Connexion par code à usage unique : pas de mot de passe à créer, à retenir,
/// ni à se faire dérober.
struct ConnexionView: View {
    @Environment(ModeleApplication.self) private var modele

    private enum Etape { case adresse, code, inscription }

    @State private var etape: Etape = .adresse
    @State private var email = ""
    @State private var code = ""
    @State private var prenom = ""
    @State private var naissance = Calendar.current.date(byAdding: .year, value: -30, to: .now) ?? .now
    @State private var enCours = false
    @State private var message: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 24) {
            Spacer(minLength: 40)

            VStack(alignment: .leading, spacing: 10) {
                Text("Weave")
                    .font(.system(size: 44, weight: .semibold, design: .serif))
                Text("\(Loom.maxActiveThreads) fils par jour. Pas un de plus.")
                    .font(.title3)
                    .foregroundStyle(.secondary)
            }

            switch etape {
            case .adresse: champAdresse
            case .code: champCode
            case .inscription: champInscription
            }

            if let message {
                Text(message)
                    .font(.footnote)
                    .foregroundStyle(.red)
            }

            Spacer()

            Text("En continuant, vous acceptez les conditions d'utilisation. Weave est réservé aux personnes majeures.")
                .font(.caption2)
                .foregroundStyle(.tertiary)
        }
        .padding(24)
        .background(Color.weaveLin.ignoresSafeArea())
    }

    private var champAdresse: some View {
        VStack(alignment: .leading, spacing: 14) {
            TextField("Votre adresse e-mail", text: $email)
                .textContentType(.emailAddress)
                .keyboardType(.emailAddress)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .padding(14)
                .background(.background, in: RoundedRectangle(cornerRadius: 12))

            Button {
                Task { await demanderCode() }
            } label: {
                Label("Recevoir un code", systemImage: "envelope")
                    .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.large)
            .disabled(!email.contains("@") || enCours)
        }
    }

    private var champCode: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Un code à six chiffres vient de partir vers \(email).")
                .font(.subheadline)
                .foregroundStyle(.secondary)

            TextField("000000", text: $code)
                .textContentType(.oneTimeCode)
                .keyboardType(.numberPad)
                .font(.system(.title, design: .monospaced))
                .multilineTextAlignment(.center)
                .padding(14)
                .background(.background, in: RoundedRectangle(cornerRadius: 12))
                .onChange(of: code) { _, nouveau in
                    code = String(nouveau.filter(\.isNumber).prefix(6))
                    if code.count == 6 { Task { await verifier() } }
                }

            Button("Changer d'adresse") {
                etape = .adresse
                code = ""
                message = nil
            }
            .font(.footnote)
        }
    }

    private var champInscription: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Première visite : deux informations, et c'est tout.")
                .font(.subheadline)
                .foregroundStyle(.secondary)

            TextField("Votre prénom", text: $prenom)
                .textContentType(.givenName)
                .padding(14)
                .background(.background, in: RoundedRectangle(cornerRadius: 12))

            DatePicker(
                "Date de naissance",
                selection: $naissance,
                in: ...Calendar.current.date(byAdding: .year, value: -18, to: .now)!,
                displayedComponents: .date
            )
            .datePickerStyle(.compact)

            Button {
                Task { await verifier() }
            } label: {
                Text("Créer mon compte").frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.large)
            .disabled(prenom.trimmingCharacters(in: .whitespaces).isEmpty || enCours)
        }
    }

    private func demanderCode() async {
        enCours = true
        message = nil
        defer { enCours = false }
        do {
            try await modele.api.requestCode(email: email.trimmingCharacters(in: .whitespaces))
            etape = .code
        } catch let erreur as WeaveAPIError {
            message = erreur.userMessage
        } catch {
            message = "Envoi impossible. Réessayez."
        }
    }

    private func verifier() async {
        enCours = true
        message = nil
        defer { enCours = false }

        let formatteur = DateFormatter()
        formatteur.dateFormat = "yyyy-MM-dd"

        do {
            let resultat = try await modele.api.verifyCode(
                email: email.trimmingCharacters(in: .whitespaces),
                code: code,
                displayName: etape == .inscription ? prenom : nil,
                birthDate: etape == .inscription ? formatteur.string(from: naissance) : nil
            )

            if resultat.needsProfile {
                // Le code était bon mais le compte n'existe pas encore : on
                // demande le strict nécessaire, puis on renvoie le même code.
                etape = .inscription
                return
            }
            await modele.seConnecter()
        } catch let erreur as WeaveAPIError {
            message = erreur.userMessage
            code = ""
        } catch {
            message = "Vérification impossible. Réessayez."
            code = ""
        }
    }
}
