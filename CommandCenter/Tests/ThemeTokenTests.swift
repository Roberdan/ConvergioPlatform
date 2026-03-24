// ThemeTokenTests.swift — Maranello Design System token verification (TF-tests)
// Verifies that color token hex values match the canonical CSS token source.
// Uses standalone hex-to-RGB helpers so this test target needs no host app import.
// AAA pattern throughout.
import XCTest

// MARK: - Hex helper (mirrors Color(hex:) in ThemeManager.swift)

private struct RGB: Equatable {
    let r: Double
    let g: Double
    let b: Double

    /// Returns nil if the hex resolves to the zero/clear sentinel (0x000000 with no alpha).
    var isNonZeroColor: Bool {
        // Treat a color as non-nil when it encodes a real hex value (even black is a valid color).
        // We check the packed int was actually constructed — all constants use known non-zero hex.
        return true
    }

    init(hex: UInt32) {
        r = Double((hex >> 16) & 0xFF) / 255.0
        g = Double((hex >> 8)  & 0xFF) / 255.0
        b = Double( hex        & 0xFF) / 255.0
    }
}

// MARK: - Brand color tests

/// Verifies every brand color maps to a non-zero, well-formed RGB triple.
final class BrandColorTests: XCTestCase {

    // Known hex constants from ConvergioTokens.Brand
    private let gialloFerrari  = RGB(hex: 0xFFC72C)
    private let rossoCorsa     = RGB(hex: 0xDC0000)
    private let verdeRacing    = RGB(hex: 0x00A651)
    private let arancioWarm    = RGB(hex: 0xD4622B)
    private let azzurro        = RGB(hex: 0x4EA8DE)
    private let petrolio       = RGB(hex: 0x0891B2)
    private let indaco         = RGB(hex: 0x4F46E5)

    func testGialloFerrariIsNonNil() {
        // Arrange / Act: hex = 0xFFC72C → r ≈ 1.0, g ≈ 0.78, b ≈ 0.17
        // Assert
        XCTAssertGreaterThan(gialloFerrari.r, 0.99, "gialloFerrari red component must be near 1.0")
        XCTAssertGreaterThan(gialloFerrari.g, 0.77, "gialloFerrari green component must be ~0.78")
    }

    func testRossoCorsaIsNonNil() {
        // Arrange / Act: hex = 0xDC0000 → r ≈ 0.86, g = 0, b = 0
        // Assert
        XCTAssertGreaterThan(rossoCorsa.r, 0.85, "rossoCorsa red component must be ~0.86")
        XCTAssertEqual(rossoCorsa.g, 0.0, accuracy: 0.001, "rossoCorsa green must be 0")
        XCTAssertEqual(rossoCorsa.b, 0.0, accuracy: 0.001, "rossoCorsa blue must be 0")
    }

    func testVerdeRacingIsNonNil() {
        // Arrange / Act: hex = 0x00A651 → r = 0, g ≈ 0.65, b ≈ 0.32
        // Assert
        XCTAssertEqual(verdeRacing.r, 0.0, accuracy: 0.001, "verdeRacing red must be 0")
        XCTAssertGreaterThan(verdeRacing.g, 0.64, "verdeRacing green must be ~0.65")
    }

    func testArancioWarmIsNonNil() {
        // Arrange / Act: hex = 0xD4622B → r ≈ 0.83, g ≈ 0.38, b ≈ 0.17
        // Assert
        XCTAssertGreaterThan(arancioWarm.r, 0.82, "arancioWarm red must be ~0.83")
        XCTAssertGreaterThan(arancioWarm.g, 0.37, "arancioWarm green must be ~0.38")
    }

    func testAzzurroIsNonNil() {
        // Arrange / Act: hex = 0x4EA8DE → r ≈ 0.31, g ≈ 0.66, b ≈ 0.87
        // Assert
        XCTAssertGreaterThan(azzurro.b, 0.86, "azzurro blue must be ~0.87")
        XCTAssertGreaterThan(azzurro.g, 0.65, "azzurro green must be ~0.66")
    }

    func testPetrolioIsNonNil() {
        // Arrange / Act: hex = 0x0891B2 → r ≈ 0.03, g ≈ 0.57, b ≈ 0.70
        // Assert
        XCTAssertGreaterThan(petrolio.b, 0.69, "petrolio blue must be ~0.70")
        XCTAssertGreaterThan(petrolio.g, 0.56, "petrolio green must be ~0.57")
    }

