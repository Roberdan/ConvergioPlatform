// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Chat tab: multi-agent chat interface

import SwiftUI

/// Multi-agent chat interface via embedded dashboard chat panel.
/// Supports multiple conversation tabs and real-time message streaming.
struct ChatTab: View {
    @State private var unreadCount: Int = 0

    var body: some View {
        VStack(spacing: 0) {
            header
            webContent
        }
    }

    private var header: some View {
        HStack {
            Image(systemName: "bubble.left.and.bubble.right")
                .foregroundStyle(.teal)
            Text("Chat")
                .font(.headline)
            Spacer()
            if unreadCount > 0 {
                Text("\(unreadCount)")
                    .font(.caption2.bold())
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(.red)
                    .foregroundStyle(.white)
                    .clipShape(Capsule())
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    private var webContent: some View {
        WebViewBridge(tab: "chat") { message in
            handleBridgeMessage(message)
        }
        .frame(minHeight: 300, maxHeight: .infinity)
    }

    private func handleBridgeMessage(_ message: BridgeMessage) {
        switch message.action {
        case "new_message":
            unreadCount += 1
        case "messages_read":
            unreadCount = 0
        default:
            break
        }
    }
}
