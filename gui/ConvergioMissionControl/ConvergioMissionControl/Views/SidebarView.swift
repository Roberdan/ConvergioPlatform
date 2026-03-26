// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Project hierarchy sidebar navigation

import SwiftUI

/// NavigationSplitView sidebar showing Projects -> Master Plans -> Sub-Plans.
/// Selection drives the detail area via `selectedPlan` binding.
struct SidebarView: View {
    @Binding var selectedPlan: PlanItem?
    @State private var viewModel = PlanViewModel()
    @State private var expandedProjects: Set<String> = []

    var body: some View {
        List(selection: $selectedPlan) {
            if viewModel.isLoading && viewModel.projects.isEmpty {
                loadingRow
            } else if let error = viewModel.errorMessage {
                errorRow(error)
            } else if viewModel.projects.isEmpty {
                emptyRow
            } else {
                projectList
            }
        }
        .listStyle(.sidebar)
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button { Task { await viewModel.fetchPlans() } } label: {
                    Image(systemName: "arrow.clockwise")
                }
                .help("Refresh plans")
            }
        }
        .onAppear { viewModel.startRefreshing() }
        .onDisappear { viewModel.stopRefreshing() }
    }

    // MARK: - Project List

    private var projectList: some View {
        ForEach(viewModel.projects) { project in
            Section {
                ForEach(project.plans) { plan in
                    planRow(plan)
                }
            } header: {
                Label(project.name, systemImage: "folder")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }
        }
    }

    // MARK: - Plan Row

    private func planRow(_ plan: PlanItem) -> some View {
        HStack(spacing: 6) {
            VStack(alignment: .leading, spacing: 2) {
                Text(plan.name)
                    .font(.caption)
                    .lineLimit(2)
                progressLabel(plan)
            }
            Spacer(minLength: 4)
            statusBadge(plan.status)
        }
        .tag(plan)
        .padding(.vertical, 2)
    }

    private func progressLabel(_ plan: PlanItem) -> some View {
        Group {
            if plan.tasksTotal > 0 {
                Text("\(plan.tasksDone)/\(plan.tasksTotal) tasks")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
        }
    }

    private func statusBadge(_ status: PlanStatus) -> some View {
        Text(status.label)
            .font(.caption2.bold())
            .padding(.horizontal, 5)
            .padding(.vertical, 2)
            .background(status.color.opacity(0.15))
            .foregroundStyle(status.color)
            .clipShape(RoundedRectangle(cornerRadius: 4))
    }

    // MARK: - States

    private var loadingRow: some View {
        HStack(spacing: 8) {
            ProgressView().scaleEffect(0.7)
            Text("Loading plans...").foregroundStyle(.secondary).font(.caption)
        }
        .listRowBackground(Color.clear)
    }

    private func errorRow(_ message: String) -> some View {
        Label(message, systemImage: "exclamationmark.triangle")
            .foregroundStyle(.red)
            .font(.caption)
            .listRowBackground(Color.clear)
    }

    private var emptyRow: some View {
        Text("No plans found")
            .foregroundStyle(.secondary)
            .font(.caption)
            .listRowBackground(Color.clear)
    }
}
