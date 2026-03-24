// ConvergioTokens.swift — Maranello Design System color tokens v1.4.0
// Translated from tokens-color.css. All hex values match the CSS source exactly.
import SwiftUI

enum ConvergioTokens {

    // MARK: — Brand palette

    enum Brand {
        /// #FFC72C — Giallo Ferrari (primary accent)
        static let gialloFerrari   = Color(hex: 0xFFC72C)
        /// #DC0000 — Rosso Corsa
        static let rossoCorsa      = Color(hex: 0xDC0000)
        /// #00A651 — Verde Racing
        static let verdeRacing     = Color(hex: 0x00A651)
        /// #D4622B — Arancio Warm
        static let arancioWarm     = Color(hex: 0xD4622B)
        /// #4EA8DE — Azzurro
        static let azzurro         = Color(hex: 0x4EA8DE)
        /// #0891B2 — Petrolio
        static let petrolio        = Color(hex: 0x0891B2)
        /// #4F46E5 — Indaco
        static let indaco          = Color(hex: 0x4F46E5)
    }

    // MARK: — Surface tokens

    enum Surface {
        /// #111111 — default card/panel background
        static let surface        = Color(hex: 0x111111)
        /// #1a1a1a — modal/dropdown background
        static let surfaceRaised  = Color(hex: 0x1A1A1A)
        /// #0a0a0a — code blocks/inset background
        static let surfaceSunken  = Color(hex: 0x0A0A0A)
    }

    // MARK: — Text tokens

    enum Text {
        /// #fafafa — primary body text
        static let textPrimary    = Color(hex: 0xFAFAFA)
        /// #9e9e9e — muted/secondary labels
        static let textMuted      = Color(hex: 0x9E9E9E)
    }

    // MARK: — Border tokens

    enum Border {
        /// #2a2a2a — default borders
        static let border         = Color(hex: 0x2A2A2A)
        /// rgba(255,255,255,0.07) — hairline dividers
        static let borderSubtle   = Color(white: 1.0, opacity: 0.07)
    }

    // MARK: — Status tokens

    enum Status {
        /// #00A651 — success
        static let success        = Color(hex: 0x00A651)
        /// #FFC72C — warning (uses accent)
        static let warning        = Color(hex: 0xFFC72C)
        /// #DC0000 — error
        static let error          = Color(hex: 0xDC0000)
        /// #448AFF — info
        static let info           = Color(hex: 0x448AFF)
    }

    // MARK: — Avorio (light) theme variants

    enum Avorio {
        /// #DC0000 — accent on Avorio theme
        static let accent         = Color(hex: 0xDC0000)
        /// #ffffff — base surface
        static let surface        = Color(hex: 0xFFFFFF)
        /// #faf3e6 — raised surface
        static let surfaceRaised  = Color(hex: 0xFAF3E6)
        /// #e8d5b0 — sunken/medio surface
        static let surfaceSunken  = Color(hex: 0xE8D5B0)
        /// #111111 — primary text
        static let textPrimary    = Color(hex: 0x111111)
        /// #6b6b6b — muted text
        static let textMuted      = Color(hex: 0x6B6B6B)
    }

    // MARK: — Colorblind (Okabe-Ito) theme

    enum Colorblind {
        /// #0072B2 — accent/blue
        static let accent         = Color(hex: 0x0072B2)
        /// #C94000 — error/orange-red
        static let error          = Color(hex: 0xC94000)
        /// #009E73 — success/green
        static let success        = Color(hex: 0x009E73)
        /// #E69F00 — warning/yellow
        static let warning        = Color(hex: 0xE69F00)
        /// #56B4E9 — info/sky blue
        static let info           = Color(hex: 0x56B4E9)
    }

    // MARK: — Role colors (role1–role12)
    // Full implementation with indexed access in ConvergioTokens+Roles.swift.

    enum Roles {
        static let role1  = Color(hex: 0x4EA8DE) // Azzurro
        static let role2  = Color(hex: 0xC2662D) // Rame
        static let role3  = Color(hex: 0x8B5CF6) // Viola
        static let role4  = Color(hex: 0x0891B2) // Petrolio
        static let role5  = Color(hex: 0x4F46E5) // Indaco
        static let role6  = Color(hex: 0x2DD4BF) // Turchese
        static let role7  = Color(hex: 0x6366F1) // Lavanda
        static let role8  = Color(hex: 0x38BDF8) // Celeste
        static let role9  = Color(hex: 0xE879F9) // Fucsia
        static let role10 = Color(hex: 0x78716C) // Pietra
        static let role11 = Color(hex: 0xF87171) // Corallo
        static let role12 = Color(hex: 0xFB923C) // Mandarino
    }
}

// MARK: — Hex initializer
// Defined as internal in ThemeManager.swift — visible to all files in this module.
