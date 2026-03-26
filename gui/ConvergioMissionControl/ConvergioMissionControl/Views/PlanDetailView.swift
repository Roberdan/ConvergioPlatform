// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Plan detail panel

import SwiftUI

/// Detail view rendered when a plan is selected in the sidebar.
/// Shows plan metadata, progress bar, and task breakdown.
struct PlanDetailView: View {
    let plan: PlanItem

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                header
                progressSection
                if !plan.subPlans.isEmpty {
                    subPlansSection
                }
                Spacer()
            }
            .padding(16)
        }
        .navigationTitle(plan.name)
    }

    // MARK: - Header

    private var header: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text(plan.name)
                    .font(.headline)
                    .lineLimit(3)
                Spacer()
                statusPill(plan.status)
            }
            HStack(spacing: 4) {
                Image(systemName: "number")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Text("Plan #\(plan.id)")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(12)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 10))
    }

    // MARK: - Progress

    private var progressSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Label("Progress", systemImage: "chart.bar.fill")
                .font(.caption.bold())
                .foregroundStyle(.secondary)

            ProgressView(value: plan.progress)
                .tint(plan.status.color)

            HStack {
                Text("\(plan.tasksDone) done")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                Spacer()
                Text("\(plan.tasksTotal - plan.tasksDone) remaining")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                Spacer()
                Text("\(Int(plan.progress * 100))%")
                    .font(.caption2.bold())
                    .foregroundStyle(plan.status.color)
            }
        }
        .padding(12)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 10))
    }

    // MARK: - Sub-Plans

    private var subPlansSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Label("Sub-Plans (\(plan.subPlans.count))", systemImage: "list.bullet.indent")
                .font(.caption.bold())
                .foregroundStyle(.secondary)

            ForEach(plan.subPlans) { sub in
                HStack(spacing: 8) {
                    Circle()
                        .fill(sub.status.color)
                        .frame(width: 6, height: 6)
                    Text(sub.name)
                        .font(.caption)
                        .lineLimit(1)
                    Spacer()
                    Text("\(sub.tasksDone)/\(sub.tasksTotal)")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                    statusPill(sub.status)
                }
                .padding(.vertical, 2)
            }
        }
        .padding(12)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 10))
    }

    // MARK: - Helpers

    private func statusPill(_ status: PlanStatus) -> some View {
        Text(status.label)
            .font(.caption2.bold())
            .padding(.horizontal, 6)
            .padding(.vertical, 3)
            .background(status.color.opacity(0.15))
            .foregroundStyle(status.color)
            .clipShape(Capsule())
    }
}

// MARK: - Empty State

/// Shown in detail area when no plan is selected in the sidebar.
struct PlanDetailEmptyView: View {
    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: "list.clipboard")
                .font(.largeTitle)
                .foregroundStyle(.tertiary)
            Text("Select a plan")
                .font(.headline)
                .foregroundStyle(.secondary)
            Text("Choose a plan from the sidebar\nto view its details.")
                .font(.caption)
                .foregroundStyle(.tertiary)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
