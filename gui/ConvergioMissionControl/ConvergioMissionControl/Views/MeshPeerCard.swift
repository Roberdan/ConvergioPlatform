// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Individual mesh peer card

import SwiftUI

/// Card showing a single mesh peer with status, role, and capabilities.
struct MeshPeerCard: View {
    let peer: MeshPeer
    @State private var isHovered = false

    var body: some View {
        HStack(spacing: 10) {
            statusIndicator
            details
            Spacer()
            capabilityBadges
        }
        .padding(.vertical, 4)
        .padding(.horizontal, 8)
        .background(isHovered ? Color.accentColor.opacity(0.06) : .clear,
                    in: RoundedRectangle(cornerRadius: 6))
        .onHover { isHovered = $0 }
        .accessibilityElement(children: .combine)
        .accessibilityLabel(accessibilityLabel)
    }

    // MARK: - Subviews

    private var statusIndicator: some View {
        ZStack {
            Circle()
                .fill(peer.isOnline ? .green : .gray)
                .frame(width: 10, height: 10)
            if peer.isOnline {
                Circle()
                    .fill(.green.opacity(0.4))
                    .frame(width: 16, height: 16)
            }
        }
        .accessibilityHidden(true)
    }

    private var details: some View {
        VStack(alignment: .leading, spacing: 2) {
            HStack(spacing: 4) {
                Text(peer.peerName)
                    .font(.subheadline.bold())
                if peer.isLocal {
                    Text("(local)")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }
            }
            HStack(spacing: 6) {
                if let role = peer.role {
                    roleBadge(role)
                }
                if let lastSeen = peer.lastSeen, !peer.isOnline {
                    Text("Last seen: \(lastSeen)")
                        .font(.caption2)
                        .foregroundStyle(.tertiary)
                }
            }
        }
    }

    private func roleBadge(_ role: String) -> some View {
        Text(role)
            .font(.caption2.bold())
            .padding(.horizontal, 5)
            .padding(.vertical, 1)
            .background(roleColor(role).opacity(0.15), in: Capsule())
            .foregroundStyle(roleColor(role))
    }

    private var capabilityBadges: some View {
        HStack(spacing: 4) {
            ForEach(peer.capabilities.prefix(3), id: \.self) { cap in
                Text(cap)
                    .font(.system(size: 9).monospaced())
                    .padding(.horizontal, 4)
                    .padding(.vertical, 1)
                    .background(.quaternary, in: RoundedRectangle(cornerRadius: 3))
                    .foregroundStyle(.secondary)
            }
        }
    }

    // MARK: - Helpers

    private func roleColor(_ role: String) -> Color {
        switch role.lowercased() {
        case "coordinator": return .purple
        case "executor": return .orange
        case "validator": return .green
        default: return .blue
        }
    }

    private var accessibilityLabel: String {
        let status = peer.isOnline ? "online" : "offline"
        let role = peer.role.map { ", role \($0)" } ?? ""
        let local = peer.isLocal ? ", local node" : ""
        return "Peer \(peer.peerName), \(status)\(role)\(local)"
    }
}
