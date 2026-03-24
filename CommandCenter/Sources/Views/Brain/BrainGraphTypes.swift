import CoreGraphics
import Foundation
import SwiftUI

enum BrainNodeKind: String {
    case agent
    case task
    case plan

    /// Brand token color for this node kind.
    var color: Color {
        switch self {
        case .plan:  ConvergioTokens.Brand.azzurro
        case .task:  ConvergioTokens.Brand.verdeRacing
        case .agent: ConvergioTokens.Roles.role3
        }
    }
}

struct BrainGraphNode: Identifiable {
    let id: String
    let title: String
    let subtitle: String
    let detail: String
    let kind: BrainNodeKind

    /// Convenience accessor: brand token color derived from node kind.
    var color: Color { kind.color }
}

struct BrainGraphEdge: Identifiable {
    let from: String
    let to: String
    let kind: String

    var id: String { "\(from)->\(to):\(kind)" }
}

struct BrainNodeState {
    var position: CGPoint
    var velocity: CGVector
}
