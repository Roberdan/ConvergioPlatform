// swift-tools-version: 5.9
// SPDX-License-Identifier: MPL-2.0

import PackageDescription

let package = Package(
    name: "ConvergioMissionControl",
    platforms: [
        .macOS(.v14)
    ],
    targets: [
        .executableTarget(
            name: "ConvergioMissionControl",
            path: "ConvergioMissionControl",
            exclude: ["Info.plist"]
        )
    ]
)
