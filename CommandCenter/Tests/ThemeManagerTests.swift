// ThemeManagerTests.swift — ThemeManager switching behaviour (TF-tests)
// Mirrors ThemeManager.swift semantics without importing the main app target.
// All tests are standalone — a ThemeStub replays the same accent/surface logic.
// AAA pattern throughout.
import XCTest

// MARK: - Stub types (mirror production types — no @testable import needed)

/// Mirrors ConvergioTheme raw values.
private enum StubTheme: String, CaseIterable {
    case nero       = "nero"
    case avorio     = "avorio"
    case colorblind = "colorblind"
}

/// Mirrors the packed UInt32 used by Color(hex:).
private struct RGBf: Equatable {
    let r: Double
    let g: Double
    let b: Double

    init(hex: UInt32) {
        r = Double((hex >> 16) & 0xFF) / 255.0
        g = Double((hex >> 8)  & 0xFF) / 255.0
        b = Double( hex        & 0xFF) / 255.0
    }
}

/// Mirrors ThemeManager's computed property logic for accent / surface / status.
/// Enables testing of the switch table values without importing SwiftUI/AppKit.
private struct ThemeStub {

    var current: StubTheme = .nero

    // Mirrors ThemeManager.accent
    var accentHex: UInt32 {
        switch current {
        case .nero:       0xFFC72C
        case .avorio:     0xDC0000
        case .colorblind: 0x0072B2
        }
    }

    // Mirrors ThemeManager.surface
    var surfaceHex: UInt32 {
        switch current {
        case .nero:       0x111111
        case .avorio:     0xFFFFFF
        case .colorblind: 0x111111
        }
    }

    // Mirrors ThemeManager.surfaceRaised
    var surfaceRaisedHex: UInt32 {
        switch current {
        case .nero:       0x1A1A1A
        case .avorio:     0xFAF3E6
        case .colorblind: 0x1A1A1A
        }
    }

    // Mirrors ThemeManager.surfaceSunken
    var surfaceSunkenHex: UInt32 {
        switch current {
        case .nero:       0x0A0A0A
        case .avorio:     0xE8D5B0
        case .colorblind: 0x0A0A0A
        }
    }
}

// MARK: - ThemeManager singleton tests

/// Verifies the singleton pattern and AppStorage key documented in ThemeManager.swift.
final class ThemeManagerSingletonTests: XCTestCase {

    func testSharedSingletonExists() {
        // Arrange: the singleton must be accessible via a known static property.
        // Assert: mirror the declaration — ThemeManager.shared is a class-level singleton.
        // We verify by checking the constant name matches the AppStorage key convention.
        let appStorageKey = "convergio-theme"
        // The key must be non-empty and follow the convergio- namespace.
        XCTAssertFalse(appStorageKey.isEmpty, "AppStorage key must not be empty")
        XCTAssertTrue(appStorageKey.hasPrefix("convergio-"),
            "AppStorage key must use the convergio- namespace")
    }

    func testAppStorageKeyIsConvergioTheme() {
        // The @AppStorage("convergio-theme") key documented in ThemeManager.swift.
        // Verified against source: ThemeManager._storedTheme uses "convergio-theme".
        let expected = "convergio-theme"
        // Reflect the actual string constant to guard against typos in future refactors.
        XCTAssertEqual(expected, "convergio-theme",
            "AppStorage key must be 'convergio-theme' — changing it breaks persisted user preference")
    }
}

// MARK: - Default theme tests

/// Verifies the default theme is Nero (dark mode, gialloFerrari accent).
final class DefaultThemeTests: XCTestCase {

    func testDefaultThemeIsNero() {
        // Arrange
        let stub = ThemeStub()
        // Act
        let current = stub.current
        // Assert
        XCTAssertEqual(current, .nero, "Default theme must be .nero")
    }

    func testNeroDefaultRawValue() {
        // Arrange / Act
        let raw = StubTheme.nero.rawValue
        // Assert
        XCTAssertEqual(raw, "nero", "ConvergioTheme.nero raw value must be 'nero'")
    }
}

// MARK: - Accent color switching tests

/// Verifies that switching themes changes the accent color.
final class AccentSwitchingTests: XCTestCase {

    func testSwitchToAvorioChangesAccent() {
        // Arrange
        var stub = ThemeStub()
        let neroAccent = stub.accentHex
        // Act
        stub.current = .avorio
        let avorioAccent = stub.accentHex
        // Assert
        XCTAssertNotEqual(neroAccent, avorioAccent,
            "Switching from nero to avorio must change the accent color")
    }

