// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Mesh tab: P2P topology and node status

import SwiftUI

/// Mesh topology and node status via embedded dashboard mesh view.
/// Displays Tailscale P2P network with node health indicators.
struct MeshTab: View {
    @State private var nodeCount: Int?

    var body: some View {
        VStack(spacing: 0) {
            header
            webContent
        }
    }

    private var header: some View {
        HStack {
            Image(systemName: "network")
                .foregroundStyle(.blue)
            Text("Mesh Network")
                .font(.headline)
            Spacer()
            if let count = nodeCount {
                Text("\(count) nodes")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    private var webContent: some View {
        WebViewBridge(tab: "mesh") { message in
            handleBridgeMessage(message)
        }
        .frame(minHeight: 280, maxHeight: .infinity)
    }

    private func handleBridgeMessage(_ message: BridgeMessage) {
        switch message.action {
        case "mesh_update":
            if let countStr = message.payload?["node_count"],
               let count = Int(countStr) {
                nodeCount = count
            }
        default:
            break
        }
    }
}
