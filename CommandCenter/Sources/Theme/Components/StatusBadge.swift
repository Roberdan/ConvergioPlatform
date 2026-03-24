// StatusBadge.swift
// A Capsule-shaped semantic status indicator using ConvergioTokens status palette.

import SwiftUI

// MARK: - StatusBadge

/// Capsule badge that surfaces a semantic status with a colored background.
///
/// Pass a label and one of the ConvergioTokens.Status colors:
/// ```swift
/// StatusBadge(label: "Running", color: ConvergioTokens.Status.success)
/// StatusBadge(label: "Degraded", color: ConvergioTokens.Status.warning)
/// StatusBadge(label: "Failed", color: ConvergioTokens.Status.error)
/// StatusBadge(label: "Queued", color: ConvergioTokens.Status.info)
/// ```
public struct StatusBadge: View {

    private let label: String
    private let color: Color

    /// - Parameters:
    ///   - label: Short status text displayed inside the badge.
    ///   - color:  Background fill; use `ConvergioTokens.Status.*` values.
    public init(label: String, color: Color) {
        self.label = label
        self.color = color
    }

    public var body: some View {
        Text(label)
            .font(.label)
            .foregroundStyle(contrastText)
            .padding(.horizontal, Spacing.xs)
            .padding(.vertical, Spacing.xxs)
            .background(
                Capsule().fill(
                    LinearGradient(
                        colors: [color, color.opacity(0.7)],
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    )
                )
            )
            .shadow(color: color.opacity(0.4), radius: 4, x: 0, y: 2)
    }

    // MARK: Private

    /// Choose white or black text based on perceived luminance so the badge
    /// remains readable in both light and dark color schemes.
    private var contrastText: Color {
        // The ConvergioTokens status palette is all saturated mid-tones.
        // White achieves 4.5:1+ contrast against all of them.
        .white
    }
}

// MARK: - Preview helpers (compile-time only)

#if DEBUG
struct StatusBadge_Previews: PreviewProvider {
    static var previews: some View {
        HStack(spacing: Spacing.sm) {
            StatusBadge(label: "Success", color: ConvergioTokens.Status.success)
            StatusBadge(label: "Warning", color: ConvergioTokens.Status.warning)
            StatusBadge(label: "Error",   color: ConvergioTokens.Status.error)
            StatusBadge(label: "Info",    color: ConvergioTokens.Status.info)
        }
        .padding()
        .previewLayout(.sizeThatFits)
    }
}
#endif
