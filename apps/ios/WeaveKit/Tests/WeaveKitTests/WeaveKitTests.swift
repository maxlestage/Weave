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

// MARK: - Retouches locales du fil

@Suite("Retouches locales du fil")
struct FeedEditTests {
    private func fil(_ plans: [Plan], restantes: Int = 5) -> Feed {
        Feed(
            plans: plans,
            requestsLeftToday: restantes,
            fromCache: false,
            generatedAt: Date(timeIntervalSince1970: 1_789_000_000)
        )
    }

    @Test("Marquer un plan comme demandé ne touche à rien d'autre")
    func demandeNeChangeQueLaDemande() {
        // Le magasin rebâtissait le plan champ par champ : treize arguments
        // recopiés à la main. Le compilateur exige qu'ils soient tous là, mais
        // rien n'empêche d'en INTERVERTIR deux de même type — `capacity` et
        // `seatsLeft` sont deux entiers, `title`, `note` et `city` trois
        // chaînes. Un plan complet se serait affiché avec des places libres.
        //
        // `Plan` est `Hashable` : comparer les deux plans compare TOUS les
        // champs, y compris ceux qu'on ajouterait demain.
        // Capacité et places libres DIFFÉRENTES, sans quoi les intervertir ne
        // se verrait pas — c'est ce que ma première version faisait, et la
        // mutation est passée sans rien faire tomber.
        let avant = plan(id: "a", distance: 12, places: 2, capacite: 5)
        let attendu = Plan(
            id: avant.id,
            author: avant.author,
            title: avant.title,
            note: avant.note,
            category: avant.category,
            startsAt: avant.startsAt,
            city: avant.city,
            distanceKm: avant.distanceKm,
            capacity: avant.capacity,
            seatsLeft: avant.seatsLeft,
            state: avant.state,
            requested: true,
            createdAt: avant.createdAt
        )
        #expect(avant.requested == false)
        #expect(avant.demande() == attendu)
    }

    @Test("Un fil retouché le dit, et garde son heure de composition")
    func retoucheHonnete() {
        // `generatedAt` est l'heure à laquelle le SERVEUR a composé le fil. La
        // toucher ici ferait croire à une composition qui n'a pas eu lieu, et
        // c'est cette heure que l'écran affiche.
        let origine = fil([plan(id: "a"), plan(id: "b")])
        let retouche = origine.remplacant(plans: [origine.plans[0]])

        #expect(retouche.generatedAt == origine.generatedAt)
        #expect(retouche.plans.map(\.id) == ["a"])
        // Ce qu'on tient ne vient plus du serveur : il a été retouché ici.
        #expect(origine.fromCache == false)
        #expect(retouche.fromCache == true)
        // Le compteur de demandes ne bouge pas quand on ne le donne pas.
        #expect(retouche.requestsLeftToday == origine.requestsLeftToday)
        // Et il bouge quand on le donne : c'est ce que fait une demande.
        #expect(origine.remplacant(plans: [], requestsLeftToday: 2).requestsLeftToday == 2)
    }
}

// MARK: - États d'une demande

@Suite("États d'une demande")
struct RequestStateTests {
    @Test("Tous les états du serveur sont connus du client")
    func etatsConnus() {
        // Un état écrit par le serveur et absent d'ici ne casse pas une ligne :
        // il casse la RÉPONSE ENTIÈRE, puisque Swift échoue à décoder une
        // énumération sans cas correspondant. « Mes demandes » tomberait d'un
        // bloc — c'est déjà arrivé sur l'état d'un plan.
        //
        // Écrits ici, et non lus du type qu'on éprouve : un test qui lit ce
        // qu'il garde ne garde rien. L'accord avec le contrat partagé est tenu
        // par un test de contrat qui lit les deux sources.
        let attendus = [
            "envoyee", "acceptee", "refusee", "expiree", "retiree", "desistee",
        ]
        #expect(Set(RequestState.allCases.map(\.rawValue)) == Set(attendus))
    }

