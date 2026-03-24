import SwiftUI

struct MeshNodeBubble: View {
    let peer: MeshPeer
    let heartbeat: HeartbeatPeer?
    let isSelected: Bool

    var body: some View {
        Group {
            if #available(macOS 26.0, *) {
                content.glassEffect()
            } else {
                content.background(.regularMaterial, in: RoundedRectangle(cornerRadius: 18, style: .continuous))
            }
        }
        .overlay(RoundedRectangle(cornerRadius: 18, style: .continuous).stroke(strokeColor, lineWidth: 2))
    }

    private var content: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Circle().fill(tint).frame(width: 10, height: 10)
                Image(systemName: peer.role == "coordinator" ? "server.rack" : "desktopcomputer")
                    .font(.caption)
                Spacer()
                if peer.isLocal {
                    Image(systemName: "house.fill").font(.caption2)
                }
            }

            Text(peer.displayName)
                .font(.caption.weight(.semibold))
                .lineLimit(1)

            Text(heartbeat?.status.capitalized ?? (peer.isOnline ? "Online" : "Offline"))
                .font(.caption2)
                .foregroundStyle(.secondary)

            ProgressView(value: min(max(peer.primaryCPU / 100, 0), 1))
                .progressViewStyle(.linear)
        }
        .padding(10)
        .frame(width: 118, alignment: .leading)
    }

    private var tint: Color {
        switch heartbeat?.status ?? (peer.isOnline ? "healthy" : "offline") {
        case "healthy": return .green
        case "stale": return .orange
        default: return .secondary
        }
    }

    private var strokeColor: Color {
        isSelected ? tint : .clear
    }
}

struct MeshPanelMaterial: ViewModifier {
    func body(content: Content) -> some View {
        if #available(macOS 26.0, *) {
            content.glassEffect()
        } else {
            content.background(.regularMaterial, in: RoundedRectangle(cornerRadius: 28, style: .continuous))
        }
    }
}

struct MeshGraphNode: Identifiable {
    let peer: MeshPeer
    let point: CGPoint

    var id: String { peer.id }
}

struct MeshGraphEdge {
    let from: String
    let to: String
    let isActive: Bool
}

struct MeshGraphLayout {
    let nodes: [MeshGraphNode]
    let edges: [MeshGraphEdge]
    let nodeMap: [String: CGPoint]

    init(peers: [MeshPeer], localNode: String?, size: CGSize) {
        guard let anchor = peers.first(where: { $0.isLocal || $0.peerName == localNode }) ?? peers.first else {
            nodes = []
            edges = []
            nodeMap = [:]
            return
        }

        let sorted = peers.sorted { $0.displayName < $1.displayName }
        let center = CGPoint(x: size.width / 2, y: size.height / 2)
        let dimension = max(min(size.width, size.height) - 80, 320)

        var builtNodes = [MeshGraphNode(peer: anchor, point: center)]
        let coordinators = sorted.filter { $0.id != anchor.id && $0.role == "coordinator" && $0.isOnline }
        let workers = sorted.filter { $0.id != anchor.id && $0.role != "coordinator" && $0.isOnline }
        let offline = sorted.filter { $0.id != anchor.id && !$0.isOnline }

        builtNodes += Self.place(coordinators, radius: dimension * 0.2, center: center, startAngle: -.pi / 2)
        builtNodes += Self.place(workers, radius: dimension * 0.32, center: center, startAngle: -.pi / 2)
        builtNodes += Self.place(
            offline,
            radius: dimension * 0.42,
            center: CGPoint(x: center.x, y: center.y + 26),
            startAngle: .pi * 0.15,
            sweep: .pi * 0.7
        )

        let coordinatorNodes = builtNodes.filter { $0.peer.role == "coordinator" && $0.peer.id != anchor.id }
        var builtEdges = builtNodes
            .filter { $0.peer.id != anchor.id }
            .map { MeshGraphEdge(from: anchor.id, to: $0.peer.id, isActive: anchor.isOnline && $0.peer.isOnline) }

        for (lhs, rhs) in zip(coordinatorNodes, coordinatorNodes.dropFirst()) {
            builtEdges.append(MeshGraphEdge(from: lhs.peer.id, to: rhs.peer.id, isActive: lhs.peer.isOnline && rhs.peer.isOnline))
        }
        if coordinatorNodes.count > 2, let first = coordinatorNodes.first, let last = coordinatorNodes.last {
            builtEdges.append(MeshGraphEdge(from: first.peer.id, to: last.peer.id, isActive: first.peer.isOnline && last.peer.isOnline))
        }

        nodes = builtNodes
        edges = builtEdges
        nodeMap = Dictionary(uniqueKeysWithValues: builtNodes.map { ($0.peer.id, $0.point) })
    }

    private static func place(
        _ peers: [MeshPeer],
        radius: CGFloat,
        center: CGPoint,
        startAngle: CGFloat,
        sweep: CGFloat = .pi * 2
    ) -> [MeshGraphNode] {
        guard !peers.isEmpty else { return [] }
        let step = sweep / CGFloat(max(peers.count, 1))
        return peers.enumerated().map { index, peer in
            let angle = startAngle + (CGFloat(index) * step)
            let point = CGPoint(x: center.x + (cos(angle) * radius), y: center.y + (sin(angle) * radius))
            return MeshGraphNode(peer: peer, point: point)
        }
    }
}
