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
                    .strokeBorder(
                        LinearGradient(
                            colors: [Color.white.opacity(0.12), Color.white.opacity(0.04)],
                            startPoint: .topLeading,
                            endPoint: .bottomTrailing
                        ),
                        lineWidth: 1
                    )
            )
            .shadow(color: .black.opacity(0.25), radius: 12, x: 0, y: 4)
            .shadow(color: .black.opacity(0.08), radius: 2, x: 0, y: 1)
    }

    // MARK: Private helpers

    @ViewBuilder
    private var cardBackground: some View {
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
