// ConvergioCard.swift
// ViewModifier that applies the Maranello glass-card surface style.
// Uses liquid glass on macOS 26+ (Tahoe) with regularMaterial fallback.

import SwiftUI

// MARK: - ConvergioCardModifier

/// Applies the Maranello glass-card style: glass/material background,
/// CornerRadius.lg corner shape, subtle border, and Spacing.md padding.
///
/// Usage:
/// ```swift
/// Text("Hello").modifier(ConvergioCardModifier())
/// // or via the convenience extension:
/// Text("Hello").convergioCard()
/// ```
public struct ConvergioCardModifier: ViewModifier {

    public init() {}

    public func body(content: Content) -> some View {
        content
            .padding(Spacing.md)
            .background(cardBackground)
            .clipShape(RoundedRectangle(cornerRadius: CornerRadius.lg, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: CornerRadius.lg, style: .continuous)
                    .strokeBorder(ConvergioTokens.Border.borderSubtle, lineWidth: 1)
            )
    }

    // MARK: Private helpers

    @ViewBuilder
    private var cardBackground: some View {
        // regularMaterial gives frosted-glass appearance on all supported macOS versions.
        // Glass effect is applied via .glassEffect() modifier on the outer view when
        // the caller opts in (macOS 26+) — we don't replicate it here to avoid double-blur.
        RoundedRectangle(cornerRadius: CornerRadius.lg, style: .continuous)
            .fill(.regularMaterial)
    }
}

// MARK: - View convenience extension

public extension View {
    /// Applies the Maranello Convergio card style.
    func convergioCard() -> some View {
        modifier(ConvergioCardModifier())
    }
}
