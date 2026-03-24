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
                        }
                    }
                }
                .frame(minHeight: 520)
                .modifier(MeshPanelMaterial())

                HStack(spacing: 16) {
                    legendItem("Healthy", color: .green)
                    legendItem("Stale", color: .orange)
                    legendItem("Offline", color: .secondary)
                    Spacer()
                    if let daemonWs = model.topology?.daemonWs, !daemonWs.isEmpty {
                        Label("Brain WS available", systemImage: "dot.radiowaves.left.and.right")
                            .foregroundStyle(.secondary)
                    }
                }
                .font(.caption)
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
                    .foregroundStyle(.red)
                    .padding(12)
            }
        }
    }

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

            VStack(alignment: .trailing, spacing: 10) {
                HStack(spacing: 10) {
                    summaryChip("\(onlineCount)/\(model.peers.count) online", icon: "network")
                    summaryChip("\(coordinatorCount) coordinators", icon: "server.rack")
                    if let lastRefresh = model.lastRefresh {
                        summaryChip(relativeTime(for: lastRefresh), icon: "clock")
                    }
                }

                Button {
                    Task { await model.refresh() }
                } label: {
                    Label("Refresh", systemImage: "arrow.clockwise")
                }
            }
        }
    }

    private var onlineCount: Int {
        model.peers.filter(\.isOnline).count
    }

    private var coordinatorCount: Int {
        model.peers.filter { $0.role == "coordinator" }.count
    }

    private func summaryChip(_ text: String, icon: String) -> some View {
        Label(text, systemImage: icon)
            .font(.caption.weight(.medium))
            .padding(.horizontal, 10)
            .padding(.vertical, 6)
            .background(.quaternary, in: Capsule())
    }

    private func legendItem(_ text: String, color: Color) -> some View {
        HStack(spacing: 6) {
            Circle().fill(color).frame(width: 10, height: 10)
            Text(text)
        }
    }

    private func relativeTime(for date: Date) -> String {
        RelativeDateTimeFormatter().localizedString(for: date, relativeTo: .now)
    }

    private func drawBackdrop(in context: inout GraphicsContext, size: CGSize) {
        let rings: [CGFloat] = [0.18, 0.32, 0.46]
        for multiplier in rings {
            let diameter = min(size.width, size.height) * multiplier * 2
            let origin = CGPoint(x: (size.width - diameter) / 2, y: (size.height - diameter) / 2)
            let ring = Path(ellipseIn: CGRect(origin: origin, size: CGSize(width: diameter, height: diameter)))
            context.stroke(ring, with: .color(.blue.opacity(0.12)), lineWidth: 1)
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
                with: .color(edge.isActive ? .blue.opacity(0.45) : .secondary.opacity(0.22)),
                style: StrokeStyle(lineWidth: edge.isActive ? 2.4 : 1.2, dash: edge.isActive ? [] : [6, 4])
            )
        }
    }
}
