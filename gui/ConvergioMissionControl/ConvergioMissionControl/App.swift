// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — macOS menu bar control plane

import SwiftUI

/// Menu bar app entry point.
/// Uses MenuBarExtra (.window style) so the app lives in the system tray
/// with no Dock icon (LSUIElement=true in Info.plist).
@main
struct ConvergioMissionControlApp: App {
    var body: some Scene {
        MenuBarExtra("Convergio", systemImage: "brain.head.profile") {
            ContentView()
                .frame(width: 480, height: 640)
        }
        .menuBarExtraStyle(.window)
    }
}
