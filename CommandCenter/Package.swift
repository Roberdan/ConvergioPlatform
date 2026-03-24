// swift-tools-version: 6.2

import PackageDescription

let package = Package(
    name: "CommandCenter",
    platforms: [
        .macOS(.v26)
    ],
    products: [
        .executable(name: "CommandCenter", targets: ["CommandCenter"])
    ],
    targets: [
        .executableTarget(
            name: "CommandCenter",
            path: "Sources"
        )
    ]
)
