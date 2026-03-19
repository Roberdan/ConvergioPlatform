// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Plans tab: plan list and execution tree

import SwiftUI

/// Plan list, execution tree, and task kanban via embedded dashboard planner view.
struct PlansTab: View {
    @State private var activePlanName: String?

    var body: some View {
        VStack(spacing: 0) {
            header
            webContent
        }
    }

    private var header: some View {
        HStack {
            Image(systemName: "list.clipboard")
                .foregroundStyle(.orange)
            Text("Plans")
                .font(.headline)
            Spacer()
            if let name = activePlanName {
                Text(name)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    private var webContent: some View {
        WebViewBridge(tab: "plans") { message in
            handleBridgeMessage(message)
        }
        .frame(minHeight: 280, maxHeight: .infinity)
    }

    private func handleBridgeMessage(_ message: BridgeMessage) {
        switch message.action {
        case "plan_selected":
            activePlanName = message.payload?["name"]
        case "plan_complete":
            activePlanName = nil
        default:
            break
        }
    }
}
