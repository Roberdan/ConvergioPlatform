import Foundation
import SwiftUI

struct MeshTopologyView: View {
    @State private var model: MeshTopologyModel

    init(client: DaemonClient = DaemonClient()) {
        _model = State(initialValue: MeshTopologyModel(client: client))
    }

    var body: some View {
        HSplitView {
            VStack(alignment: .leading, spacing: 18) {
                header

                GeometryReader { geometry in
                    let layout = MeshGraphLayout(
                        peers: model.peers,
                        localNode: model.topology?.localNode,
                        size: geometry.size
                    )
                    ZStack {
                        Canvas { context, size in
                            drawBackdrop(in: &context, size: size)
                            drawEdges(layout.edges, nodeMap: layout.nodeMap, in: &context)
                        }

                        ForEach(layout.nodes) { node in
                            Button { model.select(node.peer) } label: {
                                MeshNodeBubble(
                                    peer: node.peer,
                                    heartbeat: model.heartbeat(for: node.peer),
                                    isSelected: model.selectedPeerID == node.peer.id
                                )
                            }
                            .buttonStyle(.plain)
                            .position(x: node.point.x, y: node.point.y)
                            .accessibilityLabel(nodeA11yLabel(node))
                        }
                    }
                }
                .frame(minHeight: 520)
                .modifier(ConvergioCardModifier())

                legendBar
            }
            .padding(24)

            MeshNodeDetail(model: model, peer: model.selectedPeer)
                .frame(minWidth: 340, idealWidth: 380)
        }
        .task { await model.load() }
        .task { await model.refreshLoop() }
        .overlay(alignment: .topTrailing) {
            if let errorMessage = model.errorMessage {
                Text(errorMessage)
                    .font(.footnote)
                    .foregroundStyle(ConvergioTokens.Status.error)
                    .padding(12)
            }
        }
    }

    // MARK: - Header

    private var header: some View {
        HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: 6) {
                Text("Mesh Topology")
                    .font(.largeTitle.weight(.bold))
                Text("Canvas graph backed by /api/mesh and /api/heartbeat/status.")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            headerControls
        }
    }

    private var headerControls: some View {
        VStack(alignment: .trailing, spacing: 10) {
            HStack(spacing: 10) {
                StatusBadge(
                    label: "\(onlineCount)/\(model.peers.count) online",
                    color: ConvergioTokens.Brand.azzurro
                )
                .accessibilityLabel("Status: \(onlineCount) of \(model.peers.count) peers online")
                StatusBadge(
                    label: "\(coordinatorCount) coordinators",
                    color: ConvergioTokens.Brand.petrolio
                )
                .accessibilityLabel("Status: \(coordinatorCount) coordinators")
                if let lastRefresh = model.lastRefresh {
                    StatusBadge(
                        label: relativeTime(for: lastRefresh),
                        color: ConvergioTokens.Text.textMuted
                    )
                    .accessibilityLabel("Last refreshed \(relativeTime(for: lastRefresh))")
                }
            }
            Button {
                Task { await model.refresh() }
            } label: {
                Label("Refresh", systemImage: "arrow.clockwise")
            }
            .accessibilityLabel("Refresh mesh topology")
        }
    }

    // MARK: - Legend (glass pill badges)

    private var legendBar: some View {
        HStack(spacing: 16) {
            legendPill("Healthy", color: ConvergioTokens.Brand.verdeRacing)
                .accessibilityLabel("Legend: Healthy nodes shown in green")
            legendPill("Stale", color: ConvergioTokens.Brand.arancioWarm)
                .accessibilityLabel("Legend: Stale nodes shown in orange")
            legendPill("Offline", color: ConvergioTokens.Text.textMuted)
                .accessibilityLabel("Legend: Offline nodes shown in grey")
            Spacer()
            if let daemonWs = model.topology?.daemonWs, !daemonWs.isEmpty {
                Label("Brain WS available", systemImage: "dot.radiowaves.left.and.right")
                    .foregroundStyle(.secondary)
                    .accessibilityLabel("Status: Brain WebSocket available")
            }
        }
        .font(.caption)
    }

    private func legendPill(_ text: String, color: Color) -> some View {
        HStack(spacing: 6) {
            Circle()
                .fill(color)
                .frame(width: 8, height: 8)
                .shadow(color: color.opacity(0.6), radius: 3)
            Text(text)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 5)
        .background(
            Capsule().fill(.ultraThinMaterial)
        )
        .overlay(
            Capsule().strokeBorder(color.opacity(0.2), lineWidth: 1)
        )
    }

    // MARK: - Computed properties

    private var onlineCount: Int {
        model.peers.filter(\.isOnline).count
    }

    private var coordinatorCount: Int {
        model.peers.filter { $0.role == "coordinator" }.count
    }

    private func relativeTime(for date: Date) -> String {
        RelativeDateTimeFormatter().localizedString(for: date, relativeTo: .now)
    }

    private func nodeA11yLabel(_ node: MeshGraphNode) -> String {
        "Select node \(node.peer.displayName), " +
        "\(node.peer.isOnline ? "online" : "offline"), " +
        "role \(node.peer.role ?? "unknown")"
    }

    // MARK: - Canvas drawing

    private func drawBackdrop(in context: inout GraphicsContext, size: CGSize) {
        // Radial gradient backdrop: dark blue tint from center
        let center = CGPoint(x: size.width / 2, y: size.height / 2)
        let maxR = min(size.width, size.height) / 2
        let gradient = Gradient(colors: [
            ConvergioTokens.Brand.azzurro.opacity(0.06),
            Color.clear
        ])
        let radial = GraphicsContext.Shading.radialGradient(
            gradient, center: center,
            startRadius: 0, endRadius: maxR
        )
        context.fill(Path(CGRect(origin: .zero, size: size)), with: radial)

        // Concentric rings with subtle glow
        let rings: [CGFloat] = [0.18, 0.32, 0.46]
        let ringColor = ConvergioTokens.Brand.azzurro
        for multiplier in rings {
            let diameter = min(size.width, size.height) * multiplier * 2
            let origin = CGPoint(
                x: (size.width - diameter) / 2,
                y: (size.height - diameter) / 2
            )
            let rect = CGRect(origin: origin, size: CGSize(width: diameter, height: diameter))
            let ring = Path(ellipseIn: rect)
            // Outer glow pass
            context.stroke(ring, with: .color(ringColor.opacity(0.06)), lineWidth: 4)
            // Sharp inner line
            context.stroke(ring, with: .color(ringColor.opacity(0.15)), lineWidth: 1)
        }
    }

    private func drawEdges(
        _ edges: [MeshGraphEdge],
        nodeMap: [String: CGPoint],
        in context: inout GraphicsContext
    ) {
        for edge in edges {
            guard let from = nodeMap[edge.from], let to = nodeMap[edge.to] else { continue }
            var path = Path()
            path.move(to: from)
            path.addLine(to: to)
            context.stroke(
                path,
                with: .color(edge.isActive
                    ? ConvergioTokens.Brand.azzurro.opacity(0.55)
                    : ConvergioTokens.Border.borderSubtle),
                style: StrokeStyle(
                    lineWidth: edge.isActive ? 2.4 : 1.6,
                    dash: edge.isActive ? [] : [6, 4]
                )
            )
        }
    }
}
