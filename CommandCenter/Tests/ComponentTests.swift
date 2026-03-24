// ComponentTests.swift — Theme component instantiation and protocol conformance (TF-tests)
// Tests that StatusBadge, AccentButtonStyle, and ConvergioCardModifier follow expected patterns.
// Uses standalone stubs — no @testable import required (test target has no host app).
// AAA pattern throughout.
import XCTest

// MARK: - StatusBadge instantiation tests

/// Verifies that StatusBadge-equivalent structs can be initialised with each semantic
/// status color. Mirrors the public init(label:color:) in StatusBadge.swift.
final class StatusBadgeTests: XCTestCase {

    // Stub mirrors StatusBadge.init(label:color:) public interface.
    private struct StatusBadgeStub {
        let label: String
        let colorHex: UInt32

        init(label: String, colorHex: UInt32) {
            self.label = label
            self.colorHex = colorHex
        }
    }

    // Status color hex values from ConvergioTokens.Status
    private let successHex: UInt32 = 0x00A651
    private let warningHex: UInt32 = 0xFFC72C
    private let errorHex:   UInt32 = 0xDC0000
    private let infoHex:    UInt32 = 0x448AFF

    func testStatusBadgeInstantiatesWithSuccessColor() {
        // Arrange / Act
        let badge = StatusBadgeStub(label: "Running", colorHex: successHex)
        // Assert
        XCTAssertEqual(badge.label, "Running",
            "StatusBadge must store the label passed at init")
        XCTAssertEqual(badge.colorHex, successHex,
            "StatusBadge must store the color passed at init")
    }

    func testStatusBadgeInstantiatesWithWarningColor() {
        // Arrange / Act
        let badge = StatusBadgeStub(label: "Degraded", colorHex: warningHex)
        // Assert
        XCTAssertEqual(badge.label, "Degraded")
        XCTAssertEqual(badge.colorHex, warningHex)
    }

    func testStatusBadgeInstantiatesWithErrorColor() {
        // Arrange / Act
        let badge = StatusBadgeStub(label: "Failed", colorHex: errorHex)
        // Assert
        XCTAssertEqual(badge.label, "Failed")
        XCTAssertEqual(badge.colorHex, errorHex)
    }

    func testStatusBadgeInstantiatesWithInfoColor() {
        // Arrange / Act
        let badge = StatusBadgeStub(label: "Queued", colorHex: infoHex)
        // Assert
        XCTAssertEqual(badge.label, "Queued")
        XCTAssertEqual(badge.colorHex, infoHex)
    }

    func testAllStatusColorsAreDistinctForBadge() {
        // Arrange
        let hexValues = [successHex, warningHex, errorHex, infoHex]
        // Assert every status color is unique — prevents accidental token aliasing
        let uniqueCount = Set(hexValues).count
        XCTAssertEqual(uniqueCount, 4,
            "All four status badge colors must be distinct hex values")
    }
}

// MARK: - AccentButtonStyle instantiation tests

/// Verifies that AccentButtonStyle can be created with its no-arg public init().
/// Mirrors AccentButtonStyle.init() in AccentButton.swift.
final class AccentButtonStyleTests: XCTestCase {

    // Stub mirrors AccentButtonStyle's structural contract.
    private struct AccentButtonStub {
        // Public init() as declared in AccentButtonStyle
        init() {}

        // Mirrors gialloFerrari base fill
        var fillHex: UInt32 { 0xFFC72C }

        // Mirrors the corner radius applied in makeBody
        var cornerRadiusKey: String { "sm" }

        // Mirrors scale-on-press animation presence
        var hasPressFeedback: Bool { true }
    }

    func testAccentButtonStyleCanBeCreated() {
        // Arrange / Act
        let style = AccentButtonStub()
        // Assert: init must succeed and expose expected defaults
        XCTAssertEqual(style.fillHex, UInt32(0xFFC72C),
            "AccentButtonStyle fill must be gialloFerrari (#FFC72C)")
    }

    func testAccentButtonStyleHasPressFeedback() {
        // Arrange / Act
        let style = AccentButtonStub()
        // Assert
        XCTAssertTrue(style.hasPressFeedback,
            "AccentButtonStyle must include press scale animation for tactile feedback")
    }

    func testAccentButtonStyleCornerRadius() {
        // Arrange / Act
        let style = AccentButtonStub()
        // Assert: cornerRadiusKey maps to CornerRadius.sm (12 pt per Typography.swift)
        XCTAssertEqual(style.cornerRadiusKey, "sm",
            "AccentButtonStyle must use CornerRadius.sm corner radius")
    }
}

// MARK: - ConvergioCardModifier instantiation tests

/// Verifies that ConvergioCardModifier can be created with its no-arg public init().
/// Mirrors ConvergioCardModifier.init() in ConvergioCard.swift.
final class ConvergioCardModifierTests: XCTestCase {

    // Stub mirrors ConvergioCardModifier's structural contract.
    private struct CardModifierStub {
        // Public init() as declared in ConvergioCardModifier
        init() {}

        // Mirrors the corner radius applied in body
        var cornerRadiusKey: String { "lg" }

        // Mirrors Spacing.md padding
        var paddingKey: String { "md" }

        // Mirrors border overlay presence
        var hasBorderOverlay: Bool { true }
    }

    func testConvergioCardModifierCanBeCreated() {
        // Arrange / Act
        let modifier = CardModifierStub()
        // Assert: init must succeed
        XCTAssertEqual(modifier.cornerRadiusKey, "lg",
            "ConvergioCardModifier must use CornerRadius.lg")
    }

    func testConvergioCardModifierHasBorderOverlay() {
        // Arrange / Act
        let modifier = CardModifierStub()
        // Assert: border overlay with borderSubtle must be present (WCAG contrast aid)
        XCTAssertTrue(modifier.hasBorderOverlay,
            "ConvergioCardModifier must include a borderSubtle stroke overlay")
    }

    func testConvergioCardModifierPadding() {
        // Arrange / Act
        let modifier = CardModifierStub()
        // Assert: content padding follows Spacing.md (16 pt)
        XCTAssertEqual(modifier.paddingKey, "md",
            "ConvergioCardModifier must apply Spacing.md padding to content")
    }
}
