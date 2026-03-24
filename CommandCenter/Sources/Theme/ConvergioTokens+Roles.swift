// ConvergioTokens+Roles.swift — Indexed access and pair tokens (Maranello v1.4.0)
// Role color definitions live in ConvergioTokens.Roles (main file).
// This extension adds indexed access and pair (bg+text) tokens.
import SwiftUI

extension ConvergioTokens.Roles {

    // MARK: — Indexed access (1-based, returns .clear for out-of-range)

    static func color(for index: Int) -> Color {
        switch index {
        case 1:  role1
        case 2:  role2
        case 3:  role3
        case 4:  role4
        case 5:  role5
        case 6:  role6
        case 7:  role7
        case 8:  role8
        case 9:  role9
        case 10: role10
        case 11: role11
        case 12: role12
        default: .clear
        }
    }
}

extension ConvergioTokens {

    // MARK: — Pair colors (pairBg1/pairText1 … pairBg12/pairText12)

    struct ColorPair {
        let background: Color
        let text: Color
    }

    enum Pairs {
        static let pair1  = ColorPair(background: Color(pairHex: 0xDC0000), text: Color(pairHex: 0xFFFFFF))
        static let pair2  = ColorPair(background: Color(pairHex: 0xFFC72C), text: Color(pairHex: 0x111111))
        static let pair3  = ColorPair(background: Color(pairHex: 0x4EA8DE), text: Color(pairHex: 0x111111))
        static let pair4  = ColorPair(background: Color(pairHex: 0x0891B2), text: Color(pairHex: 0x111111))
        static let pair5  = ColorPair(background: Color(pairHex: 0xD4622B), text: Color(pairHex: 0x111111))
        static let pair6  = ColorPair(background: Color(pairHex: 0x00A651), text: Color(pairHex: 0x111111))
        static let pair7  = ColorPair(background: Color(pairHex: 0x6B7280), text: Color(pairHex: 0xFFFFFF))
        static let pair8  = ColorPair(background: Color(pairHex: 0x374151), text: Color(pairHex: 0xCCCCCC))
        static let pair9  = ColorPair(background: Color(pairHex: 0x7C3AED), text: Color(pairHex: 0xFFFFFF))
        static let pair10 = ColorPair(background: Color(pairHex: 0xF59E0B), text: Color(pairHex: 0x111111))
        static let pair11 = ColorPair(background: Color(pairHex: 0x10B981), text: Color(pairHex: 0x111111))
        static let pair12 = ColorPair(background: Color(pairHex: 0x4F46E5), text: Color(pairHex: 0xFFFFFF))

        /// Indexed access — index is 1-based (1…12). Out of range returns .init(background:.clear,text:.clear).
        static func pair(for index: Int) -> ColorPair {
            switch index {
            case 1:  pair1
            case 2:  pair2
            case 3:  pair3
            case 4:  pair4
            case 5:  pair5
            case 6:  pair6
            case 7:  pair7
            case 8:  pair8
            case 9:  pair9
            case 10: pair10
            case 11: pair11
            case 12: pair12
            default: ColorPair(background: .clear, text: .clear)
            }
        }
    }
}

// MARK: — Hex initializer for pair tokens (fileprivate)

fileprivate extension Color {
    init(pairHex hex: UInt32, opacity: Double = 1.0) {
        let r = Double((hex >> 16) & 0xFF) / 255.0
        let g = Double((hex >> 8)  & 0xFF) / 255.0
        let b = Double( hex        & 0xFF) / 255.0
        self.init(red: r, green: g, blue: b, opacity: opacity)
    }
}
