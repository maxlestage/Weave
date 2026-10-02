import SwiftUI
import WeaveKit

/// Les conversations ouvertes, pour répondre du poignet.
struct ConversationsMontreView: View {
    @Environment(ModeleMontre.self) private var modele

    private var conversations: ConversationsStore { modele.magasin.conversations }

    var body: some View {
        List {
            if conversations.ouvertes.isEmpty {
                Text("Aucune conversation ouverte.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            ForEach(conversations.ouvertes) { conversation in
                NavigationLink {
                    ConversationMontreView(conversation: conversation)
                } label: {
                    VStack(alignment: .leading, spacing: 2) {
                        HStack {
                            Text(conversation.other.displayName)
                                .font(.headline)
                                .privacySensitive()
                            if conversation.unread > 0 {
                                Spacer()
                                Text("\(conversation.unread)")
                                    .font(.caption2.weight(.bold))
                                    .foregroundStyle(Color.weaveCuivreMontre)
                            }
                        }
                        Text(conversation.planTitle)
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                    }
                }
            }
        }
        .navigationTitle("Conversations")
        .task { await conversations.refresh() }
    }
}

/// Une conversation : les derniers messages, et une réponse.
///
/// `TextField` sur la montre ouvre la saisie du système — dictée, écriture
/// manuscrite, clavier. C'est exactement « dicter deux phrases avant de
/// repartir » : rien à construire de plus, et rien qui marcherait mieux.
private struct ConversationMontreView: View {
    let conversation: Conversation

    @Environment(ModeleMontre.self) private var modele
    @State private var reponse = ""
    @State private var envoi = false
    @State private var erreur: String?

    private var conversations: ConversationsStore { modele.magasin.conversations }

    /// Les derniers seulement : un fil entier ne se lit pas à cette taille.
    private var derniers: [Message] {
        Array((conversations.messages[conversation.id] ?? []).suffix(6))
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 8) {
                Text(conversation.planTitle)
                    .font(.caption2)
                    .foregroundStyle(Color.weaveCuivreMontre)

                ForEach(derniers) { message in
                    Text(message.body)
                        .font(.caption)
                        .padding(8)
                        .frame(
                            maxWidth: .infinity,
                            alignment: message.author == .moi ? .trailing : .leading
                        )
                        .background(
                            message.author == .moi
                                ? Color.weaveCuivreMontre.opacity(0.25)
                                : Color.gray.opacity(0.2),
                            in: .rect(cornerRadius: 10)
                        )
                        .privacySensitive()
                }

                if let erreur {
                    Text(erreur)
                        .font(.caption2)
                        .foregroundStyle(.red)
                }

                TextField("Répondre", text: $reponse)
                    .onSubmit { Task { await envoyer() } }
                    .disabled(envoi)
            }
        }
        .navigationTitle(conversation.other.displayName)
        .task { await conversations.load(conversation.id) }
    }

    private func envoyer() async {
        envoi = true
        defer { envoi = false }
        erreur = nil
        if await conversations.send(reponse, in: conversation.id) {
            reponse = ""
        } else if let souci = conversations.alert {
            // Le texte dicté reste là : le redicter serait la pire des
            // réponses à un refus.
            erreur = souci.userMessage
            conversations.alert = nil
        }
    }
}