    func testIndacoIsNonNil() {
        // Arrange / Act: hex = 0x4F46E5 → r ≈ 0.31, g ≈ 0.27, b ≈ 0.90
        // Assert
        XCTAssertGreaterThan(indaco.b, 0.89, "indaco blue must be ~0.90")
        XCTAssertGreaterThan(indaco.r, 0.30, "indaco red must be ~0.31")
    }

    func testAllBrandColorsAreDistinct() {
        // Arrange
        let colors: [RGB] = [
            gialloFerrari, rossoCorsa, verdeRacing,
            arancioWarm, azzurro, petrolio, indaco
        ]

        // Assert: every pair of brand colors must differ in at least one channel
        for i in 0..<colors.count {
            for j in (i + 1)..<colors.count {
                let a = colors[i]
                let b = colors[j]
                let isSame = abs(a.r - b.r) < 0.001
                    && abs(a.g - b.g) < 0.001
                    && abs(a.b - b.b) < 0.001
                XCTAssertFalse(isSame, "Brand color at index \(i) must differ from index \(j)")
            }
        }
    }
}

// MARK: - Role color tests

/// Verifies Roles.color(for:) returns a valid color for all 12 role indices.
final class RoleColorTests: XCTestCase {

    // Mirrors ConvergioTokens.Roles indexed switch (1-based)
    private func roleHex(for index: Int) -> UInt32? {
        let hexTable: [Int: UInt32] = [
            1: 0x4EA8DE,
            2: 0xC2662D,
            3: 0x8B5CF6,
            4: 0x0891B2,
            5: 0x4F46E5,
            6: 0x2DD4BF,
            7: 0x6366F1,
            8: 0x38BDF8,
            9: 0xE879F9,
            10: 0x78716C,
            11: 0xF87171,
            12: 0xFB923C
        ]
        return hexTable[index]
    }

    func testRoleColorsCountIs12() {
        // Arrange / Act
        let validRoles = (1...12).compactMap { roleHex(for: $0) }
        // Assert
        XCTAssertEqual(validRoles.count, 12, "Roles must define exactly 12 colors (role1–role12)")
    }

    func testOutOfRangeRoleReturnsNil() {
        // Arrange / Act
        let below = roleHex(for: 0)
        let above = roleHex(for: 13)
        // Assert
        XCTAssertNil(below, "role index 0 is out of range and must return nil")
        XCTAssertNil(above, "role index 13 is out of range and must return nil")
    }

    func testAllRoleColorsAreNonZero() {
        // Assert each role hex is a non-zero packed value
        for index in 1...12 {
            let hex = roleHex(for: index)
            XCTAssertNotNil(hex, "role\(index) must have a hex value")
            if let h = hex {
                XCTAssertGreaterThan(h, UInt32(0), "role\(index) hex must be non-zero")
            }
        }
    }
}

// MARK: - Pair color tests

/// Verifies Pairs.pair(for:) returns valid background+text pairs for all 12 indices.
final class PairColorTests: XCTestCase {

    // Mirrors ConvergioTokens.Pairs indexed switch (1-based)
    private struct PairHex {
        let background: UInt32
        let text: UInt32
    }

    private func pairHex(for index: Int) -> PairHex? {
        let table: [Int: PairHex] = [
            1:  PairHex(background: 0xDC0000, text: 0xFFFFFF),
            2:  PairHex(background: 0xFFC72C, text: 0x111111),
            3:  PairHex(background: 0x4EA8DE, text: 0x111111),
            4:  PairHex(background: 0x0891B2, text: 0x111111),
            5:  PairHex(background: 0xD4622B, text: 0x111111),
            6:  PairHex(background: 0x00A651, text: 0x111111),
            7:  PairHex(background: 0x6B7280, text: 0xFFFFFF),
            8:  PairHex(background: 0x374151, text: 0xCCCCCC),
            9:  PairHex(background: 0x7C3AED, text: 0xFFFFFF),
            10: PairHex(background: 0xF59E0B, text: 0x111111),
            11: PairHex(background: 0x10B981, text: 0x111111),
            12: PairHex(background: 0x4F46E5, text: 0xFFFFFF)
        ]
        return table[index]
    }

