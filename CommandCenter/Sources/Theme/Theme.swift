// Theme.swift — Maranello Design System barrel.
// Re-exports all theme modules so consumers need only `import Theme`
// (or reference them directly within the same module/target).
//
// Included:
//   ConvergioTokens  — color tokens (brand, surface, text, border, status)
//   Spacing          — 4-pt spacing scale
//   CornerRadius     — corner radius scale
//   Font extensions  — heroTitle, sectionTitle, cardTitle, bodyMedium, label, mono
//   ConvergioCardModifier  — glass-card surface style
//   StatusBadge            — semantic capsule badge
//   SectionHeaderModifier  — uppercase heading + accent underline
//   AccentButtonStyle      — giallo-ferrari primary action button

@_exported import SwiftUI

// Token layers — ConvergioTokens, Spacing, CornerRadius, Font extensions
// are defined in ConvergioTokens.swift and Typography.swift within this target;
// they are automatically visible to all files in the same module.

// Component layer — all four component types are in Sources/Theme/Components/
// and are equally part of this module without an explicit import statement.
