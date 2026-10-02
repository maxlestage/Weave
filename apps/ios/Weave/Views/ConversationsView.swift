import SwiftUI
import WeaveKit

/// Les conversations ouvertes, et les demandes qu'on a envoyées.
///
/// Une conversation naît d'une acceptation, jamais autrement : il n'existe
/// aucun moyen d'écrire à quelqu'un qui n'a pas dit oui.
struct ConversationsView: View {
    @Environment(ModeleApplication.self) private var modele
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
            if modele.conversations.conversations.isEmpty {
                ContentUnavailableView(
                    "Aucune conversation",
                    systemImage: "bubble.left.and.bubble.right",
                    description: Text("Une conversation s'ouvre quand quelqu'un accepte votre demande, ou que vous acceptez la sienne.")
                )
            } else {
                List(modele.conversations.conversations) { conversation in
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
        await modele.conversations.refresh()
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
            if conversation.closed {
                Text("Fermée")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            } else if let dernier = conversation.lastMessage {
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
    @State private var confirmeDesistement = false

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
            // Une place accordée se rend — jusqu'à l'heure du rendez-vous.
            //
            // Rien ne le permettait : une fois accepté, on ne pouvait plus
            // reculer. La place restait prise pour quelqu'un qui ne viendrait
            // pas, l'autre attendait au café sans rien savoir, et la seule
            // sortie était de ne pas venir.
            if demande.state == .acceptee && demande.planStartsAt > .now {
                Button("Je ne peux pas venir", role: .destructive) {
                    confirmeDesistement = true
                }
            }
        }
        .confirmationDialog(
            "Rendre votre place ?",
            isPresented: $confirmeDesistement,
            titleVisibility: .visible
        ) {
            Button("Rendre ma place", role: .destructive) {
                Task { await modele.plans.release(demande) }
            }
            Button("Annuler", role: .cancel) {}
        } message: {
            // Les deux conséquences, dites avant plutôt qu'après. La seconde
            // surtout : se désister n'est pas gratuit, et l'apprendre une fois
            // le geste fait serait déloyal.
            Text(
                "La place repart au fil, et la personne qui vous attendait est prévenue. "
                    + "Votre demande du jour reste dépensée. "
                    + "La conversation, elle, reste ouverte : vous pouvez y dire un mot."
            )
        }
    }
}

/// Une conversation, toujours à deux — même sur un plan de groupe : on parle à
/// quelqu'un, pas à une salle.
struct ConversationView: View {
    let conversation: Conversation

    @Environment(ModeleApplication.self) private var modele
    @Environment(\.dismiss) private var dismiss
    @State private var brouillon = ""
    @State private var envoi = false
    @State private var confirmeFermeture = false
    @State private var erreur: String?

    private var messages: [Message] { modele.conversations.messages[conversation.id] ?? [] }

    /// L'état le plus frais : la liste a pu être relue depuis l'ouverture,
    /// notamment juste après une fermeture.
    private var fermee: Bool {
        modele.conversations.conversations.first { $0.id == conversation.id }?.closed
            ?? conversation.closed
    }

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

            if let erreur {
                Text(erreur)
                    .font(.footnote)
                    .foregroundStyle(.red)
                    .padding(.horizontal, 16)
                    .padding(.top, 6)
            }

            if fermee {
                // Rien ne disait qu'une conversation était close : le champ de
                // saisie disparaissait, et c'était tout.
                Text("Conversation fermée. Ses messages seront effacés dans \(messageRetentionDays) jours.")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                    .padding(12)
                    .frame(maxWidth: .infinity)
                    .background(.bar)
            } else {
                HStack(spacing: 10) {
                    TextField("Écrire…", text: $brouillon, axis: .vertical)
                        .lineLimit(1...4)
                        .textFieldStyle(.roundedBorder)
                        // Le serveur refuse au-delà. Sans cette borne ici, on
                        // écrivait sans fin et l'on perdait son texte à
                        // l'envoi, pour une règle que rien n'annonçait.
                        .onChange(of: brouillon) { _, texte in
                            if texte.count > conversationMaxChars {
                                brouillon = String(texte.prefix(conversationMaxChars))
                            }
                        }
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
        .toolbar {
            // Signaler et bloquer se trouvent là où l'on parle : c'est ici que
            // se passe ce qui se signale, et c'est ici qu'il faut pouvoir y
            // couper court sans chercher.
            ToolbarItem(placement: .topBarTrailing) {
                MenuDeProtection(
                    compteID: conversation.other.id,
                    prenom: conversation.other.displayName
                ) {
                    // Le lien est coupé : la conversation n'existe plus pour
                    // nous, et rester dessus n'aurait pas de sens.
                    dismiss()
                }
            }
            // Le site le promet : « elle se ferme quand vous voulez ». La
            // route existait ; aucun bouton n'y menait.
            if !fermee {
                ToolbarItem(placement: .topBarLeading) {
                    Button("Fermer", role: .destructive) { confirmeFermeture = true }
                }
            }
        }
        .confirmationDialog(
            "Fermer cette conversation ?",
            isPresented: $confirmeFermeture,
            titleVisibility: .visible
        ) {
            Button("Fermer la conversation", role: .destructive) {
                Task { await fermer() }
            }
            Button("Annuler", role: .cancel) {}
        } message: {
            // Les conséquences avant le geste : on ne rouvre pas, et les
            // messages partent pour de bon après le délai.
            Text("Ni vous ni l'autre personne ne pourrez plus y écrire. Les messages restent lisibles \(messageRetentionDays) jours, puis sont effacés.")
        }
        .task { await modele.conversations.load(conversation.id) }
    }

    private func envoyer() async {
        envoi = true
        defer { envoi = false }
        erreur = nil
        if await modele.conversations.send(brouillon, in: conversation.id) {
            brouillon = ""
        } else if let souci = modele.conversations.alert {
            // Le texte reste dans le champ : un refus ne doit pas coûter ce
            // qu'on a écrit.
            erreur = souci.userMessage
            modele.conversations.alert = nil
        }
    }

    private func fermer() async {
        erreur = nil
        if !(await modele.conversations.close(conversation.id)),
           let souci = modele.conversations.alert
        {
            erreur = souci.userMessage
            modele.conversations.alert = nil
        }
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