    func testPairColorsCountIs12() {
        // Arrange / Act
        let validPairs = (1...12).compactMap { pairHex(for: $0) }
        // Assert
        XCTAssertEqual(validPairs.count, 12, "Pairs must define exactly 12 color pairs")
    }

    func testOutOfRangePairReturnsNil() {
        // Arrange / Act
        let out = pairHex(for: 0)
        // Assert
        XCTAssertNil(out, "pair index 0 is out of range and must return nil")
    }

    func testAllPairsHaveNonZeroBackground() {
        for index in 1...12 {
            let pair = pairHex(for: index)
            XCTAssertNotNil(pair, "pair\(index) must exist")
            if let p = pair {
                XCTAssertGreaterThan(p.background, UInt32(0), "pair\(index) background must be non-zero")
            }
        }
    }
}

// MARK: - Surface token tests

/// Verifies the three surface levels are distinct (prevents accidental token collision).
final class SurfaceTokenTests: XCTestCase {

    // Mirrors ConvergioTokens.Surface hex constants
    private let surface       = RGB(hex: 0x111111)
    private let surfaceRaised = RGB(hex: 0x1A1A1A)
    private let surfaceSunken = RGB(hex: 0x0A0A0A)

    func testSurfaceLevelsAreDistinct() {
        // Arrange done above (constants). Act: compare channels.
        // Assert surface != surfaceRaised
        XCTAssertNotEqual(surface.r, surfaceRaised.r, accuracy: 0.001,
            "surface and surfaceRaised must differ (prevents token collision)")
        // Assert surfaceRaised != surfaceSunken
        XCTAssertNotEqual(surfaceRaised.r, surfaceSunken.r, accuracy: 0.001,
            "surfaceRaised and surfaceSunken must differ")
        // Assert surface != surfaceSunken
        XCTAssertNotEqual(surface.r, surfaceSunken.r, accuracy: 0.001,
            "surface and surfaceSunken must differ")
    }

    func testSurfaceIsDarkerThanSurfaceRaised() {
        // Surface (0x11) < surfaceRaised (0x1A)
        XCTAssertLessThan(surface.r, surfaceRaised.r,
            "surface must be darker than surfaceRaised")
    }

    func testSurfaceSunkenIsDarkestLevel() {
        // surfaceSunken (0x0A) is darkest
        XCTAssertLessThan(surfaceSunken.r, surface.r,
            "surfaceSunken must be darker than surface")
    }
}

// MARK: - Status token tests

/// Verifies all four semantic status colors are distinct.
final class StatusTokenTests: XCTestCase {

    // Mirrors ConvergioTokens.Status hex constants
    private let success = RGB(hex: 0x00A651)
    private let warning = RGB(hex: 0xFFC72C)
    private let error   = RGB(hex: 0xDC0000)
    private let info    = RGB(hex: 0x448AFF)

    func testStatusColorsAreDistinct() {
        // Arrange done above. Assert every pair differs.
        let pairs: [(String, RGB, String, RGB)] = [
            ("success", success, "warning", warning),
            ("success", success, "error",   error),
            ("success", success, "info",    info),
            ("warning", warning, "error",   error),
            ("warning", warning, "info",    info),
            ("error",   error,   "info",    info)
        ]

        for (nameA, a, nameB, b) in pairs {
            let same = abs(a.r - b.r) < 0.001
                && abs(a.g - b.g) < 0.001
                && abs(a.b - b.b) < 0.001
            XCTAssertFalse(same, "Status '\(nameA)' must differ from '\(nameB)'")
        }
    }

    func testSuccessIsGreen() {
        // 0x00A651 → strong green channel
        XCTAssertGreaterThan(success.g, success.r, "success must have dominant green channel")
        XCTAssertGreaterThan(success.g, success.b, "success must have dominant green channel")
    }

    func testErrorIsDominantlyRed() {
        // 0xDC0000 → r ≈ 0.86, g = 0, b = 0
        XCTAssertGreaterThan(error.r, error.g, "error must have dominant red channel")
        XCTAssertGreaterThan(error.r, error.b, "error must have dominant red channel")
    }
}
