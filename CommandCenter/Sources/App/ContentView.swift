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
        VStack(spacing: 0) {
            // Convergio wordmark
            Text("CONVERGIO")
                .font(.sectionTitle)
                .foregroundStyle(ConvergioTokens.Brand.gialloFerrari)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, Spacing.md)
                .padding(.top, Spacing.md)
                .padding(.bottom, Spacing.sm)
                .accessibilityLabel("Convergio Command Center")

            List(SidebarItem.allCases, selection: $model.selection) { item in
                Label(item.title, systemImage: item.icon)
                    .foregroundStyle(.primary)
                    .labelStyle(ColoredIconLabelStyle(color: item.color))
                    .tag(Optional(item))
                    .listRowSeparator(.hidden)
                    .accessibilityLabel(item.title)
            }
            .listStyle(.sidebar)
            .navigationTitle("")
            .tint(ConvergioTokens.Brand.gialloFerrari)

            // Daemon status footer
            HStack(spacing: Spacing.xs) {
                Circle()
                    .fill(
                        model.statusText.contains("Ready")
                            ? ConvergioTokens.Status.success
                            : ConvergioTokens.Text.textMuted
                    )
                    .frame(width: 8, height: 8)
                Text(model.statusText)
                    .font(.label)
                    .foregroundStyle(ConvergioTokens.Text.textMuted)
                Spacer()
            }
            .padding(.horizontal, Spacing.md)
            .padding(.vertical, Spacing.sm)
        }
    }

    @ToolbarContentBuilder
    private var toolbarStatus: some ToolbarContent {
        ToolbarItem(placement: .principal) {
            if #available(macOS 26.0, *) {
                HStack(spacing: 10) {
                    Circle()
                        .fill(ConvergioTokens.Brand.gialloFerrari)
                        .frame(width: 10, height: 10)
                        .accessibilityHidden(true)
                    Text(model.statusText)
                        .font(.subheadline.weight(.medium))
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .glassEffect()
                .accessibilityElement(children: .combine)
                .accessibilityLabel("Status: \(model.statusText)")
            } else {
                HStack(spacing: 10) {
                    Circle()
                        .fill(ConvergioTokens.Brand.gialloFerrari)
                        .frame(width: 10, height: 10)
                        .accessibilityHidden(true)
                    Text(model.statusText)
                        .font(.subheadline.weight(.medium))
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .background(.thinMaterial, in: Capsule())
                .accessibilityElement(children: .combine)
                .accessibilityLabel("Status: \(model.statusText)")
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

// Applies the Maranello per-section accent to the Label icon only.
private struct ColoredIconLabelStyle: LabelStyle {
    let color: Color

    func makeBody(configuration: Configuration) -> some View {
        HStack(spacing: Spacing.xs) {
            configuration.icon
                .foregroundStyle(color)
            configuration.title
        }
    }
}
