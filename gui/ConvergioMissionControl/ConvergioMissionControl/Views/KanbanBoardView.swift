// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Plan kanban board view

import SwiftUI

private let kanbanColumnWidth: CGFloat = 220

/// Horizontal kanban board showing plan tasks grouped by status.
/// Columns: Pending | In Progress | Done.
struct KanbanBoardView: View {
    let planId: Int
    @State private var viewModel: KanbanViewModel

    init(planId: Int) {
        self.planId = planId
        _viewModel = State(wrappedValue: KanbanViewModel(planId: planId))
    }

    var body: some View {
        Group {
            if viewModel.isLoading && viewModel.tasks.isEmpty {
                loadingView
            } else if let error = viewModel.errorMessage {
                errorView(error)
            } else {
                board
            }
        }
        .onAppear { viewModel.start() }
        .onDisappear { viewModel.stop() }
    }

    // MARK: - Board

    private var board: some View {
        ScrollView(.horizontal, showsIndicators: true) {
            HStack(alignment: .top, spacing: 12) {
                columnView(
                    title: KanbanColumn.pending.rawValue,
                    tasks: viewModel.pendingTasks,
                    column: .pending
                )
                columnView(
                    title: KanbanColumn.inProgress.rawValue,
                    tasks: viewModel.inProgressTasks,
                    column: .inProgress
                )
                columnView(
                    title: KanbanColumn.done.rawValue,
                    tasks: viewModel.doneTasks,
                    column: .done
                )
            }
            .padding(12)
        }
        .accessibilityLabel("Kanban board for plan \(planId)")
    }

    // MARK: - Column

    private func columnView(
        title: String,
        tasks: [DaemonTask],
        column: KanbanColumn
    ) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            columnHeader(title: title, count: tasks.count, column: column)
            ScrollView(.vertical, showsIndicators: false) {
                LazyVStack(spacing: 8) {
                    ForEach(tasks) { task in
                        KanbanCardView(task: task)
                            .frame(width: kanbanColumnWidth)
                    }
                    if tasks.isEmpty {
                        emptyColumnPlaceholder
                    }
                }
            }
        }
        .frame(width: kanbanColumnWidth)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("\(title) column, \(tasks.count) tasks")
    }

    private func columnHeader(title: String, count: Int, column: KanbanColumn) -> some View {
        HStack(spacing: 6) {
            Circle()
                .fill(columnAccentColor(column))
                .frame(width: 8, height: 8)
            Text(title)
                .font(.caption.bold())
                .foregroundStyle(.primary)
            Spacer()
            Text("\(count)")
                .font(.caption2.bold())
                .padding(.horizontal, 6)
                .padding(.vertical, 2)
                .background(.quaternary, in: Capsule())
                .foregroundStyle(.secondary)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 6)
        .background(.quinary, in: RoundedRectangle(cornerRadius: 6))
        .accessibilityHidden(true)
    }

    private var emptyColumnPlaceholder: some View {
        Text("No tasks")
            .font(.caption)
            .foregroundStyle(.tertiary)
            .frame(maxWidth: .infinity)
            .padding(.vertical, 20)
    }

    // MARK: - States

    private var loadingView: some View {
        VStack(spacing: 8) {
            ProgressView()
            Text("Loading tasks...")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .accessibilityLabel("Loading kanban board")
    }

    private func errorView(_ message: String) -> some View {
        VStack(spacing: 8) {
            Image(systemName: "exclamationmark.triangle")
                .font(.title2)
                .foregroundStyle(.orange)
            Text("Could not load tasks")
                .font(.subheadline.bold())
            Text(message)
                .font(.caption)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
        }
        .padding()
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .accessibilityLabel("Error loading tasks: \(message)")
    }

    // MARK: - Helpers

    private func columnAccentColor(_ column: KanbanColumn) -> Color {
        switch column {
        case .pending: return .gray
        case .inProgress: return .blue
        case .done: return .green
        }
    }
}
