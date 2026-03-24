import SwiftUI

struct TaskDetailView: View {
    let task: PlanContextTask

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                Text(task.taskId ?? "#\(task.id)")
                    .font(.caption.monospaced())
                    .foregroundStyle(.secondary)

                Text(task.title)
                    .font(.title2.weight(.semibold))

                detailRow("Status", task.status)
                detailRow("Executor", task.executorAgent ?? "Unassigned")
                detailRow("Model", task.model ?? "Not set")
                detailRow("Effort", task.effort.map(String.init) ?? "—")
                detailRow("Type", task.type ?? "—")

                if let testCriteria = task.testCriteria {
                    Text("Test criteria")
                        .font(.headline)
                    Text(testCriteria)
                        .foregroundStyle(.secondary)
                }

                if let verify = task.verify, !verify.isEmpty {
                    Text("Verify commands")
                        .font(.headline)
                    ForEach(verify, id: \.self) { command in
                        Text(command)
                            .font(.system(.body, design: .monospaced))
                    }
                }
            }
            .padding(24)
        }
        .frame(minWidth: 520, minHeight: 420)
    }

    private func detailRow(_ title: String, _ value: String) -> some View {
        HStack {
            Text(title)
                .font(.headline)
            Spacer()
            Text(value)
                .foregroundStyle(.secondary)
        }
    }
}
