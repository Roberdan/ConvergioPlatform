import SwiftUI

struct MeshNodeBubble: View {
    let peer: MeshPeer
    let heartbeat: HeartbeatPeer?
    let isSelected: Bool

    @State private var pulsing = false

    var body: some View {
        content
            .modifier(ConvergioCardModifier())
            .overlay(
                RoundedRectangle(cornerRadius: CornerRadius.lg, style: .continuous)
                    .stroke(strokeColor, lineWidth: 2)
            )
            .shadow(
                color: isOnline ? nodeTint.opacity(pulsing ? 0.35 : 0.12) : .clear,
                radius: isOnline ? 10 : 0, y: 0
            )
            .shadow(color: .black.opacity(0.2), radius: 6, y: 3)
            .accessibilityElement(children: .combine)
            .accessibilityLabel(accessibilityText)
            .onAppear {
                if isOnline {
                    withAnimation(.easeInOut(duration: 1.4).repeatForever(autoreverses: true)) {
                        pulsing = true
                    }
                }
            }
    }

    private var isOnline: Bool {
        heartbeat?.status == "healthy" || (heartbeat == nil && peer.isOnline)
    }

    private var content: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                // Status ring with glow
                ZStack {
                    Circle()
                        .fill(nodeTint.opacity(0.2))
                        .frame(width: 16, height: 16)
                    Circle()
                        .fill(nodeTint)
                        .frame(width: 10, height: 10)
                }
                .shadow(color: isOnline ? nodeTint.opacity(0.6) : .clear, radius: 4)
                .accessibilityLabel(statusText)

                Image(systemName: peer.role == "coordinator" ? "server.rack" : "desktopcomputer")
                    .font(.caption)
                    .accessibilityLabel(peer.role == "coordinator" ? "Coordinator" : "Worker")
                Spacer()
                if peer.isLocal {
                    Image(systemName: "house.fill")
                        .font(.caption2)
                        .foregroundStyle(ConvergioTokens.Brand.gialloFerrari)
                        .accessibilityLabel("Local node")
                }
            }

            Text(peer.displayName)
                .font(.caption.weight(.semibold))
                .lineLimit(1)

            Text(statusText)
                .font(.caption2)
                .foregroundStyle(.secondary)

            ProgressView(value: min(max(peer.primaryCPU / 100, 0), 1))
                .progressViewStyle(.linear)
                .tint(cpuTint)
                .accessibilityLabel("CPU usage: \(Int(peer.primaryCPU)) percent")
        }
        .padding(12)
        .frame(width: 130, alignment: .leading)
    }

    private var statusText: String {
        heartbeat?.status.capitalized ?? (peer.isOnline ? "Online" : "Offline")
    }

    private var accessibilityText: String {
        "\(peer.displayName), \(peer.role ?? "worker"), " +
        "Status: \(statusText), " +
        "CPU \(Int(peer.primaryCPU)) percent" +
        (isSelected ? ", selected" : "")
    }

    private var nodeTint: Color {
        switch heartbeat?.status ?? (peer.isOnline ? "healthy" : "offline") {
        case "healthy": return ConvergioTokens.Brand.verdeRacing
        case "stale":   return ConvergioTokens.Brand.arancioWarm
        default:        return ConvergioTokens.Text.textMuted
        }
    }

    /// CPU bar color shifts from green to orange to red.
    private var cpuTint: Color {
        let cpu = peer.primaryCPU
        if cpu < 60 { return ConvergioTokens.Brand.verdeRacing }
        if cpu < 85 { return ConvergioTokens.Brand.arancioWarm }
        return ConvergioTokens.Status.error
    }

    private var strokeColor: Color {
        isSelected ? nodeTint : .clear
    }
}

// MARK: - Graph data structures

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
        guard let anchor = peers.first(where: {
            $0.isLocal || $0.peerName == localNode
        }) ?? peers.first else {
            nodes = []; edges = []; nodeMap = [:]; return
        }

        let sorted = peers.sorted { $0.displayName < $1.displayName }
        let center = CGPoint(x: size.width / 2, y: size.height / 2)
        let dimension = max(min(size.width, size.height) - 80, 320)

        var built = [MeshGraphNode(peer: anchor, point: center)]
        let coordinators = sorted.filter { $0.id != anchor.id && $0.role == "coordinator" && $0.isOnline }
        let workers = sorted.filter { $0.id != anchor.id && $0.role != "coordinator" && $0.isOnline }
        let offline = sorted.filter { $0.id != anchor.id && !$0.isOnline }

        built += Self.place(coordinators, radius: dimension * 0.2, center: center, startAngle: -.pi / 2)
        built += Self.place(workers, radius: dimension * 0.32, center: center, startAngle: -.pi / 2)
        built += Self.place(
            offline, radius: dimension * 0.42,
            center: CGPoint(x: center.x, y: center.y + 26),
            startAngle: .pi * 0.15, sweep: .pi * 0.7
        )

        let coordNodes = built.filter { $0.peer.role == "coordinator" && $0.peer.id != anchor.id }
        var builtEdges = built
            .filter { $0.peer.id != anchor.id }
            .map { MeshGraphEdge(from: anchor.id, to: $0.peer.id, isActive: anchor.isOnline && $0.peer.isOnline) }

        for (lhs, rhs) in zip(coordNodes, coordNodes.dropFirst()) {
            builtEdges.append(
                MeshGraphEdge(from: lhs.peer.id, to: rhs.peer.id, isActive: lhs.peer.isOnline && rhs.peer.isOnline)
            )
        }
        if coordNodes.count > 2, let first = coordNodes.first, let last = coordNodes.last {
            builtEdges.append(
                MeshGraphEdge(from: first.peer.id, to: last.peer.id, isActive: first.peer.isOnline && last.peer.isOnline)
            )
        }

        nodes = built
        edges = builtEdges
        nodeMap = Dictionary(uniqueKeysWithValues: built.map { ($0.peer.id, $0.point) })
    }

    private static func place(
        _ peers: [MeshPeer], radius: CGFloat, center: CGPoint,
        startAngle: CGFloat, sweep: CGFloat = .pi * 2
    ) -> [MeshGraphNode] {
        guard !peers.isEmpty else { return [] }
        let step = sweep / CGFloat(max(peers.count, 1))
        return peers.enumerated().map { index, peer in
            let angle = startAngle + (CGFloat(index) * step)
            let pt = CGPoint(x: center.x + (cos(angle) * radius), y: center.y + (sin(angle) * radius))
            return MeshGraphNode(peer: peer, point: pt)
        }
    }
}
