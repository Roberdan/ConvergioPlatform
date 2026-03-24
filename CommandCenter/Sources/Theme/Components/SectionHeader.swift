// SectionHeader.swift
// ViewModifier that styles a Text as a Maranello section heading with accent underline.

import SwiftUI

// MARK: - SectionHeaderModifier

/// Transforms a Text view into a styled section heading:
/// - uppercase transform
/// - `.sectionTitle` font (title2, semibold)
/// - 2 pt ConvergioTokens.Brand.gialloFerrari underline bar beneath the label
///
/// Usage:
/// ```swift
/// Text("Active Plans").modifier(SectionHeaderModifier())
/// // or via the convenience extension:
/// Text("Active Plans").sectionHeader()
/// ```
public struct SectionHeaderModifier: ViewModifier {

    public init() {}

    public func body(content: Content) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            content
                .font(.sectionTitle)               // title2, semibold
                .textCase(.uppercase)

            // 2 pt accent bar beneath the heading label.
            // Uses a fixed height Rectangle so it scales with the container width.
            Rectangle()
                .fill(ConvergioTokens.Brand.gialloFerrari)
                .frame(height: 2)
        }
    }
}

// MARK: - View convenience extension

public extension View {
    /// Applies the Maranello section-header style (uppercase + accent underline).
    func sectionHeader() -> some View {
        modifier(SectionHeaderModifier())
    }
}
