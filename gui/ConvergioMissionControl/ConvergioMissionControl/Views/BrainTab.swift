// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Brain tab: neural visualization and daemon health

import SwiftUI

/// Neural visualization and daemon health overview via embedded dashboard brain canvas.
/// Renders in compact 300px height for menu bar popover layout.
struct BrainTab: View {
    @State private var lastEvent: String?

    var body: some View {
        VStack(spacing: 0) {
            header
            webContent
        }
    }

    private var header: some View {
        HStack {
            Image(systemName: "brain")
                .foregroundStyle(.purple)
            Text("Neural Overview")
                .font(.headline)
            Spacer()
            if let event = lastEvent {
                Text(event)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    private var webContent: some View {
        WebViewBridge(tab: "brain") { message in
            handleBridgeMessage(message)
        }
        .frame(minHeight: 300, maxHeight: .infinity)
    }

    private func handleBridgeMessage(_ message: BridgeMessage) {
        switch message.action {
        case "brain_pulse":
            lastEvent = message.payload?["label"] ?? "pulse"
        case "health_update":
            lastEvent = message.payload?["status"]
        default:
            break
        }
    }
}
