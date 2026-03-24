import SwiftUI

/// Pill-shaped model badge with gradient fill.
/// Opus = gold gradient, Sonnet = silver gradient, Haiku = bronze gradient.
struct PremiumModelBadge: View {
    let modelName: String

    var body: some View {
        Text(modelName)
            .font(.label)
            .foregroundStyle(foregroundColor)
            .padding(.horizontal, Spacing.xs)
            .padding(.vertical, Spacing.xxs)
            .background(
                Capsule().fill(
                    LinearGradient(
                        colors: gradientColors,
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    )
                )
            )
    }

    private var lower: String { modelName.lowercased() }

    private var gradientColors: [Color] {
        if lower.contains("opus") {
            // Gold gradient
            return [Color(hex: 0xFFC72C), Color(hex: 0xD4A017)]
        } else if lower.contains("haiku") {
            // Bronze gradient
            return [Color(hex: 0xD4622B), Color(hex: 0x8B4513)]
        } else {
            // Silver gradient (sonnet and others)
            return [Color(hex: 0x9E9E9E), Color(hex: 0x6B6B6B)]
        }
    }

    private var foregroundColor: Color {
        if lower.contains("opus") {
            return Color(hex: 0x111111)
        } else {
            return .white
        }
    }
}
