import Observation
import SwiftUI

struct ContentView: View {
    @Bindable var model: CommandCenterAppModel
    var notificationManager: NotificationManager = .shared

    var body: some View {
        NavigationSplitView {
            sidebar
                .navigationSplitViewColumnWidth(min: 260, ideal: 300, max: 340)
        } detail: {
            detailView
                .toolbar { toolbarStatus }
        }
        .task {
            await notificationManager.startIfNeeded()
        }
    }

    private var sidebar: some View {
        Group {
            if #available(macOS 26.0, *) {
                listContent
                    .glassEffect()
            } else {
                listContent
                    .background(.thinMaterial)
            }
        }
    }

    private var listContent: some View {
        List(SidebarItem.allCases, selection: $model.selection) { item in
            Label(item.title, systemImage: item.icon)
                .tag(Optional(item))
        }
        .listStyle(.sidebar)
        .navigationTitle("Command Center")
    }

    @ToolbarContentBuilder
    private var toolbarStatus: some ToolbarContent {
        ToolbarItem(placement: .principal) {
            if #available(macOS 26.0, *) {
                HStack(spacing: 10) {
                    Circle()
                        .fill(.orange)
                        .frame(width: 10, height: 10)
                    Text(model.statusText)
                        .font(.subheadline.weight(.medium))
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .glassEffect()
            } else {
                HStack(spacing: 10) {
                    Circle()
                        .fill(.orange)
                        .frame(width: 10, height: 10)
                    Text(model.statusText)
                        .font(.subheadline.weight(.medium))
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .background(.thinMaterial, in: Capsule())
            }
        }
    }

    @ViewBuilder
    private var detailView: some View {
        switch model.selectedItem {
        case .plans:
            PlanDashboard()
        case .agents:
            AgentCatalogView()
        case .mesh:
            MeshTopologyView()
        case .evolution:
            EvolutionDashboardView()
        case .costs:
            CostDashboardView()
        case .terminal:
            GhosttyTerminal()
        case .brain:
            BrainVisualizationView(notificationManager: notificationManager)
        default:
            SectionPlaceholderView(item: model.selectedItem)
        }
    }
}