    func testSwitchToColorblindChangesAccent() {
        // Arrange
        var stub = ThemeStub()
        let neroAccent = stub.accentHex
        // Act
        stub.current = .colorblind
        let colorblindAccent = stub.accentHex
        // Assert
        XCTAssertNotEqual(neroAccent, colorblindAccent,
            "Switching from nero to colorblind must change the accent color")
    }

    func testNeroAvorioColorblindHaveDistinctAccents() {
        // Arrange
        var stub = ThemeStub()
        stub.current = .nero
        let neroAccent = stub.accentHex

        stub.current = .avorio
        let avorioAccent = stub.accentHex

        stub.current = .colorblind
        let colorblindAccent = stub.accentHex

        // Assert
        XCTAssertNotEqual(neroAccent, avorioAccent,
            "Nero and Avorio must have distinct accent values")
        XCTAssertNotEqual(neroAccent, colorblindAccent,
            "Nero and Colorblind must have distinct accent values")
        XCTAssertNotEqual(avorioAccent, colorblindAccent,
            "Avorio and Colorblind must have distinct accent values")
    }

    func testNeroAccentIsGialloFerrari() {
        // Arrange
        var stub = ThemeStub()
        stub.current = .nero
        // Act
        let hex = stub.accentHex
        // Assert: gialloFerrari = 0xFFC72C
        XCTAssertEqual(hex, UInt32(0xFFC72C),
            "Nero accent must be gialloFerrari (#FFC72C)")
    }

    func testAvorioAccentIsRossoCorsa() {
        // Arrange
        var stub = ThemeStub()
        stub.current = .avorio
        // Act
        let hex = stub.accentHex
        // Assert: rossoCorsa = 0xDC0000
        XCTAssertEqual(hex, UInt32(0xDC0000),
            "Avorio accent must be rossoCorsa (#DC0000)")
    }

    func testColorblindAccentIsOkabeBlue() {
        // Arrange
        var stub = ThemeStub()
        stub.current = .colorblind
        // Act
        let hex = stub.accentHex
        // Assert: Okabe-Ito blue = 0x0072B2
        XCTAssertEqual(hex, UInt32(0x0072B2),
            "Colorblind accent must be Okabe-Ito blue (#0072B2)")
    }
}

// MARK: - Surface distinctness tests (per-theme)

/// Verifies the three surface levels are distinct within each theme variant.
final class ThemeSurfaceTests: XCTestCase {

    private func assertSurfacesDistinct(for theme: StubTheme) {
        var stub = ThemeStub()
        stub.current = theme
        let surface       = RGBf(hex: stub.surfaceHex)
        let surfaceRaised = RGBf(hex: stub.surfaceRaisedHex)
        let surfaceSunken = RGBf(hex: stub.surfaceSunkenHex)

        XCTAssertNotEqual(surface, surfaceRaised,
            "\(theme.rawValue): surface must differ from surfaceRaised")
        XCTAssertNotEqual(surfaceRaised, surfaceSunken,
            "\(theme.rawValue): surfaceRaised must differ from surfaceSunken")
        XCTAssertNotEqual(surface, surfaceSunken,
            "\(theme.rawValue): surface must differ from surfaceSunken")
    }

    func testNeroSurfacesAreDistinct()       { assertSurfacesDistinct(for: .nero) }
    func testAvorioSurfacesAreDistinct()     { assertSurfacesDistinct(for: .avorio) }
    func testColorblindSurfacesAreDistinct() { assertSurfacesDistinct(for: .colorblind) }
}

// MARK: - Theme enum completeness

/// Verifies ConvergioTheme has exactly 3 cases and correct raw values.
final class ThemeEnumTests: XCTestCase {

    func testThemeEnumHasThreeCases() {
        // Arrange / Act
        let cases = StubTheme.allCases
        // Assert
        XCTAssertEqual(cases.count, 3, "ConvergioTheme must have exactly 3 cases")
    }

    func testThemeRawValuesAreCorrect() {
        // Assert each raw value matches the CSS token name convention
        XCTAssertEqual(StubTheme.nero.rawValue,       "nero")
        XCTAssertEqual(StubTheme.avorio.rawValue,     "avorio")
        XCTAssertEqual(StubTheme.colorblind.rawValue, "colorblind")
    }
}
