import CoreGraphics
import Foundation

enum BrainNodeKind: String {
    case agent
    case task
    case plan
}

struct BrainGraphNode: Identifiable {
    let id: String
    let title: String
    let subtitle: String
    let detail: String
    let kind: BrainNodeKind
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
