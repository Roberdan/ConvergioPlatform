import SwiftUI

// Branded empty-state for sections without a dedicated view yet.
// Uses per-section accent color from SidebarItem.color (F-04) and
// ConvergioCard treatment so the surface matches the rest of the DS.
struct SectionPlaceholderView: View {
    let item: SidebarItem

    var body: some View {
        VStack(spacing: Spacing.lg) {
            // Large section icon with per-section accent
            Image(systemName: item.icon)
                .font(.system(size: 64, weight: .light))
                .foregroundStyle(item.color)
                .accessibilityHidden(true)

            VStack(spacing: Spacing.xs) {
                Text(item.title)
                    .font(.title.weight(.semibold))
                    .foregroundStyle(ConvergioTokens.Text.textPrimary)

                Text(item.summary)
                    .font(.body)
                    .foregroundStyle(ConvergioTokens.Text.textMuted)
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: 400)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(ConvergioTokens.Surface.surface)
        .modifier(ConvergioCardModifier())
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(item.title): \(item.summary)")
    }
}
