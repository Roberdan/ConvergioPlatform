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
                            description: Text("Click a node in the graph to inspect live details.")
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

    // MARK: - Header

    private var header: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 6) {
                    Text("Brain Visualization")
                        .font(.largeTitle.weight(.bold))
                    Text("Realtime task, plan, and agent relationships.")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                if model.isLoading {
                    ProgressView()
                        .accessibilityLabel("Loading brain data")
                }
            }

            HStack(spacing: 8) {
                headerChip("\(model.planCount) plans", icon: "folder")
                headerChip("\(model.taskCount) tasks", icon: "checklist")
                headerChip("\(model.agentCount) agents", icon: "person.3")
                headerChip(
                    model.shaderReady ? "Metal ready" : "Metal unavailable",
                    icon: "cpu"
                )
            }
        }
    }

    /// Glass pill chip for header metrics — premium look with glass background.
    private func headerChip(_ text: String, icon: String) -> some View {
        HStack(spacing: 4) {
            Image(systemName: icon)
                .font(.caption2)
                .foregroundStyle(ConvergioTokens.Text.textMuted)
            Text(text)
                .font(.label)
                .foregroundStyle(ConvergioTokens.Text.textPrimary)
        }
        .padding(.horizontal, Spacing.xs)
        .padding(.vertical, Spacing.xxs)
        .background(.ultraThinMaterial, in: Capsule())
        .overlay(Capsule().strokeBorder(ConvergioTokens.Border.borderSubtle, lineWidth: 1))
    }

    // MARK: - Graph surface

    private var graphSurface: some View {
        ZStack {
            MetalBackdropView(isActive: model.shaderReady)
                .clipShape(RoundedRectangle(cornerRadius: 28, style: .continuous))
                .accessibilityHidden(true)

            GeometryReader { geometry in
                ZStack {
                    Canvas { context, _ in
                        drawEdges(in: &context)
                    }
                    .accessibilityHidden(true)

                    ForEach(model.nodes) { node in
                        if let position = model.position(for: node.id) {
                            Button {
                                model.select(node.id)
                            } label: {
                                BrainNodeGlyph(node: node, isSelected: node.id == model.selectedNodeID)
                            }
                            .buttonStyle(.plain)
                            .position(position)
                            .accessibilityLabel(
                                "Select \(node.kind.rawValue) \(node.title)"
                                + (node.id == model.selectedNodeID ? ", selected" : "")
                            )
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
        .overlay(
            RoundedRectangle(cornerRadius: 28, style: .continuous)
                .strokeBorder(ConvergioTokens.Border.border.opacity(0.4), lineWidth: 1)
        )
        .accessibilityLabel(
            "Brain graph: \(model.planCount) plans, \(model.taskCount) tasks, \(model.agentCount) agents"
        )
    }

    // MARK: - Edge rendering

    private func drawEdges(in context: inout GraphicsContext) {
        for edge in model.edges {
            guard
                let from = model.position(for: edge.from),
                let to = model.position(for: edge.to)
            else { continue }
            let sourceColor = model.nodes.first { $0.id == edge.from }?.color ?? ConvergioTokens.Brand.azzurro
            let targetColor = model.nodes.first { $0.id == edge.to }?.color ?? ConvergioTokens.Brand.verdeRacing
            // Higher opacity edges for visibility; glow layer underneath
            var glowPath = Path()
            glowPath.move(to: from)
            glowPath.addLine(to: to)
            let glowGrad = Gradient(colors: [sourceColor.opacity(0.35), targetColor.opacity(0.25)])
            context.stroke(
                glowPath,
                with: .linearGradient(glowGrad, startPoint: from, endPoint: to),
                style: StrokeStyle(lineWidth: edge.kind == "plan-task" ? 6 : 4)
            )
            var path = Path()
            path.move(to: from)
            path.addLine(to: to)
            let gradient = Gradient(colors: [sourceColor.opacity(0.8), targetColor.opacity(0.55)])
            let lineWidth: CGFloat = edge.kind == "plan-task" ? 2 : 1.2
            let dash: [CGFloat] = edge.kind == "assignment" ? [5, 4] : []
            context.stroke(
                path,
                with: .linearGradient(gradient, startPoint: from, endPoint: to),
                style: StrokeStyle(lineWidth: lineWidth, dash: dash)
            )
        }
    }

    // MARK: - Node inspector

    private func nodeInspector(_ node: BrainGraphNode) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            // Accent header bar matching node color
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text(node.title)
                    .font(.title2.weight(.semibold))
                    .foregroundStyle(ConvergioTokens.Text.textPrimary)
                StatusBadge(label: node.kind.rawValue.capitalized, color: node.color)
            }
            .padding(Spacing.md)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(node.color.opacity(0.15))

            VStack(alignment: .leading, spacing: 10) {
                Text(node.subtitle)
                    .font(.subheadline)
                    .foregroundStyle(ConvergioTokens.Text.textMuted)
                Text(node.detail)
                    .font(.body.monospaced())
                    .foregroundStyle(ConvergioTokens.Text.textPrimary)
                    .textSelection(.enabled)
            }
            .padding(Spacing.md)
        }
        .background(.ultraThinMaterial)
        .clipShape(RoundedRectangle(cornerRadius: CornerRadius.lg, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: CornerRadius.lg, style: .continuous)
                .strokeBorder(node.color.opacity(0.3), lineWidth: 1)
        )
        .shadow(color: node.color.opacity(0.15), radius: 12, y: 4)
    }
}
