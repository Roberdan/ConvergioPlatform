# CommandCenter Theme System

Maranello Luce design tokens translated to Swift. Single source of truth for color, typography, spacing, and component styling across all views.

## ConvergioTokens — Color References

Raw palette constants and semantic role aliases:

```swift
// Raw token
Color(ConvergioTokens.Colors.maranelloRed)

// Semantic role (preferred)
Color(ConvergioTokens.Roles.accent)       // primary brand action
Color(ConvergioTokens.Roles.surface)      // card / panel background
Color(ConvergioTokens.Roles.textPrimary)  // body text
Color(ConvergioTokens.Roles.destructive)  // warnings, errors
```

Use semantic roles in views. Use raw tokens only when a role does not exist.

## Typography — Font Extensions, Spacing, CornerRadius

```swift
// Fonts
Text("Dashboard").font(.convergioTitle)
Text("Plan name").font(.convergioBody)
Text("BETA").font(.convergioCaption)

// Spacing (use instead of magic numbers)
.padding(Spacing.md)          // 16 pt
.padding(.horizontal, Spacing.lg)  // 24 pt

// Corner radius
.cornerRadius(CornerRadius.card)   // 12 pt
.cornerRadius(CornerRadius.badge)  // 6 pt
```

## Components

| Component | Usage |
|---|---|
| `ConvergioCard` | Generic elevated container — wraps any content in a themed surface |
| `StatusBadge` | Colored pill for plan/task status labels |
| `SectionHeader` | Sidebar and list section titles with brand typography |
| `AccentButton` | Primary CTA button using `Roles.accent` fill |

```swift
ConvergioCard { Text("Content") }

StatusBadge(label: "In Progress", status: .inProgress)

SectionHeader("Plans")

AccentButton("Approve") { approve() }
```

## ThemeManager

`ThemeManager` is an `ObservableObject` injected at app root. Views read the active theme reactively.

```swift
// App root (CommandCenterApp.swift)
@StateObject private var themeManager = ThemeManager()
// ...
ContentView().environment(themeManager)

// Any view
@EnvironmentObject var theme: ThemeManager
// theme.current == .editorial | .nero | .avorio
```

Switching theme:

```swift
theme.select(.nero)
```

Available themes: `.editorial` (light, default), `.nero` (dark), `.avorio` (warm light).
