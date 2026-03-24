// ThemeManager.swift — Maranello Design System multi-theme support v1.0.0
// Three themes: Nero (dark default), Avorio (warm light), Colorblind (Okabe-Ito palette).
// Why: Accessibility and user preference require switchable token sets at runtime.
import SwiftUI

// MARK: — Theme enum

enum ConvergioTheme: String, CaseIterable, Identifiable {
    case nero       = "nero"
    case avorio     = "avorio"
    case colorblind = "colorblind"

    var id: String { rawValue }

    var displayName: String {
        switch self {
        case .nero:       "Nero"
        case .avorio:     "Avorio"
        case .colorblind: "Colorblind"
        }
    }
}

// MARK: — ThemeManager

@MainActor
@Observable
final class ThemeManager {

    static let shared = ThemeManager()

    @ObservationIgnored
    @AppStorage("convergio-theme") private var _storedTheme: String = ConvergioTheme.nero.rawValue

    var current: ConvergioTheme {
        get { ConvergioTheme(rawValue: _storedTheme) ?? .nero }
        set { _storedTheme = newValue.rawValue }
    }

    private init() {}

    // MARK: — Accent

    var accent: Color {
        switch current {
        case .nero:       ConvergioTokens.Brand.gialloFerrari
        case .avorio:     Color(hex: 0xDC0000)
        case .colorblind: Color(hex: 0x0072B2)
        }
    }

    // MARK: — Surfaces

    var surface: Color {
        switch current {
        case .nero:       Color(hex: 0x111111)
        case .avorio:     Color(hex: 0xFFFFFF)
        case .colorblind: Color(hex: 0x111111)
        }
    }

    var surfaceRaised: Color {
        switch current {
        case .nero:       Color(hex: 0x1A1A1A)
        case .avorio:     Color(hex: 0xFAF3E6)
        case .colorblind: Color(hex: 0x1A1A1A)
        }
    }

    var surfaceSunken: Color {
        switch current {
        case .nero:       Color(hex: 0x0A0A0A)
        case .avorio:     Color(hex: 0xE8D5B0)
        case .colorblind: Color(hex: 0x0A0A0A)
        }
    }

    // MARK: — Text

    var text: Color {
        switch current {
        case .nero:       Color(hex: 0xFAFAFA)
        case .avorio:     Color(hex: 0x111111)
        case .colorblind: Color(hex: 0xFAFAFA)
        }
    }

    var textMuted: Color {
        switch current {
        case .nero:       Color(hex: 0x9E9E9E)
        case .avorio:     Color(hex: 0x6B6B6B)
        case .colorblind: Color(hex: 0x9E9E9E)
        }
    }

    // MARK: — Border

    var border: Color {
        switch current {
        case .nero:       Color(hex: 0x2A2A2A)
        case .avorio:     Color(hex: 0xD7C39A)
        case .colorblind: Color(hex: 0x2A2A2A)
        }
    }

    // MARK: — Status

    var success: Color {
        switch current {
        case .nero:       Color(hex: 0x00A651)
        case .avorio:     Color(hex: 0x008844)
        case .colorblind: Color(hex: 0x009E73)
        }
    }

    var warning: Color {
        switch current {
        case .nero:       Color(hex: 0xFFC72C)
        case .avorio:     Color(hex: 0xD4A000)
        case .colorblind: Color(hex: 0xE69F00)
        }
    }

    var error: Color {
        switch current {
        case .nero:       Color(hex: 0xDC0000)
        case .avorio:     Color(hex: 0xDC0000)
        case .colorblind: Color(hex: 0xC94000)
        }
    }

    var info: Color {
        switch current {
        case .nero:       Color(hex: 0x448AFF)
        case .avorio:     Color(hex: 0x2B7EB5)
        case .colorblind: Color(hex: 0x56B4E9)
        }
    }
}

// MARK: — ThemeToggleView

/// Compact segmented picker for switching between Nero, Avorio, and Colorblind themes.
/// Placement: settings panels or toolbar. Does not require Environment injection — reads ThemeManager.shared.
struct ThemeToggleView: View {

    @State private var manager = ThemeManager.shared

    var body: some View {
        Picker("Theme", selection: Bindable(manager).current) {
            ForEach(ConvergioTheme.allCases) { theme in
                Text(theme.displayName).tag(theme)
            }
        }
        .pickerStyle(.segmented)
        .labelsHidden()
        .accessibilityLabel("Color theme selector")
    }
}

// MARK: — Hex initializer (internal to Theme module)

extension Color {
    /// Initialise from a packed 24-bit RGB hex literal, e.g. `Color(hex: 0xFFC72C)`.
    init(hex: UInt32, opacity: Double = 1.0) {
        let r = Double((hex >> 16) & 0xFF) / 255.0
        let g = Double((hex >> 8)  & 0xFF) / 255.0
        let b = Double( hex        & 0xFF) / 255.0
        self.init(red: r, green: g, blue: b, opacity: opacity)
    }
}
