// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Plans tab: project hierarchy navigation

import SwiftUI

/// Plans tab: sidebar with project/plan hierarchy + detail view.
/// Replaces the embedded WebView with native NavigationSplitView.
struct PlansTab: View {
    @State private var selectedPlan: PlanItem?
    @State private var columnVisibility: NavigationSplitViewVisibility = .all
    @State private var showKanban = false

    var body: some View {
        NavigationSplitView(columnVisibility: $columnVisibility) {
            SidebarView(selectedPlan: $selectedPlan)
                .navigationTitle("Plans")
        } detail: {
            if let plan = selectedPlan {
                detailContent(for: plan)
            } else {
                PlanDetailEmptyView()
            }
        }
        .navigationSplitViewStyle(.balanced)
    }

    @ViewBuilder
    private func detailContent(for plan: PlanItem) -> some View {
        VStack(spacing: 0) {
            Picker("View", selection: $showKanban) {
                Text("Overview").tag(false)
                Text("Kanban").tag(true)
            }
            .pickerStyle(.segmented)
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
            .accessibilityLabel("Switch between overview and kanban views")

            if showKanban {
                KanbanBoardView(planId: plan.id)
            } else {
                PlanDetailView(plan: plan)
            }
        }
    }
}
