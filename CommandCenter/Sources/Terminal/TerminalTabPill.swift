import SwiftUI

// MARK: - Tab pill for the terminal tab strip

struct TerminalTabPill: View {
    let tab: TerminalSession
    let isActive: Bool
    let onSelect: () -> Void
    let onClose: () -> Void

    var body: some View {
        HStack(spacing: 8) {
            Button(tab.title, action: onSelect)
                .buttonStyle(.plain)
                .foregroundStyle(isActive ? ConvergioTokens.Surface.surface : ConvergioTokens.Text.textPrimary)
                .accessibilityLabel("Terminal tab: \(tab.title)\(isActive ? ", active" : "")")

            Button(action: onClose) {
                Image(systemName: "xmark.circle.fill")
                    .foregroundStyle(
                        isActive ? ConvergioTokens.Surface.surface.opacity(0.7) : ConvergioTokens.Text.textMuted
                    )
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Close tab \(tab.title)")
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 8)
        .background(pillBackground)
    }

    @ViewBuilder
    private var pillBackground: some View {
        if isActive {
            Capsule()
                .fill(ConvergioTokens.Brand.gialloFerrari)
                .shadow(color: ConvergioTokens.Brand.gialloFerrari.opacity(0.45), radius: 8, y: 2)
        } else {
            Capsule()
                .fill(ConvergioTokens.Surface.surfaceRaised)
        }
    }
}
