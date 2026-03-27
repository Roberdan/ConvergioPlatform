// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — macOS menu bar control plane

import SwiftUI

/// Menu bar app entry point.
/// Uses MenuBarExtra (.window style) so the app lives in the system tray
/// with no Dock icon (LSUIElement=true in Info.plist).
@main
struct ConvergioMissionControlApp: App {
    @State private var status = MenuBarStatus()

    var body: some Scene {
        MenuBarExtra {
            ContentView()
                .frame(width: 480, height: 640)
                .onAppear { status.startPolling() }
                .onDisappear { status.stopPolling() }
                .environment(status)
        } label: {
            HStack(spacing: 4) {
                Image(systemName: status.menuBarIcon)
                if status.activeAgents > 0 {
                    Text("\(status.activeAgents)")
                        .font(.caption2.bold())
                }
            }
            .accessibilityLabel(
                "Convergio: \(status.isHealthy ? "connected" : "offline"), \(status.activeAgents) agents"
            )
        }
        .menuBarExtraStyle(.window)
    }
}
