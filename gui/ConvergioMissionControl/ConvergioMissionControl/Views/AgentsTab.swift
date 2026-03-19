// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Agents tab: agent roster and activity feed

import SwiftUI

/// Agent roster and activity feed via embedded dashboard agents view.
/// Shows running agents, their projects, and real-time status.
struct AgentsTab: View {
    @State private var activeAgentCount: Int?

    var body: some View {
        VStack(spacing: 0) {
            header
            webContent
        }
    }

    private var header: some View {
        HStack {
            Image(systemName: "person.3")
                .foregroundStyle(.green)
            Text("Agents")
                .font(.headline)
            Spacer()
            if let count = activeAgentCount {
                Text("\(count) active")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    private var webContent: some View {
        WebViewBridge(tab: "agents") { message in
            handleBridgeMessage(message)
        }
        .frame(minHeight: 280, maxHeight: .infinity)
    }

    private func handleBridgeMessage(_ message: BridgeMessage) {
        switch message.action {
        case "agents_update":
            if let countStr = message.payload?["active_count"],
               let count = Int(countStr) {
                activeAgentCount = count
            }
        default:
            break
        }
    }
}
