// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "CommandCenter",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "CommandCenter", targets: ["CommandCenter"])
    ],
    targets: [
        .executableTarget(
            name: "CommandCenter",
            path: "Sources/CommandCenter",
            // Info.plist used by app bundle; excluded from SPM resource processing
            exclude: ["Info.plist"]
        )
    ]
)
