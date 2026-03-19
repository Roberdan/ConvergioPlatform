// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Terminal tab: xterm.js PTY via WebSocket

import SwiftUI

/// Embedded terminal for daemon interaction via xterm.js PTY.
/// Connects to daemon WS-PTY endpoint for a full shell experience.
struct TerminalTab: View {
    @State private var sessionActive = false

    var body: some View {
        VStack(spacing: 0) {
            header
            webContent
        }
    }

    private var header: some View {
        HStack {
            Image(systemName: "terminal")
                .foregroundStyle(.mint)
            Text("Terminal")
                .font(.headline)
            Spacer()
            Circle()
                .fill(sessionActive ? .green : .gray)
                .frame(width: 8, height: 8)
            Text(sessionActive ? "Connected" : "Idle")
                .font(.caption2)
                .foregroundStyle(.secondary)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    private var webContent: some View {
        WebViewBridge(tab: "terminal") { message in
            handleBridgeMessage(message)
        }
        .frame(minHeight: 300, maxHeight: .infinity)
    }

    private func handleBridgeMessage(_ message: BridgeMessage) {
        switch message.action {
        case "pty_connected":
            sessionActive = true
        case "pty_disconnected":
            sessionActive = false
        default:
            break
        }
    }
}
