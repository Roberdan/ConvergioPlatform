import Charts
import SwiftUI

struct PlanDetailView: View {
    let plan: PlanListItem
    let context: PlanContextResponse
    let chartData: [WaveProgressDatum]
    let onSelectTask: (PlanContextTask) -> Void
    let onMoveTask: (Int, String) -> Void

    @State private var dropTargetStatus: String? = nil

    private let columns = ["pending", "in_progress", "submitted", "done"]

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: Spacing.lg) {
                Text(plan.name)
                    .font(.heroTitle)
                    .foregroundStyle(ConvergioTokens.Text.textPrimary)

                Chart(chartData) { datum in
                    BarMark(
                        x: .value("Wave", datum.wave),
                        y: .value("Completion", datum.completion * 100)
                    )
                    .foregroundStyle(ConvergioTokens.Brand.azzurro)
                }
                .frame(height: 220)
                .accessibilityLabel(chartAccessibilityLabel)

                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: Spacing.sm) {
                        ForEach(context.waves) { wave in
                            wavePill(wave)
                        }
                    }
                }

                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(alignment: .top, spacing: Spacing.md) {
                        ForEach(columns, id: \.self) { status in
                            kanbanColumn(status: status)
                        }
                    }
                }
            }
            .padding(Spacing.lg)
        }
    }

    // MARK: - Accessibility helpers

    private var chartAccessibilityLabel: String {
        let count = chartData.count
        let sum = chartData.map(\.completion).reduce(0, +)
        let avg = count > 0 ? Int((sum / Double(count)) * 100) : 0
        return "Wave completion chart: \(count) waves, average \(avg) percent complete"
    }

    // MARK: - Wave pill

    private func wavePill(_ wave: PlanContextWave) -> some View {
        VStack(alignment: .leading, spacing: Spacing.xxs) {
            Text(wave.waveLabel)
                .font(.cardTitle)
                .foregroundStyle(ConvergioTokens.Text.textPrimary)
            Text(wave.name)
                .font(.label)
                .foregroundStyle(ConvergioTokens.Text.textMuted)
            StatusBadge(
                label: wave.status.capitalized,
                color: waveStatusColor(wave.status)
            )
            .accessibilityLabel("Status: \(wave.status.capitalized)")
        }
        .convergioCard()
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(wave.waveLabel) \(wave.name), status \(wave.status)")
        // 3 px left border with wave status color
        .overlay(alignment: .leading) {
            RoundedRectangle(cornerRadius: CornerRadius.lg, style: .continuous)
                .fill(waveStatusColor(wave.status))
                .frame(width: 3)
                .padding(.vertical, 1)
                .accessibilityHidden(true)
        }
    }

    private func waveStatusColor(_ status: String) -> Color {
        switch status.lowercased() {
        case "in_progress", "active":
            return ConvergioTokens.Brand.azzurro
        case "done", "completed", "merged":
            return ConvergioTokens.Brand.verdeRacing
        case "blocked":
            return ConvergioTokens.Brand.rossoCorsa
        default:
            // pending, draft, todo
            return ConvergioTokens.Text.textMuted
        }
    }

    // MARK: - Kanban column

    private func kanbanColumn(status: String) -> some View {
        let tasks = context.tasks.filter { $0.status == status }
        let isTarget = dropTargetStatus == status
        return VStack(alignment: .leading, spacing: Spacing.sm) {
            columnHeader(status: status, count: tasks.count)
            ForEach(tasks) { task in
                taskCard(task)
            }
        }
        .frame(width: 280, alignment: .topLeading)
        .padding(Spacing.sm)
        .background(ConvergioTokens.Surface.surface.opacity(0.5),
                    in: RoundedRectangle(cornerRadius: CornerRadius.md, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: CornerRadius.md)
                .stroke(ConvergioTokens.Brand.gialloFerrari, lineWidth: 2)
                .opacity(isTarget ? 1 : 0)
        )
        .dropDestination(for: String.self) { items, _ in
            guard let raw = items.first, let taskID = Int(raw) else { return false }
            onMoveTask(taskID, status)
            return true
        } isTargeted: { active in
            dropTargetStatus = active ? status : nil
        }
    }

    // MARK: - Column header

    private func columnHeader(status: String, count: Int) -> some View {
        let bg = columnHeaderColor(status)
        let label = status.replacingOccurrences(of: "_", with: " ").capitalized
        return Text("\(label) (\(count))")
            .font(.cardTitle)
            .foregroundStyle(ConvergioTokens.Text.textPrimary)
            .padding(.horizontal, Spacing.sm)
            .padding(.vertical, Spacing.xxs)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(bg, in: RoundedRectangle(cornerRadius: CornerRadius.sm, style: .continuous))
    }

    private func columnHeaderColor(_ status: String) -> Color {
        switch status {
        case "pending":     return ConvergioTokens.Text.textMuted.opacity(0.25)
        case "in_progress": return ConvergioTokens.Brand.azzurro.opacity(0.25)
        case "submitted":   return ConvergioTokens.Brand.arancioWarm.opacity(0.25)
        case "done":        return ConvergioTokens.Brand.verdeRacing.opacity(0.25)
        default:            return ConvergioTokens.Surface.surface
        }
    }

    // MARK: - Task card

    private func taskCard(_ task: PlanContextTask) -> some View {
        VStack(alignment: .leading, spacing: Spacing.xs) {
            Text(task.taskId ?? "#\(task.id)")
                .font(.mono)
                .foregroundStyle(ConvergioTokens.Text.textMuted)

            Text(task.title)
                .font(.label)
                .foregroundStyle(ConvergioTokens.Text.textPrimary)
                .multilineTextAlignment(.leading)

            HStack(spacing: Spacing.xxs) {
                if let effort = task.effort {
                    StatusBadge(label: "E\(effort)", color: ConvergioTokens.Status.info)
                        .accessibilityLabel("Effort: \(effort)")
                }
                if let model = task.model {
                    StatusBadge(label: model.lowercased(), color: modelBadgeColor(model))
                        .accessibilityLabel("Model: \(model)")
                }
                Spacer()
                if let agent = task.executorAgent {
                    Text(agent)
                        .font(.label)
                        .foregroundStyle(ConvergioTokens.Text.textPrimary)
                        .padding(.horizontal, Spacing.xs)
                        .padding(.vertical, Spacing.xxs)
                        .background(agentCapsuleColor(agent), in: Capsule())
                        .accessibilityLabel("Agent: \(agent)")
                }
            }
        }
        .frame(width: 252, alignment: .leading)
        .modifier(ConvergioCardModifier())
        .contentShape(Rectangle())
        .accessibilityElement(children: .combine)
        .accessibilityLabel(
            "Task \(task.taskId ?? "#\(task.id)"), \(task.title)" +
            (task.executorAgent.map { ", assigned to \($0)" } ?? "")
        )
        .accessibilityAddTraits(.isButton)
        .onTapGesture { onSelectTask(task) }
        .draggable(String(task.id))
    }

    // MARK: - Badge helpers

    private func modelBadgeColor(_ model: String) -> Color {
        let lowered = model.lowercased()
        if lowered.contains("opus")   { return ConvergioTokens.Brand.gialloFerrari }
        if lowered.contains("sonnet") { return ConvergioTokens.Text.textMuted }
        if lowered.contains("haiku")  { return ConvergioTokens.Brand.arancioWarm }
        return ConvergioTokens.Surface.surfaceRaised
    }

    private func agentCapsuleColor(_ agent: String) -> Color {
        // Stable hash → role color so each agent name gets a consistent color
        let index = abs(agent.hashValue) % 12 + 1
        return ConvergioTokens.Roles.color(for: index).opacity(0.35)
    }
}
