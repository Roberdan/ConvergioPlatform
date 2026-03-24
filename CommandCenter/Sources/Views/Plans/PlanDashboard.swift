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
                    .foregroundStyle(.red)
                    .padding(12)
            }
        }
    }

    private func planCard(_ plan: PlanListItem) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text(plan.name)
                        .font(.headline)
                    Text(plan.status.capitalized)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Text("#\(plan.id)")
                    .font(.caption.monospacedDigit())
                    .foregroundStyle(.secondary)
            }

            ProgressView(value: plan.mergeProgress)

            HStack {
                Label("\(plan.wavesMerged ?? 0)/\(plan.wavesTotal ?? 0) merged", systemImage: "arrow.merge")
                Spacer()
                Label("\(plan.tasksDone ?? 0)/\(plan.tasksTotal ?? 0)", systemImage: "checkmark.circle")
            }
            .font(.caption)
            .foregroundStyle(.secondary)
        }
        .padding(18)
        .frame(maxWidth: .infinity, alignment: .leading)
        .modifier(PlanCardGlassModifier())
    }
}

private struct PlanCardGlassModifier: ViewModifier {
    func body(content: Content) -> some View {
        if #available(macOS 26.0, *) {
            content.glassEffect()
        } else {
            content.background(.regularMaterial, in: RoundedRectangle(cornerRadius: 24, style: .continuous))
        }
    }
}
