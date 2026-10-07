import SwiftUI
import WeaveKit

/// Se connecter depuis la montre, sans l'iPhone.
///
/// La montre n'avait qu'un écran tant qu'elle n'avait pas de session :
/// « Ouvrez Weave sur votre iPhone ». Si le relais par l'iPhone ne venait pas
/// — application iPhone jamais rouverte, Bluetooth coupé, montre jumelée après
/// coup —, il n'y avait aucune autre issue. On s'y connecte désormais comme sur
/// l'iPhone : une adresse, puis le code reçu par e-mail. La saisie passe par la
/// dictée, le griffonnage ou le clavier de la montre.
///
/// Le relais par l'iPhone reste : il suffit encore d'ouvrir Weave sur l'iPhone
/// pour que la montre se connecte toute seule.
struct ConnexionMontreView: View {
    @Environment(ModeleMontre.self) private var modele

    @State private var email = ""
    @State private var code = ""
    @State private var codeEnvoye = false
    @State private var enCours = false
    @State private var message: String?

    var body: some View {
        Section {
            if codeEnvoye {
                Text("Un code est parti vers \(email).")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                TextField("Code à six chiffres", text: $code)
                Button("Se connecter") {
                    Task { await verifier() }
                }
                .disabled(code.trimmingCharacters(in: .whitespaces).count < 6 || enCours)
                Button("Changer d'adresse") {
                    codeEnvoye = false
                    code = ""
                    message = nil
                }
                .font(.caption2)
            } else {
                TextField("Adresse e-mail", text: $email)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                Button("Recevoir un code") {
                    Task { await demander() }
                }
                .disabled(!email.contains("@") || enCours)
            }

            if let message {
                Text(message)
                    .font(.caption2)
                    .foregroundStyle(.red)
            }
        } header: {
            Text("Se connecter")
        } footer: {
            Text("Ou ouvrez Weave sur l'iPhone : la montre s'y relie toute seule.")
        }
    }

    private func demander() async {
        enCours = true
        defer { enCours = false }
        message = await modele.magasin.demanderCode(email: email)
        if message == nil {
            email = email.trimmingCharacters(in: .whitespaces)
            codeEnvoye = true
        }
    }

    private func verifier() async {
        enCours = true
        defer { enCours = false }
        switch await modele.connecter(email: email, code: code) {
        case .connecte:
            message = nil
        case .compteAbsent:
            message = String(localized: "Ce compte n'existe pas encore. Créez-le dans Weave sur l'iPhone.")
        case .echec(let raison):
            message = raison
            code = ""
        }
    }
}
