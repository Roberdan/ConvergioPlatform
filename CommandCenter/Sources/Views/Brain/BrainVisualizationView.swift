import SwiftUI

struct BrainVisualizationView: View {
    let notificationManager: NotificationManager
    @State private var model: BrainVisualizationModel

    init(
        client: DaemonClient = DaemonClient(),
        notificationManager: NotificationManager = .shared
    ) {
        self.notificationManager = notificationManager
        _model = State(initialValue: BrainVisualizationModel(client: client))
    }

    var body: some View {
        HSplitView {
            VStack(alignment: .leading, spacing: 18) {
                header
                graphSurface
            }
            .padding(24)

            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    if let selectedNode = model.selectedNode {
                        nodeInspector(selectedNode)
                    } else {
                        ContentUnavailableView(
                            "Select a node",
                            systemImage: "brain",
                            description: Text("Click an agent, task, or plan node in the graph to inspect live details.")
                        )
                    }
                    NotificationPreferencesView(manager: notificationManager)
                }
                .padding(24)
            }
            .frame(minWidth: 340, idealWidth: 380)
        }
        .task { await model.load() }
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
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 6) {
                    Text("Brain Visualization")
                        .font(.largeTitle.weight(.bold))
                    Text("Realtime task, plan, and agent relationships backed by /api/brain and /ws/brain.")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }

                Spacer()

                if model.isLoading {
                    ProgressView()
                }
            }

            HStack(spacing: 10) {
                chip("\(model.planCount) plans", icon: "hexagon")
                chip("\(model.taskCount) tasks", icon: "square.fill")
                chip("\(model.agentCount) agents", icon: "circle.fill")
                chip(model.shaderReady ? "Metal ready" : "Metal unavailable", icon: "cpu")
            }
        }
    }

    private var graphSurface: some View {
        ZStack {
            MetalBackdropView(isActive: model.shaderReady)
                .clipShape(RoundedRectangle(cornerRadius: 28, style: .continuous))

            GeometryReader { geometry in
                ZStack {
                    Canvas { context, _ in
                        drawEdges(in: &context)
                    }

                    ForEach(model.nodes) { node in
                        if let position = model.position(for: node.id) {
                            Button {
                                model.select(node.id)
                            } label: {
                                BrainNodeGlyph(node: node, isSelected: node.id == model.selectedNodeID)
                            }
                            .buttonStyle(.plain)
                            .position(position)
                        }
                    }
                }
                .onAppear { model.setViewport(geometry.size) }
                .onChange(of: geometry.size) { _, newSize in
                    model.setViewport(newSize)
                }
            }
        }
        .frame(minHeight: 560)
    }

    private func drawEdges(in context: inout GraphicsContext) {
        for edge in model.edges {
            guard let from = model.position(for: edge.from), let to = model.position(for: edge.to) else { continue }
            var path = Path()
            path.move(to: from)
            path.addLine(to: to)
            context.stroke(
                path,
                with: .color(edge.kind == "plan-task" ? .cyan.opacity(0.28) : .blue.opacity(0.18)),
                style: StrokeStyle(lineWidth: edge.kind == "plan-task" ? 2 : 1.2, dash: edge.kind == "assignment" ? [5, 4] : [])
            )
        }
    }

    private func nodeInspector(_ node: BrainGraphNode) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(node.title)
                .font(.title2.weight(.semibold))
            Text(node.subtitle)
                .font(.subheadline)
                .foregroundStyle(.secondary)
            Text(node.detail)
                .font(.body.monospaced())
                .textSelection(.enabled)
        }
        .padding(20)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 24, style: .continuous))
    }

    private func chip(_ text: String, icon: String) -> some View {
        Label(text, systemImage: icon)
            .font(.caption.weight(.medium))
            .padding(.horizontal, 10)
            .padding(.vertical, 6)
            .background(.quaternary, in: Capsule())
    }
}

private struct BrainNodeGlyph: View {
    let node: BrainGraphNode
    let isSelected: Bool

    var body: some View {
        VStack(spacing: 6) {
            shapeView
                .frame(width: 38, height: 38)
                .overlay(
                    Text(symbol)
                        .font(.caption.weight(.bold))
                        .foregroundStyle(.white)
                )

            Text(node.title)
                .font(.caption2.weight(.semibold))
                .foregroundStyle(.white)
                .lineLimit(2)
                .multilineTextAlignment(.center)
                .frame(width: 110)
        }
        .shadow(color: .black.opacity(0.28), radius: 10, y: 6)
    }

    @ViewBuilder
    private var shapeView: some View {
        switch node.kind {
        case .agent:
            Circle()
                .fill(fillColor)
                .overlay(Circle().stroke(isSelected ? Color.white : .clear, lineWidth: 2))
        case .task:
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .fill(fillColor)
                .overlay(
                    RoundedRectangle(cornerRadius: 10, style: .continuous)
                        .stroke(isSelected ? Color.white : .clear, lineWidth: 2)
                )
        case .plan:
            HexagonShape()
                .fill(fillColor)
                .overlay(HexagonShape().stroke(isSelected ? Color.white : .clear, lineWidth: 2))
        }
    }

    private var fillColor: Color {
        switch node.kind {
        case .agent: return .blue
        case .task: return .green
        case .plan: return .purple
        }
    }

    private var symbol: String {
        switch node.kind {
        case .agent: return "A"
        case .task: return "T"
        case .plan: return "P"
        }
    }
}

private struct HexagonShape: InsettableShape {
    var insetAmount: CGFloat = 0

    func path(in rect: CGRect) -> Path {
        let rect = rect.insetBy(dx: insetAmount, dy: insetAmount)
        let points = [
            CGPoint(x: rect.midX, y: rect.minY),
            CGPoint(x: rect.maxX, y: rect.minY + rect.height * 0.25),
            CGPoint(x: rect.maxX, y: rect.minY + rect.height * 0.75),
            CGPoint(x: rect.midX, y: rect.maxY),
            CGPoint(x: rect.minX, y: rect.minY + rect.height * 0.75),
            CGPoint(x: rect.minX, y: rect.minY + rect.height * 0.25),
        ]
        var path = Path()
        path.addLines(points)
        path.closeSubpath()
        return path
    }

    func inset(by amount: CGFloat) -> some InsettableShape {
        var shape = self
        shape.insetAmount += amount
        return shape
    }
}
