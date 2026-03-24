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
                    .background(
                        LinearGradient(
                            colors: [ConvergioTokens.Surface.surfaceSunken, ConvergioTokens.Surface.surface],
                            startPoint: .top,
                            endPoint: .bottom
                        )
                    )
            }
        }
    }

    private var listContent: some View {
        VStack(spacing: 0) {
            // Convergio wordmark
            Text("CONVERGIO")
                .font(.system(size: 22, weight: .bold, design: .default))
                .tracking(6)
                .foregroundStyle(ConvergioTokens.Brand.gialloFerrari)
                .shadow(color: ConvergioTokens.Brand.gialloFerrari.opacity(0.4), radius: 8, y: 2)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, Spacing.md)
                .padding(.top, Spacing.lg)
                .padding(.bottom, Spacing.md)
                .accessibilityLabel("Convergio Command Center")

            List(SidebarItem.allCases, selection: $model.selection) { item in
                HStack(spacing: 0) {
                    RoundedRectangle(cornerRadius: 2)
                        .fill(ConvergioTokens.Brand.gialloFerrari)
                        .frame(width: 3, height: 20)
                        .opacity(model.selection == item ? 1 : 0)
                    Label(item.title, systemImage: item.icon)
                        .foregroundStyle(.primary)
                        .labelStyle(ColoredIconLabelStyle(color: item.color))
                        .padding(.leading, Spacing.xs)
                }
                .tag(Optional(item))
                .listRowSeparator(.hidden)
                .listRowInsets(EdgeInsets(top: 6, leading: 8, bottom: 6, trailing: 8))
                .accessibilityLabel(item.title)
            }
            .listStyle(.sidebar)
            .navigationTitle("")
            .tint(ConvergioTokens.Brand.gialloFerrari)

            // Daemon status footer
            HStack(spacing: Spacing.xs) {
                let isReady = model.statusText.contains("Ready")
                let dotColor = isReady
                    ? ConvergioTokens.Status.success
                    : ConvergioTokens.Text.textMuted
                Circle()
                    .fill(dotColor)
                    .frame(width: 8, height: 8)
                    .shadow(color: isReady ? dotColor.opacity(0.6) : .clear, radius: 6)
                Text(model.statusText)
                    .font(.label)
                    .foregroundStyle(ConvergioTokens.Text.textMuted)
                Spacer()
            }
            .padding(.horizontal, Spacing.sm)
            .padding(.vertical, Spacing.xs)
            .background(.ultraThinMaterial, in: Capsule())
            .padding(.horizontal, Spacing.sm)
            .padding(.bottom, Spacing.sm)
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
                        .shadow(color: ConvergioTokens.Brand.gialloFerrari.opacity(0.5), radius: 6)
                        .accessibilityHidden(true)
                    Text(model.statusText)
                        .font(.subheadline.weight(.medium))
                }
                .padding(.horizontal, 14)
                .padding(.vertical, 8)
                .background(.thinMaterial, in: Capsule())
                .shadow(color: .black.opacity(0.25), radius: 8, y: 2)
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
