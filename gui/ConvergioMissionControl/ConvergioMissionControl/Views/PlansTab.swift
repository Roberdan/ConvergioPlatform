// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Plans tab: project hierarchy navigation

import SwiftUI

/// Plans tab: sidebar with project/plan hierarchy + detail view.
/// Replaces the embedded WebView with native NavigationSplitView.
struct PlansTab: View {
    @State private var selectedPlan: PlanItem?
    @State private var columnVisibility: NavigationSplitViewVisibility = .all

    var body: some View {
        NavigationSplitView(columnVisibility: $columnVisibility) {
            SidebarView(selectedPlan: $selectedPlan)
                .navigationTitle("Plans")
        } detail: {
            if let plan = selectedPlan {
                PlanDetailView(plan: plan)
            } else {
                PlanDetailEmptyView()
            }
        }
        .navigationSplitViewStyle(.balanced)
    }
}
