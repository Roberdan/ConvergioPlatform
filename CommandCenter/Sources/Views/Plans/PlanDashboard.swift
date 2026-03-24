import Observation
import SwiftUI

struct PlanDashboard: View {
    @State private var model: PlanDashboardModel
    @State private var selectedTask: PlanContextTask?

    init(client: DaemonClient = DaemonClient()) {
        _model = State(initialValue: PlanDashboardModel(client: client))
    }

    var body: some View {
        HSplitView {
            ScrollView {
                LazyVStack(spacing: 16) {
                    ForEach(model.plans) { plan in
                        Button {
                            Task { await model.selectPlan(plan.id) }
                        } label: {
                            planCard(plan)
                        }
                        .buttonStyle(.plain)
                        .accessibilityLabel("Plan \(plan.name), status \(plan.status)")
                    }
                }
                .padding(24)
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

            ProgressView(value: plan.mergeProgress)
                .tint(ConvergioTokens.Brand.gialloFerrari)
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
        .accessibilityElement(children: .combine)
        .accessibilityLabel(
            "Plan \(plan.name), status \(plan.status), " +
            "\(plan.wavesMerged ?? 0) of \(plan.wavesTotal ?? 0) waves merged, " +
            "\(done) tasks done, \(pending) remaining"
        )
        // 3 px left status border overlaid on top of the card shape
        .overlay(alignment: .leading) {
            RoundedRectangle(cornerRadius: CornerRadius.lg, style: .continuous)
                .fill(planStatusColor(plan.status))
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
