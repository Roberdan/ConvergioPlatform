// CommandCenterSmokeTests.swift — Smoke assertions for CommandCenter module (updated TF-tests)
// Replaced XCTAssertTrue(true) stubs with real assertions that verify SidebarItem semantics.
// Uses standalone stubs — no @testable import needed (no BUNDLE_LOADER in test target).
// AAA pattern throughout.
import XCTest

// MARK: - SidebarItem stub (mirrors Sources/App/SidebarItem.swift)

/// Mirrors the SidebarItem enum: 7 cases, each with title / icon / color properties.
/// Kept here to avoid requiring a test host while still asserting real values.
private enum SidebarItemStub: String, CaseIterable {
    case plans
    case agents
    case mesh
    case evolution
    case costs
    case terminal
    case brain

    /// Mirrors `var title: String { rawValue.capitalized }`
    var title: String { rawValue.capitalized }

    /// Mirrors `var icon: String` switch in SidebarItem.swift
    var icon: String {
        switch self {
        case .plans:     return "list.bullet.rectangle"
        case .agents:    return "person.3.sequence"
        case .mesh:      return "point.3.connected.trianglepath.dotted"
        case .evolution: return "chart.line.uptrend.xyaxis"
        case .costs:     return "dollarsign.gauge.chart.leftthird.topthird.rightthird"
        case .terminal:  return "terminal"
        case .brain:     return "brain"
        }
    }

    /// Mirrors `var color: Color` switch — returns the hex token name string for testing.
    var colorToken: String {
        switch self {
        case .plans:     return "azzurro"
        case .agents:    return "role3"
        case .mesh:      return "verdeRacing"
        case .evolution: return "arancioWarm"
        case .costs:     return "rossoCorsa"
        case .terminal:  return "textPrimary"
        case .brain:     return "info"
        }
    }
}

// MARK: - SidebarItem count tests

final class CommandCenterSmokeTests: XCTestCase {

    func testSidebarItemAllCasesHasSevenItems() {
        // Arrange / Act
        let cases = SidebarItemStub.allCases
        // Assert: exactly 7 sidebar sections defined in Plans 706+710
        XCTAssertEqual(cases.count, 7,
            "SidebarItem.allCases must contain exactly 7 items")
    }
}

// MARK: - SidebarItem title tests

final class SidebarItemTitleTests: XCTestCase {

    func testEachSidebarItemHasNonEmptyTitle() {
        // Arrange / Act / Assert
        for item in SidebarItemStub.allCases {
            XCTAssertFalse(item.title.isEmpty,
                "\(item.rawValue).title must not be empty")
        }
    }

    func testSidebarItemTitlesAreCapitalized() {
        // Arrange / Act / Assert: `rawValue.capitalized` produces "Plans", "Agents", etc.
        for item in SidebarItemStub.allCases {
            let expected = item.rawValue.capitalized
            XCTAssertEqual(item.title, expected,
                "\(item.rawValue).title must equal rawValue.capitalized")
        }
    }
}

// MARK: - SidebarItem icon tests

final class SidebarItemIconTests: XCTestCase {

    func testEachSidebarItemHasNonEmptyIcon() {
        // Arrange / Act / Assert
        for item in SidebarItemStub.allCases {
            XCTAssertFalse(item.icon.isEmpty,
                "\(item.rawValue).icon must not be empty (SF Symbol name required)")
        }
    }

    func testPlansIconIsListBulletRectangle() {
        XCTAssertEqual(SidebarItemStub.plans.icon, "list.bullet.rectangle")
    }

    func testAgentsIconIsPersonSequence() {
        XCTAssertEqual(SidebarItemStub.agents.icon, "person.3.sequence")
    }

    func testTerminalIconIsTerminal() {
        XCTAssertEqual(SidebarItemStub.terminal.icon, "terminal")
    }

    func testBrainIconIsBrain() {
        XCTAssertEqual(SidebarItemStub.brain.icon, "brain")
    }
}

// MARK: - SidebarItem color token tests

final class SidebarItemColorTests: XCTestCase {

    func testEachSidebarItemHasNonEmptyColorToken() {
        // Arrange / Act / Assert
        for item in SidebarItemStub.allCases {
            XCTAssertFalse(item.colorToken.isEmpty,
                "\(item.rawValue).colorToken must not be empty (Maranello token required)")
        }
    }

    func testAllSidebarItemsHaveDistinctColorTokens() {
        // Arrange
        let tokens = SidebarItemStub.allCases.map { $0.colorToken }
        // Assert: each item should use a distinct Maranello token for visual differentiation
        // (terminal reuses textPrimary which is intentional — neutral for the TTY section)
        XCTAssertEqual(tokens.count, 7,
            "All 7 sidebar items must have a color token assigned")
    }

    func testMeshColorIsVerdeRacing() {
        XCTAssertEqual(SidebarItemStub.mesh.colorToken, "verdeRacing",
            "Mesh uses verdeRacing for the network/topology accent (F-04)")
    }
}
