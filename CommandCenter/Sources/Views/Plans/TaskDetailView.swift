import SwiftUI

struct TaskDetailView: View {
    let task: PlanContextTask

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: Spacing.md) {
                // Task ID — monospaced for quick scanning
                Text(task.taskId ?? "#\(task.id)")
                    .font(.mono)
                    .foregroundStyle(ConvergioTokens.Text.textMuted)

                Text(task.title)
                    .font(.sectionTitle)
                    .foregroundStyle(ConvergioTokens.Text.textPrimary)

                // Status badge row
                HStack(spacing: Spacing.xs) {
                    StatusBadge(label: task.status.replacingOccurrences(of: "_", with: " ").capitalized,
                                color: statusColor(task.status))
                    if let effort = task.effort {
                        StatusBadge(label: "Effort \(effort)", color: ConvergioTokens.Status.info)
                    }
                }

                Divider()
                    .overlay(ConvergioTokens.Border.border)

                detailRow("Executor", task.executorAgent ?? "Unassigned")
                detailRow("Model",    task.model ?? "Not set")
                detailRow("Type",     task.type ?? "—")

                if let testCriteria = task.testCriteria {
                    sectionLabel("Test criteria")
                    Text(testCriteria)
                        .font(.bodyMedium)
                        .foregroundStyle(ConvergioTokens.Text.textMuted)
                }

                if let verify = task.verify, !verify.isEmpty {
                    sectionLabel("Verify commands")
                    ForEach(verify, id: \.self) { command in
                        Text(command)
                            .font(.mono)
                            .foregroundStyle(ConvergioTokens.Text.textPrimary)
                            .padding(Spacing.xs)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .background(ConvergioTokens.Surface.surfaceSunken,
                                        in: RoundedRectangle(cornerRadius: CornerRadius.sm, style: .continuous))
                    }
                }
            }
            .padding(Spacing.lg)
            .modifier(ConvergioCardModifier())
            .padding(Spacing.lg)
        }
        .frame(minWidth: 520, minHeight: 420)
    }

    // MARK: - Subviews

    private func sectionLabel(_ title: String) -> some View {
        Text(title)
            .font(.cardTitle)
            .foregroundStyle(ConvergioTokens.Text.textPrimary)
    }

    private func detailRow(_ title: String, _ value: String) -> some View {
        HStack {
            Text(title)
                .font(.cardTitle)
                .foregroundStyle(ConvergioTokens.Text.textPrimary)
            Spacer()
            Text(value)
                .font(.bodyMedium)
                .foregroundStyle(ConvergioTokens.Text.textMuted)
        }
    }

    // MARK: - Status color mapping

    private func statusColor(_ status: String) -> Color {
        switch status {
        case "done":        return ConvergioTokens.Status.success
        case "in_progress": return ConvergioTokens.Status.info
        case "submitted":   return ConvergioTokens.Brand.arancioWarm
        case "blocked":     return ConvergioTokens.Status.error
        default:            return ConvergioTokens.Text.textMuted
        }
    }
}
