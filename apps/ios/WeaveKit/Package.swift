// swift-tools-version: 6.2
import PackageDescription

/// Cœur partagé par l'application iPhone, l'extension Live Activity et
/// l'application Apple Watch : modèles, client d'API et magasins d'état.
/// Aucune dépendance externe — tout repose sur la bibliothèque standard,
/// Foundation et Observation.
let package = Package(
    name: "WeaveKit",
    platforms: [
        .iOS(.v26),
        .watchOS(.v26),
    ],
    products: [
        .library(name: "WeaveKit", targets: ["WeaveKit"]),
    ],
    targets: [
        .target(
            name: "WeaveKit",
            swiftSettings: [
                .swiftLanguageMode(.v6),
                .enableUpcomingFeature("ExistentialAny"),
            ]
        ),
        .testTarget(
            name: "WeaveKitTests",
            dependencies: ["WeaveKit"],
            swiftSettings: [.swiftLanguageMode(.v6)]
        ),
    ]
)
