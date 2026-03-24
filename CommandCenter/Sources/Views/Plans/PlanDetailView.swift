import Charts
import SwiftUI

struct PlanDetailView: View {
    let plan: PlanListItem
    let context: PlanContextResponse
    let chartData: [WaveProgressDatum]
    let onSelectTask: (PlanContextTask) -> Void
    let onMoveTask: (Int, String) -> Void

    private let columns = ["pending", "in_progress", "submitted", "done"]

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                Text(plan.name)
                    .font(.largeTitle.weight(.bold))

                Chart(chartData) { datum in
                    BarMark(
                        x: .value("Wave", datum.wave),
                        y: .value("Completion", datum.completion * 100)
                    )
                }
                .frame(height: 220)

                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 12) {
                        ForEach(context.waves) { wave in
                            VStack(alignment: .leading, spacing: 6) {
                                Text(wave.waveLabel)
                                    .font(.headline)
                                Text(wave.name)
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                Text(wave.status.capitalized)
                                    .font(.caption2)
                                    .padding(.horizontal, 8)
                                    .padding(.vertical, 4)
                                    .background(.quaternary, in: Capsule())
                            }
                            .padding(14)
                            .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 18, style: .continuous))
                        }
                    }
                }

                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(alignment: .top, spacing: 16) {
                        ForEach(columns, id: \.self) { status in
                            column(title: status.replacingOccurrences(of: "_", with: " ").capitalized, status: status)
                        }
                    }
                }
            }
            .padding(24)
        }
    }

    private func column(title: String, status: String) -> some View {
        let tasks = context.tasks.filter { $0.status == status }
        return VStack(alignment: .leading, spacing: 12) {
            Text("\(title) (\(tasks.count))")
                .font(.headline)

            ForEach(tasks) { task in
                VStack(alignment: .leading, spacing: 8) {
                    Text(task.taskId ?? "#\(task.id)")
                        .font(.caption.monospaced())
                        .foregroundStyle(.secondary)
                    Text(task.title)
                        .font(.subheadline.weight(.medium))
                        .multilineTextAlignment(.leading)
                }
                .padding(12)
                .frame(width: 260, alignment: .leading)
                .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
                .contentShape(Rectangle())
                .onTapGesture { onSelectTask(task) }
                .draggable(String(task.id))
            }
        }
        .frame(width: 280, alignment: .topLeading)
        .padding(14)
        .background(.quaternary.opacity(0.12), in: RoundedRectangle(cornerRadius: 18, style: .continuous))
        .dropDestination(for: String.self) { items, _ in
            guard let raw = items.first, let taskID = Int(raw) else { return false }
            onMoveTask(taskID, status)
            return true
        }
    }
}
