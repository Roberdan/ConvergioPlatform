import SwiftUI

/// Premium node detail header with gradient background and status badge.
struct MeshNodeHeader: View {
    let peer: MeshPeer
    let statusLabel: String
    let statusTint: Color

    var body: some View {
        HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: 6) {
                Text(peer.displayName)
                    .font(.largeTitle.weight(.bold))
                Text(peer.peerName)
                    .font(.callout.monospaced())
                    .foregroundStyle(.secondary)
            }
            Spacer()
            StatusBadge(label: statusLabel, color: statusTint)
        }
        .padding(Spacing.md)
        .background(
            RoundedRectangle(cornerRadius: CornerRadius.lg, style: .continuous)
                .fill(
                    LinearGradient(
                        colors: [
                            statusTint.opacity(0.12),
                            Color.clear
                        ],
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    )
                )
        )
        .background(
            RoundedRectangle(cornerRadius: CornerRadius.lg, style: .continuous)
                .fill(.ultraThinMaterial)
        )
        .clipShape(RoundedRectangle(cornerRadius: CornerRadius.lg, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: CornerRadius.lg, style: .continuous)
                .strokeBorder(statusTint.opacity(0.15), lineWidth: 1)
        )
    }
}
