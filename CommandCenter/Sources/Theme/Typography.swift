// Typography.swift
// CommandCenter — semantic font scale, spacing, and corner radius tokens.
//
// Mirrors the Maranello design system (tokens-spacing-type.css v1.4.0).
// All fonts use SF Pro (system font) — no custom font bundling required.

import SwiftUI

// MARK: - Font extensions

public extension Font {

    /// Hero / page-level display heading. Maps to --text-hero (largeTitle, bold).
    static let heroTitle: Font = .largeTitle.bold()

    /// Section heading within a view. Maps to --text-h2 (title2, semibold).
    static let sectionTitle: Font = .title2.weight(.semibold)

    /// Card or list-item headline. Maps to --text-h3 (headline, semibold).
    static let cardTitle: Font = .headline

    /// Standard body copy. Maps to --text-body (body, regular).
    static let bodyMedium: Font = .body

    /// Small label / metadata. Maps to --text-caption (caption, semibold).
    static let label: Font = .caption.weight(.semibold)

    /// Monospaced body — terminal output, code, numeric values.
    static let mono: Font = .system(.body, design: .monospaced)
}

// MARK: - Spacing

/// Static spacing scale in points.
/// Matches Maranello --space-* tokens converted from rem to native points
/// (base 16 px, scaled to 4-pt grid used in Apple HIG).
///
/// Usage:  `.padding(.horizontal, Spacing.md)`
public enum Spacing {
    /// 4 pt — micro gap; icon-to-label inset.
    public static let xxs: CGFloat = 4
    /// 8 pt — tight gap; between grouped controls.
    public static let xs: CGFloat = 8
    /// 12 pt — small gap; compact list rows.
    public static let sm: CGFloat = 12
    /// 16 pt — base unit; default content padding.
    public static let md: CGFloat = 16
    /// 24 pt — comfortable section gap.
    public static let lg: CGFloat = 24
    /// 32 pt — large section or card separation.
    public static let xl: CGFloat = 32
    /// 48 pt — page-level vertical rhythm.
    public static let xxl: CGFloat = 48
}

// MARK: - CornerRadius

/// Static corner radius scale in points.
/// Matches Maranello --radius-* tokens scaled to macOS native conventions.
///
/// Usage:  `.cornerRadius(CornerRadius.md)`
public enum CornerRadius {
    /// 12 pt — compact elements; tags, badges, small chips.
    public static let sm: CGFloat = 12
    /// 18 pt — standard cards and panels.
    public static let md: CGFloat = 18
    /// 24 pt — prominent surfaces; sidebars, sheets.
    public static let lg: CGFloat = 24
    /// 32 pt — hero containers; onboarding cards.
    public static let xl: CGFloat = 32
}
