import SwiftUI
import WidgetKit

/// Extension widget : elle héberge la Live Activity « Métier » et, sur montre,
/// la complication du cadran.
@main
struct WeaveActivityBundle: WidgetBundle {
    var body: some Widget {
        WeaveLiveActivity()
    }
}
