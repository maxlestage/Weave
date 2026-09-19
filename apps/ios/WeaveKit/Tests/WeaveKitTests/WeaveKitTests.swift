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

    @Test("Les dates avec et sans millisecondes sont acceptées, millisecondes comprises")
    func dates() throws {
        func lire(_ texte: String) throws -> Date? {
            let json = #"{"planTitle":"Brunch","planStartsAt":"\#(texte)","pendingRequests":1,"awaitingReply":0,"updatedAt":"\#(texte)"}"#
            return try decodeur.decode(
                WeaveActivityAttributes.ContentState.self,
                from: Data(json.utf8)
            ).planStartsAt
        }

        let avec = try #require(try lire("2026-09-11T21:35:25.942Z"))
        let sans = try #require(try lire("2026-09-11T21:35:25Z"))

        // Les deux formes passent — mais pas au même instant. Se contenter de
        // « ce n'est pas nul » laissait passer un décodeur qui tronque à la
        // seconde : le décompte d'une Live Activity se serait décalé sans que
        // rien ne le dise.
        #expect(abs(avec.timeIntervalSince(sans) - 0.942) < 0.001)
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

// MARK: - Ce qui quitte l'appareil

@Suite("La fiche qui part")
struct ProfileBodyTests {
    /// Le pas, ÉCRIT ICI et non lu de ce qu'on éprouve.
    ///
    /// Ma première version disait `WeaveAPI.pasDeLaGrillePosition` : porter la
    /// grille à 0,0001° — onze mètres au lieu d'un kilomètre — montait donc
    /// l'attente avec elle, et le test restait au vert. C'est la même faute
    /// qu'un test de longueur de message plus tôt dans ce dépôt : un test qui
    /// lit ce qu'il garde ne garde rien.
    ///
    /// L'accord entre ce nombre et celui du serveur est tenu ailleurs, par un
    /// test de contrat qui lit les deux sources.
    private static let pas = 0.01

    @Test("La position est arrondie avant de quitter l'appareil")
    func positionArrondie() {
        // Paris, à la précision d'un GPS : sept décimales, soit le centimètre.
        let corps = WeaveAPI.corpsDeFiche(
            city: "Paris",
            latitude: 48.8584312,
            longitude: 2.2944813,
            gender: .homme,
            bio: nil
        )

        // La promesse n'est pas « moins précis », elle est « sur la grille ».
        // On vérifie donc que le point TOMBE sur un nœud : diviser par le pas
        // doit donner un entier. Comparer à une valeur écrite en dur laisserait
        // passer un arrondi sur une grille plus fine.
        for coordonnee in [corps.latitude, corps.longitude] {
            let crans = coordonnee / Self.pas
            #expect(
                abs(crans - crans.rounded()) < 1e-6,
                "\(coordonnee) ne tombe pas sur la grille de \(Self.pas)°"
            )
        }

        // Au plus proche, et non « quelque part sur la grille » : le point ne
        // bouge que d'un DEMI-cran au plus. Tronquer vers zéro tombe aussi sur
        // la grille — ma première version l'acceptait donc — mais déplace
        // jusqu'à un cran entier. Ici 48,8584 monte à 48,86 ; tronquer
        // donnerait 48,85, soit 0,0084° de trop.
        #expect(abs(corps.latitude - 48.8584312) <= Self.pas / 2)
        #expect(abs(corps.longitude - 2.2944813) <= Self.pas / 2)
    }

    @Test("Une position déjà sur la grille ne bouge pas")
    func positionStable() {
        // Le serveur ré-arrondit sur la même grille. S'il déplaçait un point
        // déjà posé dessus, l'arrondi fait ici ne servirait à rien : deux
        // arrondis identiques doivent se composer sans rien bouger.
        let corps = WeaveAPI.corpsDeFiche(
            city: "Lyon", latitude: 45.76, longitude: 4.83, gender: .femme, bio: nil
        )
        let deuxFois = WeaveAPI.corpsDeFiche(
            city: "Lyon",
            latitude: corps.latitude,
            longitude: corps.longitude,
            gender: .femme,
            bio: nil
        )
        #expect(corps.latitude == deuxFois.latitude)
        #expect(corps.longitude == deuxFois.longitude)
    }

    @Test("L'hémisphère sud et l'ouest s'arrondissent aussi")
    func positionNegative() {
        // `Int(x)` tronque vers zéro : une grille construite ainsi décalerait
        // tout l'hémisphère sud d'un demi-cran dans le mauvais sens.
        let corps = WeaveAPI.corpsDeFiche(
            city: "Montevideo",
            latitude: -34.9011237,
            longitude: -56.1645314,
            gender: .autre,
            bio: nil
        )
        for coordonnee in [corps.latitude, corps.longitude] {
            let crans = coordonnee / Self.pas
            #expect(abs(crans - crans.rounded()) < 1e-6)
        }
        // Le demi-cran, ici aussi : c'est au sud que tronquer vers zéro se
        // voit le mieux, puisqu'il y remonte vers l'équateur.
        #expect(abs(corps.latitude - -34.9011237) <= Self.pas / 2)
        #expect(abs(corps.longitude - -56.1645314) <= Self.pas / 2)
    }

    @Test("La ville et la présentation partent sans leurs espaces")
    func blancsRetires() {
        let corps = WeaveAPI.corpsDeFiche(
            city: "  Lille  ", latitude: 50.63, longitude: 3.06, gender: .homme, bio: "  bonjour  "
        )
        #expect(corps.city == "Lille")
        #expect(corps.bio == "bonjour")

        // Une présentation blanche vaut absente : envoyer « " " » poserait une
        // bio faite d'un espace, que rien ensuite ne distingue d'un choix.
        let vide = WeaveAPI.corpsDeFiche(
            city: "Lille", latitude: 50.63, longitude: 3.06, gender: .homme, bio: "   "
        )
        #expect(vide.bio == nil)
    }
}

// MARK: - Fabriques

/// LE décodeur du client, et non une copie.
///
/// Cette fabrique en construisait un second, à l'identique. Les tests
/// éprouvaient donc leur propre copie : remettre `.iso8601` dans celui du
/// client — le défaut que son commentaire raconte, celui qui faisait tomber
/// chaque session au bout du quart d'heure — les laissait tous au vert.
private let decodeur = WeaveAPI.decoder

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
