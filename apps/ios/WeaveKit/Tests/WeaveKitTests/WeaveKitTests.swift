import Foundation
import Testing
@testable import WeaveKit

// MARK: - Le fil

@Suite("Le fil")
struct FeedTests {
    @Test("L'ordre reçu du serveur est conservé tel quel")
    func ordreIntact() {
        // Imminence puis proximité, décidées par le serveur. Un tri local, même
        // « utile », introduirait un classement que personne n'a annoncé.
        let plans = [
            plan(id: "a", debut: .now.addingTimeInterval(7200), distance: 30),
            plan(id: "b", debut: .now.addingTimeInterval(3600), distance: 2),
            plan(id: "c", debut: .now.addingTimeInterval(10800), distance: 1),
        ]
        let fil = Feed(plans: plans, requestsLeftToday: 5, fromCache: false, generatedAt: .now)
        #expect(fil.plans.map(\.id) == ["a", "b", "c"])
        #expect(fil.soonest?.id == "a")
    }

    @Test("Un plan complet, déjà demandé ou passé ne se rejoint plus")
    func rejoignable() {
        #expect(plan(id: "a").isJoinable)
        #expect(!plan(id: "a", places: 0).isJoinable)
        #expect(!plan(id: "a", demande: true).isJoinable)
        #expect(!plan(id: "a", debut: .now.addingTimeInterval(-60)).isJoinable)
    }
}

// MARK: - Décodage

@Suite("Décodage des charges utiles")
struct DecodingTests {
    @Test("Un plan renvoyé par l'API se décode entièrement")
    func planComplet() throws {
        let json = """
        {
          "id": "abc",
          "author": {
            "id": "u1",
            "displayName": "Théo",
            "age": 23,
            "photoUrl": "https://media.weave.app/x.jpg?sig=abc",
            "verified": true
          },
          "title": "Bloc au mur de 19 h, niveau débutant",
          "note": "Je grimpe depuis six mois, très mal.",
          "category": "sport",
          "startsAt": "2026-09-11T19:00:00.000Z",
          "city": "Paris",
          "distanceKm": 2,
          "capacity": 1,
          "seatsLeft": 1,
          "state": "ouvert",
          "requested": false,
          "createdAt": "2026-09-10T08:12:00.000Z"
        }
        """
        let plan = try decodeur.decode(Plan.self, from: Data(json.utf8))
        #expect(plan.author.displayName == "Théo")
        #expect(plan.category == .sport)
        #expect(plan.state == .ouvert)
        #expect(plan.author.photoURL?.host() == "media.weave.app")
    }

    @Test("Les dates avec et sans millisecondes sont acceptées")
    func dates() throws {
        for texte in ["2026-09-11T21:35:25.942Z", "2026-09-11T21:35:25Z"] {
            let json = #"{"planTitle":"Brunch","planStartsAt":"\#(texte)","pendingRequests":1,"awaitingReply":0,"updatedAt":"\#(texte)"}"#
            let etat = try decodeur.decode(
                WeaveActivityAttributes.ContentState.self,
                from: Data(json.utf8)
            )
            #expect(etat.planStartsAt != nil)
        }
    }

    @Test("Toutes les catégories du serveur sont connues du client")
    func categories() throws {
        // Si le serveur en ajoute une, ce test tombe avant que l'application
        // n'échoue silencieusement à décoder un fil entier.
        let attendues = [
            "sortie", "sport", "culture", "repas",
            "musique", "jeux", "balade", "benevolat",
        ]
        #expect(Set(PlanCategory.allCases.map(\.rawValue)) == Set(attendues))
    }
}

// MARK: - État de la Live Activity

@Suite("Live Activity")
struct ActivityStateTests {
    @Test("Les demandes reçues passent devant le rendez-vous")
    func priorite() {
        // C'est la seule chose qui attend une action de sa part.
        #expect(etat(titre: "Brunch", recues: 2).summary == "2 personnes veulent venir")
        #expect(etat(titre: "Brunch", recues: 1).summary == "Quelqu'un veut venir")
        #expect(etat(titre: "Brunch").summary == "Brunch")
        #expect(etat(envoyees: 2).summary == "2 demandes en attente")
        #expect(etat().summary == "Aucun plan à venir")
    }

    @Test("Un état sans rien à montrer est inactif")
    func inactif() {
        #expect(etat().isIdle)
        #expect(WeaveActivityAttributes.ContentState.idle.isIdle)
        #expect(!etat(titre: "Brunch").isIdle)
        #expect(!etat(recues: 1).isIdle)
    }

    @Test("Le décompte n'est proposé que pour un rendez-vous à venir")
    func decompte() {
        #expect(etat(titre: "A", debut: .now.addingTimeInterval(600)).countdown != nil)
        #expect(etat(titre: "A", debut: .now.addingTimeInterval(-600)).countdown == nil)
        #expect(etat().countdown == nil)
    }

    private func etat(
        titre: String? = nil,
        debut: Date? = nil,
        recues: Int = 0,
        envoyees: Int = 0
    ) -> WeaveActivityAttributes.ContentState {
        WeaveActivityAttributes.ContentState(
            planTitle: titre,
            planStartsAt: debut,
            pendingRequests: recues,
            awaitingReply: envoyees
        )
    }
}

// MARK: - Session et catalogue

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
        #expect(UnitSku.renfort.productID == "com.weave.app.unit.renfort")
    }

    @Test("Aucun produit ne vend de remontée dans le fil")
    func pasDeVisibilite() {
        // L'invariant tient d'abord côté serveur ; ce test empêche qu'un tel
        // produit apparaisse ici sans qu'on s'en aperçoive.
        let interdits = ["boost", "remontee", "relance", "mise_en_avant", "spotlight"]
        for sku in UnitSku.allCases {
            #expect(!interdits.contains(sku.rawValue))
        }
    }
}

// MARK: - Fabriques

private let decodeur: JSONDecoder = {
    let decodeur = JSONDecoder()
    decodeur.dateDecodingStrategy = .custom { decoder in
        let texte = try decoder.singleValueContainer().decode(String.self)
        if let date = DateWeave.lire(texte) { return date }
        throw DecodingError.dataCorrupted(
            .init(codingPath: decoder.codingPath, debugDescription: "Date illisible")
        )
    }
    return decodeur
}()

private func plan(
    id: String,
    debut: Date = .now.addingTimeInterval(3600),
    distance: Int = 5,
    places: Int = 1,
    demande: Bool = false
) -> Plan {
    let json = """
    {
      "id": "\(id)",
      "author": {
        "id": "u-\(id)",
        "displayName": "Auteur \(id)",
        "age": 22,
        "photoUrl": null,
        "verified": false
      },
      "title": "Un plan de test",
      "note": "",
      "category": "sortie",
      "startsAt": "\(DateWeave.avecFractions.format(debut))",
      "city": "Paris",
      "distanceKm": \(distance),
      "capacity": 1,
      "seatsLeft": \(places),
      "state": "ouvert",
      "requested": \(demande),
      "createdAt": "\(DateWeave.avecFractions.format(.now))"
    }
    """
    return try! decodeur.decode(Plan.self, from: Data(json.utf8))
}