    @Test("Chaque état porte un libellé distinct, écrit pour être lu")
    func libellesDistincts() {
        for etat in RequestState.allCases {
            #expect(etat.displayName != etat.rawValue, "« \(etat.rawValue) » s'affiche tel quel")
            #expect(etat.displayName.first?.isUppercase == true)
        }
        // Deux états qui s'affichent pareil rendent l'écran illisible : « close »
        // et « place rendue » ne veulent pas dire la même chose à qui les lit.
        #expect(
            Set(RequestState.allCases.map(\.displayName)).count == RequestState.allCases.count
        )
    }

    @Test("Une demande dont la place a été rendue se décode")
    func desisteeSeDecode() throws {
        // Le décodage entier, et non le seul état : c'est la liste complète que
        // l'écran perdrait.
        let json = #"""
        {"id":"r1","planId":"p1","planTitle":"Un cafe","planStartsAt":"2026-10-01T18:00:00.000Z",
        "author":{"id":"u1","displayName":"Ana","age":30,"photoUrl":null,"verified":false},
        "message":"Je serais venu avec plaisir mais je ne peux pas.",
        "state":"desistee","sentAt":"2026-09-20T10:00:00.000Z",
        "decidedAt":"2026-09-20T11:00:00.000Z","conversationId":"c1"}
        """#
        let demande = try decodeur.decode(JoinRequest.self, from: Data(json.utf8))
        #expect(demande.state == .desistee)
        #expect(demande.state.displayName == "Place rendue")
    }
}

// MARK: - Consentement

@Suite("Consentement")
struct ConsentTests {
    private func lire(_ json: String) throws -> Consentements {
        try decodeur.decode(Consentements.self, from: Data(json.utf8))
    }

    @Test("Un consentement absent n'est pas un consentement donné")
    func absentVautRefus() throws {
        // C'est la règle de l'article 9 : le silence ne vaut pas accord. Rien
        // ne la tenait — `?? true` laissait les quinze tests au vert, et le fil
        // aurait filtré par genre sans qu'on ait jamais rien demandé.
        let etat = try lire(#"{"consents":[],"policyVersion":"2026-09-12"}"#)
        #expect(etat.estActif(.donneesSensibles) == false)
        #expect(etat.etat(.donneesSensibles) == nil)
    }

    @Test("Un consentement retiré ne vaut plus")
    func retireNeVautPlus() throws {
        // La ligne RESTE en base après un retrait — la politique promet de
        // consigner les deux dates. Se fier à sa présence plutôt qu'à `active`
        // rendrait donc un retrait sans effet, et c'est la seule chose qu'un
        // retrait doit avoir.
        let etat = try lire(#"""
        {"consents":[{"kind":"donnees_sensibles","active":false,
        "version":"2026-09-12","grantedAt":"2026-09-01T10:00:00.000Z",
        "revokedAt":"2026-09-10T10:00:00.000Z"}],"policyVersion":"2026-09-12"}
        """#)
        #expect(etat.etat(.donneesSensibles) != nil)
        #expect(etat.estActif(.donneesSensibles) == false)
    }

    @Test("Un consentement en vigueur vaut")
    func accordeVaut() throws {
        // Une garde qui refuse tout ne garde rien.
        let etat = try lire(#"""
        {"consents":[{"kind":"donnees_sensibles","active":true,
        "version":"2026-09-12","grantedAt":"2026-09-01T10:00:00.000Z",
        "revokedAt":null}],"policyVersion":"2026-09-12"}
        """#)
        #expect(etat.estActif(.donneesSensibles) == true)
    }

    @Test("Seule une demande en attente est en attente")
    func verificationEnAttente() throws {
        func etat(_ statut: String) throws -> EtatDeVerification {
            try decodeur.decode(EtatDeVerification.self, from: Data(#"""
            {"verified":false,"request":{"state":"\#(statut)",
            "createdAt":"2026-09-01T10:00:00.000Z","handledAt":null,
            "decision":""}}
            """#.utf8))
        }
        #expect(try etat("en_attente").enAttente == true)
        // Une demande TRANCHÉE n'attend plus. Confondre les deux laisserait
        // l'écran annoncer une vérification en cours après un refus, et il n'y
        // aurait plus aucun moyen d'en déposer une autre.
        #expect(try etat("acceptee").enAttente == false)
        #expect(try etat("refusee").enAttente == false)

        let aucune = try decodeur.decode(
            EtatDeVerification.self, from: Data(#"{"verified":true,"request":null}"#.utf8)
        )
        #expect(aucune.enAttente == false)
    }
}

// MARK: - Critères du fil

@Suite("Critères du fil")
struct PreferencesTests {
    private func lire(escaleUntil: String?, distance: Int = 25, applique: Int = 25) throws
        -> Preferences
    {
        let fin = escaleUntil.map { "\"\($0)\"" } ?? "null"
        return try decodeur.decode(Preferences.self, from: Data(#"""
        {"minAge":18,"maxAge":32,"maxDistanceKm":\#(distance),
        "effectiveDistanceKm":\#(applique),"seeking":["femme"],
        "categories":["sport"],"days":[],
        "escaleCity":"Lyon","escaleUntil":\#(fin)}
        """#.utf8))
    }

    @Test("Une escale terminée n'est plus en cours")
    func escaleTerminee() throws {
        // Le serveur laisse les DEUX colonnes en place après le terme : la
        // ville reste écrite. S'y fier plutôt qu'à l'échéance composerait le
        // fil autour de cette ville pour toujours — on rentre chez soi, et le
        // fil reste en voyage.
        let finie = try lire(escaleUntil: "2020-01-01T00:00:00.000Z")
        #expect(finie.escaleCity == "Lyon")
        #expect(finie.escaleEnCours == false)

        let encours = try lire(escaleUntil: "2099-01-01T00:00:00.000Z")
        #expect(encours.escaleEnCours == true)

        let jamais = try lire(escaleUntil: nil)
        #expect(jamais.escaleEnCours == false)
    }

    @Test("Un serveur qui ne parle pas encore du rappel ne fait pas tomber les critères")
    func rappelAbsent() throws {
        // Une application à jour peut parler à un serveur qui ne l'est pas
        // encore — c'est même l'ordre normal d'un déploiement. Un champ
        // obligatoire aurait fait échouer le décodage ENTIER des critères :
        // l'écran des réglages serait resté vide, et pas seulement sans son
        // interrupteur.
        let criteres = try lire(escaleUntil: nil)
        #expect(criteres.remindersOn == nil)
        // Absent vaut actif, comme au serveur : l'interrupteur ne s'affiche
        // pas éteint chez quelqu'un qui n'a rien coupé.
        #expect(criteres.rappelsActifs == true)
    }

    @Test("Le rappel coupé se lit comme coupé")
    func rappelCoupe() throws {
        let json = #"""
        {"minAge":18,"maxAge":32,"maxDistanceKm":25,"effectiveDistanceKm":25,
        "seeking":[],"categories":[],"days":[],"escaleCity":null,"escaleUntil":null,
        "remindersOn":false}
        """#
        let criteres = try decodeur.decode(Preferences.self, from: Data(json.utf8))
        #expect(criteres.rappelsActifs == false)
    }

    @Test("Un rayon rabattu se voit")
    func distanceRabattue() throws {
        // Sans « critères précis », le fil rabat le rayon sur un cran. Le
        // réglage choisi reste affiché — il revient exact dès que l'offre le
        // permet — mais le fil en retient un autre, et c'est ce décalage seul
        // qui peut l'expliquer à qui s'étonne de son fil.
        #expect(try lire(escaleUntil: nil, distance: 27, applique: 25).distanceRabattue)
        #expect(try !lire(escaleUntil: nil, distance: 25, applique: 25).distanceRabattue)
    }
}

// MARK: - Genres

@Suite("Genres")
struct GenderTests {
    @Test("Chaque genre porte un libellé écrit pour être lu")
    func libelles() {
        // Les valeurs brutes servent à la correspondance, comparée caractère
        // par caractère, et le test de contrat les tient. Ce sont donc des
        // clés, pas des mots : montrer « non_binaire » à quelqu'un lui montre
        // la plomberie. Rien ne l'empêchait.
        for genre in Gender.allCases {
            #expect(
                genre.libelle != genre.rawValue || genre.rawValue.first!.isUppercase,
                "« \(genre.rawValue) » s'affiche tel quel"
            )
            #expect(!genre.libelle.contains("_"), "« \(genre.libelle) » garde un tiret bas")
            #expect(genre.libelle.first?.isUppercase == true)
        }
        // Et les libellés se distinguent les uns des autres : un `switch` qui
        // rend le même mot deux fois rendrait le choix illisible.
        #expect(Set(Gender.allCases.map(\.libelle)).count == Gender.allCases.count)
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
    // Distincte des places par défaut : les deux étaient à 1, et les
    // intervertir ne se voyait donc pas. Un plan de quatre dont il reste une
    // place est aussi le cas ordinaire.
    capacite: Int = 4,
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
      "capacity": \(capacite),
      "seatsLeft": \(places),
      "state": "ouvert",
      "requested": \(demande),
      "createdAt": "\(DateWeave.avecFractions.format(.now))"
    }
    """
    return try! decodeur.decode(Plan.self, from: Data(json.utf8))
}
