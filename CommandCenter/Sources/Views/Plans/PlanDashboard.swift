import Observation
import SwiftUI

struct PlanDashboard: View {
    @State private var model: PlanDashboardModel
    @State private var selectedTask: PlanContextTask?

    init(client: DaemonClient = DaemonClient()) {
        _model = State(initialValue: PlanDashboardModel(client: client))
    }

    @State private var hoveredPlanID: Int?

    var body: some View {
        HSplitView {
            ScrollView {
                LazyVStack(alignment: .leading, spacing: Spacing.lg) {
                    Text("Plans")
                        .sectionHeader()
                        .padding(.bottom, Spacing.xs)
                    ForEach(model.plans) { plan in
                        Button {
                            Task { await model.selectPlan(plan.id) }
                        } label: {
                            planCard(plan)
                                .scaleEffect(hoveredPlanID == plan.id ? 1.01 : 1.0)
                                .animation(.easeOut(duration: 0.15), value: hoveredPlanID)
                        }
                        .buttonStyle(.plain)
                        .onHover { inside in hoveredPlanID = inside ? plan.id : nil }
                        .accessibilityLabel("Plan \(plan.name), status \(plan.status)")
                    }
                }
                .padding(Spacing.lg)
            }
            .frame(minWidth: 320, idealWidth: 360, maxWidth: 420)

            if let context = model.selectedContext,
               let plan = model.plans.first(where: { $0.id == context.plan.id }) {
                PlanDetailView(
                    plan: plan,
                    context: context,
                    chartData: model.chartData(),
                    onSelectTask: { selectedTask = $0 },
                    onMoveTask: { taskID, status in
                        Task { await model.moveTask(taskID, to: status) }
                    }
                )
            } else {
                ContentUnavailableView("Select a plan", systemImage: "list.bullet.rectangle")
            }
        }
        .task { await model.load() }
        .sheet(item: $selectedTask) { task in
            TaskDetailView(task: task)
        }
        .overlay(alignment: .topTrailing) {
            if let errorMessage = model.errorMessage {
                Text(errorMessage)
                    .font(.footnote)
                    .foregroundStyle(ConvergioTokens.Status.error)
                    .padding(12)
            }
        }
    }

    // MARK: - Plan card

    private func planCard(_ plan: PlanListItem) -> some View {
        let done = plan.tasksDone ?? 0
        let total = plan.tasksTotal ?? 0
        let pending = total - done
        return VStack(alignment: .leading, spacing: 12) {
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text(plan.name)
                        .font(.headline)
                        .foregroundStyle(ConvergioTokens.Text.textPrimary)
                    Text(plan.status.capitalized)
                        .font(.caption)
                        .foregroundStyle(ConvergioTokens.Text.textMuted)
                }
                Spacer()
                Text("#\(plan.id)")
                    .font(.caption.monospacedDigit())
                    .foregroundStyle(ConvergioTokens.Text.textMuted)
            }

            // Custom progress bar with glow
            GeometryReader { geo in
                let w = geo.size.width * CGFloat(plan.mergeProgress)
                ZStack(alignment: .leading) {
                    Capsule().fill(ConvergioTokens.Border.border).frame(height: 6)
                    Capsule()
                        .fill(ConvergioTokens.Brand.gialloFerrari)
                        .frame(width: max(w, 0), height: 6)
                        .shadow(color: ConvergioTokens.Brand.gialloFerrari.opacity(0.5), radius: 6, y: 1)
                }
            }
            .frame(height: 6)
            .accessibilityLabel("Merge progress \(Int((plan.mergeProgress) * 100)) percent")

            HStack {
                Label("\(plan.wavesMerged ?? 0)/\(plan.wavesTotal ?? 0) merged",
                      systemImage: "arrow.merge")
                    .foregroundStyle(ConvergioTokens.Text.textMuted)
                Spacer()
                HStack(spacing: 6) {
                    if done > 0 {
                        StatusBadge(
                            label: "\(done) done",
                            color: ConvergioTokens.Status.success
                        )
                        .accessibilityLabel("Status: \(done) tasks done")
                    }
                    if pending > 0 {
                        StatusBadge(
                            label: "\(pending) left",
                            color: ConvergioTokens.Text.textMuted
                        )
                        .accessibilityLabel("Status: \(pending) tasks remaining")
                    }
                }
            }
            .font(.caption)
        }
        .padding(18)
        .frame(maxWidth: .infinity, alignment: .leading)
        .modifier(ConvergioCardModifier())
        .shadow(color: .black.opacity(0.3), radius: 12, y: 4)
        .accessibilityElement(children: .combine)
        .accessibilityLabel(
            "Plan \(plan.name), status \(plan.status), " +
            "\(plan.wavesMerged ?? 0) of \(plan.wavesTotal ?? 0) waves merged, " +
            "\(done) tasks done, \(pending) remaining"
        )
        // Gradient left status border
        .overlay(alignment: .leading) {
            RoundedRectangle(cornerRadius: CornerRadius.lg, style: .continuous)
                .fill(
                    LinearGradient(
                        colors: [planStatusColor(plan.status), planStatusColor(plan.status).opacity(0.1)],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                )
                .frame(width: 3)
                .padding(.vertical, 1)
                .accessibilityHidden(true)
        }
    }

    // MARK: - Helpers

    private func planStatusColor(_ status: String) -> Color {
        switch status.lowercased() {
        case "doing", "in_progress", "active":
            return ConvergioTokens.Brand.azzurro
        case "done", "completed", "merged":
            return ConvergioTokens.Brand.verdeRacing
        default:
            // draft, todo, pending, etc.
            return ConvergioTokens.Text.textMuted
        }
    }
}
