import SwiftUI
import WeaveKit

/// Les conversations ouvertes, et les demandes qu'on a envoyées.
///
/// Une conversation naît d'une acceptation, jamais autrement : il n'existe
/// aucun moyen d'écrire à quelqu'un qui n'a pas dit oui.
struct ConversationsView: View {
    @Environment(ModeleApplication.self) private var modele
    @State private var conversations: [Conversation] = []
    @State private var onglet = Onglet.conversations

    private enum Onglet: String, CaseIterable {
        case conversations = "Conversations"
        case envoyees = "Mes demandes"
    }

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                Picker("Affichage", selection: $onglet) {
                    ForEach(Onglet.allCases, id: \.self) { Text($0.rawValue).tag($0) }
                }
                .pickerStyle(.segmented)
                .padding(.horizontal, 16)
                .padding(.bottom, 8)

                switch onglet {
                case .conversations: listeConversations
                case .envoyees: listeEnvoyees
                }
            }
            .background(Color.weaveLin.ignoresSafeArea())
            .navigationTitle("Suites")
            .task { await recharger() }
            .refreshable { await recharger() }
        }
    }

    private var listeConversations: some View {
        Group {
            if conversations.isEmpty {
                ContentUnavailableView(
                    "Aucune conversation",
                    systemImage: "bubble.left.and.bubble.right",
                    description: Text("Une conversation s'ouvre quand quelqu'un accepte votre demande, ou que vous acceptez la sienne.")
                )
            } else {
                List(conversations) { conversation in
                    NavigationLink {
                        ConversationView(conversation: conversation).environment(modele)
                    } label: {
                        ConversationLigne(conversation: conversation)
                    }
                }
                .listStyle(.plain)
            }
        }
    }

    private var listeEnvoyees: some View {
        Group {
            if modele.plans.sent.isEmpty {
                ContentUnavailableView(
                    "Aucune demande envoyée",
                    systemImage: "paperplane",
                    description: Text("Vous en avez \(modele.plans.requestsLeftToday) en réserve aujourd'hui.")
                )
            } else {
                List(modele.plans.sent) { demande in
                    DemandeEnvoyeeLigne(demande: demande)
                }
                .listStyle(.plain)
            }
        }
    }

    private func recharger() async {
        await modele.plans.refreshSent()
        conversations = (try? await modele.api.conversations()) ?? []
    }
}

private struct ConversationLigne: View {
    let conversation: Conversation

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(conversation.other.displayName)
                    .font(.headline)
                Spacer()
                if conversation.unread > 0 {
                    Text("\(conversation.unread)")
                        .font(.caption.weight(.bold))
                        .padding(.horizontal, 7)
                        .padding(.vertical, 2)
                        .background(Color.weaveCuivre, in: .capsule)
                        .foregroundStyle(.white)
                }
            }
            Text(conversation.planTitle)
                .font(.subheadline)
                .foregroundStyle(Color.weaveCuivre)
            if let dernier = conversation.lastMessage {
                Text(dernier)
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
        }
        .padding(.vertical, 4)
    }
}

private struct DemandeEnvoyeeLigne: View {
    @Environment(ModeleApplication.self) private var modele
    let demande: JoinRequest

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(demande.planTitle).font(.headline)
                Spacer()
                Text(demande.state.displayName)
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(demande.state == .acceptee ? Color.weaveCuivre : .secondary)
            }
            Text(demande.planStartsAt.weaveWhenLabel)
                .font(.subheadline)
                .foregroundStyle(.secondary)
            Text(demande.message)
                .font(.footnote)
                .foregroundStyle(.secondary)
                .lineLimit(2)
        }
        .padding(.vertical, 4)
        .swipeActions {
            if demande.state == .envoyee {
                Button("Retirer") {
                    Task { await modele.plans.withdraw(demande) }
                }
            }
        }
    }
}

/// Une conversation, toujours à deux — même sur un plan de groupe : on parle à
/// quelqu'un, pas à une salle.
struct ConversationView: View {
    let conversation: Conversation

    @Environment(ModeleApplication.self) private var modele
    @State private var messages: [Message] = []
    @State private var brouillon = ""
    @State private var envoi = false

    var body: some View {
        VStack(spacing: 0) {
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(spacing: 10) {
                        EnTetePlan(conversation: conversation)

                        ForEach(messages) { message in
                            Bulle(message: message).id(message.id)
                        }
                    }
                    .padding(16)
                }
                .onChange(of: messages.count) { _, _ in
                    withAnimation { proxy.scrollTo(messages.last?.id, anchor: .bottom) }
                }
            }

            if !conversation.closed {
                HStack(spacing: 10) {
                    TextField("Écrire…", text: $brouillon, axis: .vertical)
                        .lineLimit(1...4)
                        .textFieldStyle(.roundedBorder)
                    Button {
                        Task { await envoyer() }
                    } label: {
                        Image(systemName: "arrow.up.circle.fill").font(.title2)
                    }
                    .disabled(brouillon.trimmingCharacters(in: .whitespaces).isEmpty || envoi)
                }
                .padding(12)
                .background(.bar)
            }
        }
        .background(Color.weaveLin.ignoresSafeArea())
        .navigationTitle(conversation.other.displayName)
        .navigationBarTitleDisplayMode(.inline)
        .task { await charger() }
    }

    private func charger() async {
        messages = (try? await modele.api.messages(conversationID: conversation.id)) ?? []
    }

    private func envoyer() async {
        let texte = brouillon.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !texte.isEmpty else { return }
        envoi = true
        if let message = try? await modele.api.send(conversationID: conversation.id, body: texte) {
            messages.append(message)
            brouillon = ""
        }
        envoi = false
    }
}

private struct EnTetePlan: View {
    let conversation: Conversation

    var body: some View {
        VStack(spacing: 4) {
            Text(conversation.planTitle)
                .font(.subheadline.weight(.semibold))
            Text(conversation.planStartsAt.weaveWhenLabel)
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(.vertical, 10)
        .frame(maxWidth: .infinity)
        .background(Color.white, in: .rect(cornerRadius: 14))
    }
}

private struct Bulle: View {
    let message: Message

    var body: some View {
        HStack {
            if message.author == .moi { Spacer(minLength: 48) }

            Text(message.body)
                .padding(.horizontal, 14)
                .padding(.vertical, 10)
                .background(
                    message.author == .moi ? Color.weaveCuivre : Color.white,
                    in: .rect(cornerRadius: 16)
                )
                .foregroundStyle(message.author == .moi ? .white : .primary)

            if message.author != .moi { Spacer(minLength: 48) }
        }
    }
}
