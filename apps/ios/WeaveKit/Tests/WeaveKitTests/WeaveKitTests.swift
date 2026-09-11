import Foundation
import Testing
@testable import WeaveKit

// MARK: - Invariant des trois fils

@Suite("Le métier")
struct LoomTests {
    @Test("Le plafond de trois est appliqué même si l'API en renvoie plus")
    func plafond() {
        let loom = Loom(
            threads: (0..<7).map { fil(id: "\($0)") },
            nextWeavingAt: .now,
            freeSlots: 0,
            nextRefillAt: nil,
            fromCache: true
        )
        #expect(loom.threads.count == Loom.maxActiveThreads)
    }

    @Test("Le fil le plus proche du dénouage est celui mis en avant")
    func plusPresse() {
        let loom = Loom(
            threads: [
                fil(id: "a", expire: .now.addingTimeInterval(3600)),
                fil(id: "b", expire: .now.addingTimeInterval(600)),
                fil(id: "c", expire: .now.addingTimeInterval(7200)),
            ],
            nextWeavingAt: .now,
            freeSlots: 0,
            nextRefillAt: nil,
            fromCache: true
        )
        #expect(loom.soonest?.id == "b")
    }

    @Test("Un fil échu est signalé comme tel")
    func echu() {
        #expect(fil(id: "a", expire: .now.addingTimeInterval(-1)).isExpired)
        #expect(!fil(id: "a", expire: .now.addingTimeInterval(60)).isExpired)
    }

    @Test("Le flou local suit le niveau de révélation", arguments: [
        (0, 18.0), (33, 12.06), (66, 6.12), (100, 0.0),
    ])
    func flou(pourcentage: Int, attendu: Double) {
        let valeur = fil(id: "a", reveal: pourcentage).blurRadius
        #expect(abs(valeur - attendu) < 0.01)
    }
}

// MARK: - Décodage

@Suite("Décodage des charges utiles")
struct DecodingTests {
    @Test("Un fil renvoyé par l'API se décode entièrement")
    func filComplet() throws {
        let json = """
        {
          "id": "abc",
          "state": "propose",
          "displayName": "Théo",
          "age": 31,
          "distanceKm": 6,
          "city": "Paris",
          "motif": ["photo", "voile"],
          "fragments": [
            { "id": "f1", "kind": "question", "prompt": "Un dimanche ?", "body": "Tôt." }
          ],
          "revealPercent": 0,
          "photoUrl": "https://media.weave.app/x.jpg?sig=abc",
          "expiresAt": "2026-09-11T21:35:25.942Z",
          "exchanges": 0,
          "awaitingYou": true
        }
        """
        let carte = try decodeur.decode(ThreadCard.self, from: Data(json.utf8))
        #expect(carte.displayName == "Théo")
        #expect(carte.state == .propose)
        #expect(carte.fragments.first?.kind == .question)
        #expect(carte.photoURL?.host() == "media.weave.app")
    }

    @Test("Les dates avec et sans millisecondes sont acceptées")
    func dates() throws {
        for texte in ["2026-09-11T21:35:25.942Z", "2026-09-11T21:35:25Z"] {
            let json = #"{"activeThreads":1,"awaitingYou":0,"soonestExpiryAt":"\#(texte)","soonestName":"A","nextRefillAt":null,"updatedAt":"\#(texte)"}"#
            let etat = try decodeur.decode(
                WeaveActivityAttributes.ContentState.self,
                from: Data(json.utf8)
            )
            #expect(etat.soonestExpiryAt != nil)
        }
    }
}

// MARK: - État de la Live Activity

@Suite("Live Activity")
struct ActivityStateTests {
    @Test("Le résumé s'adapte au nombre de fils en attente")
    func resume() {
        #expect(etat(total: 0, attente: 0).summary == "Aucun fil sur le métier")
        #expect(etat(total: 3, attente: 0).summary == "3 fils en cours")
        #expect(etat(total: 3, attente: 1).summary == "1 fil attend votre réponse")
        #expect(etat(total: 3, attente: 2).summary == "2 fils attendent votre réponse")
    }

    @Test("Le décompte n'est proposé que pour une échéance à venir")
    func decompte() {
        #expect(etat(total: 1, attente: 1, expire: .now.addingTimeInterval(600)).countdown != nil)
        #expect(etat(total: 1, attente: 1, expire: .now.addingTimeInterval(-600)).countdown == nil)
        #expect(etat(total: 0, attente: 0).countdown == nil)
    }

    private func etat(
        total: Int,
        attente: Int,
        expire: Date? = nil
    ) -> WeaveActivityAttributes.ContentState {
        WeaveActivityAttributes.ContentState(
            activeThreads: total,
            awaitingYou: attente,
            soonestExpiryAt: expire,
            soonestName: nil,
            nextRefillAt: nil
        )
    }
}

// MARK: - Session

@Suite("Session")
struct SessionTests {
    @Test("Une session proche de l'expiration demande un renouvellement")
    func renouvellement() {
        let bientot = Session(accessToken: "a", refreshToken: "r", expiresAt: .now.addingTimeInterval(30))
        let large = Session(accessToken: "a", refreshToken: "r", expiresAt: .now.addingTimeInterval(600))
        #expect(bientot.needsRefresh)
        #expect(!large.needsRefresh)
    }

    @Test("Chaque produit à l'unité porte un identifiant StoreKit distinct")
    func identifiants() {
        let identifiants = Set(UnitSku.allCases.map(\.productID))
        #expect(identifiants.count == UnitSku.allCases.count)
        #expect(UnitSku.echo.productID == "com.weave.app.unit.echo")
    }
}

// MARK: - Fabriques

private let decodeur: JSONDecoder = {
    let decodeur = JSONDecoder()
    decodeur.dateDecodingStrategy = .custom { decoder in
        let texte = try decoder.singleValueContainer().decode(String.self)
        if let date = ISO8601DateFormatter.weaveWithFraction.date(from: texte) { return date }
        if let date = ISO8601DateFormatter.weave.date(from: texte) { return date }
        throw DecodingError.dataCorrupted(
            .init(codingPath: decoder.codingPath, debugDescription: "Date illisible")
        )
    }
    return decodeur
}()

private func fil(
    id: String,
    expire: Date = .now.addingTimeInterval(3600),
    reveal: Int = 0
) -> ThreadCard {
    let json = """
    {
      "id": "\(id)",
      "state": "propose",
      "displayName": "Fil \(id)",
      "age": 30,
      "distanceKm": 5,
      "city": "Paris",
      "motif": [],
      "fragments": [],
      "revealPercent": \(reveal),
      "photoUrl": null,
      "expiresAt": "\(ISO8601DateFormatter.weaveWithFraction.string(from: expire))",
      "exchanges": 0,
      "awaitingYou": true
    }
    """
    return try! decodeur.decode(ThreadCard.self, from: Data(json.utf8))
}
