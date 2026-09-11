import SwiftUI
import WeaveKit

struct RacineView: View {
    @Environment(ModeleApplication.self) private var modele

    var body: some View {
        Group {
            if !modele.pret {
                ProgressView().controlSize(.large)
            } else if modele.connecte {
                MetierView()
            } else {
                ConnexionView()
            }
        }
        .animation(.easeInOut(duration: 0.25), value: modele.connecte)
    }
}
