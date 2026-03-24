// AccentButton.swift
// ButtonStyle that applies the Maranello giallo-ferrari primary action style.

import SwiftUI

// MARK: - AccentButtonStyle

/// A prominent ButtonStyle using ConvergioTokens.Brand.gialloFerrari as fill.
///
/// Features:
/// - Background: ConvergioTokens.Brand.gialloFerrari (hover: gialloFerrariLight ≈ +20% brightness)
/// - Text: ConvergioTokens.Surface.surface (dark label on yellow for WCAG contrast)
/// - Corner radius: CornerRadius.sm (12 pt)
/// - Padding: horizontal Spacing.lg (24 pt), vertical Spacing.sm (12 pt)
/// - Press: scale 0.97 with spring animation
///
/// Usage:
/// ```swift
/// Button("Run Plan") { ... }
///     .buttonStyle(AccentButtonStyle())
/// ```
public struct AccentButtonStyle: ButtonStyle {

    // Track hover state for the fill swap.
    @State private var isHovered = false

    public init() {}

    public func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.label)
            .foregroundStyle(ConvergioTokens.Surface.surface)
            .padding(.horizontal, Spacing.lg)      // 24 pt
            .padding(.vertical, Spacing.sm)        // 12 pt
            .background(
                RoundedRectangle(cornerRadius: CornerRadius.sm, style: .continuous)
                    .fill(isHovered ? gialloFerrariLight : ConvergioTokens.Brand.gialloFerrari)
            )
            // Scale feedback on press for tactile feel.
            .scaleEffect(configuration.isPressed ? 0.97 : 1.0)
            .animation(.spring(response: 0.2, dampingFraction: 0.7), value: configuration.isPressed)
            .animation(.easeInOut(duration: 0.15), value: isHovered)
            .onHover { hovering in isHovered = hovering }
    }

    // MARK: Private

    /// Lighter variant of gialloFerrari used on hover.
    /// Derived by brightening the base color by ~20 % in the HSB model
    /// (no separate token exists in v1.4.0).
    private var gialloFerrariLight: Color {
        Color(hue: 0.124, saturation: 0.82, brightness: 1.0)
    }
}

// MARK: - Preview helpers

#if DEBUG
struct AccentButtonStyle_Previews: PreviewProvider {
    static var previews: some View {
        VStack(spacing: Spacing.md) {
            Button("Run Plan")  { }
                .buttonStyle(AccentButtonStyle())
            Button("Disabled")  { }
                .buttonStyle(AccentButtonStyle())
                .disabled(true)
        }
        .padding()
        .previewLayout(.sizeThatFits)
    }
}
#endif
