import SwiftUI

struct CostDashboardView: View {
    @State private var model: CostDashboardModel

    init(client: DaemonClient = DaemonClient()) {
        _model = State(initialValue: CostDashboardModel(client: client))
    }

    var body: some View {
        HSplitView {
            ScrollView {
                VStack(alignment: .leading, spacing: Spacing.lg) {
                    header

                    Picker("Window", selection: selectedRangeBinding) {
                        ForEach(CostRange.allCases) { range in
                            Text(range.title).tag(range)
                        }
                    }
                    .pickerStyle(.segmented)
                    .frame(maxWidth: 320)
                    .accessibilityLabel("Cost time window selector")

                    kpiGrid

                    ModelCostBreakdown(
                        models: model.modelCosts(for: model.selectedRange),
                        trend: model.trend(for: model.selectedRange),
                        rangeTitle: model.selectedRange.title
                    )

                    RunHistoryView(
                        runs: model.runs,
                        selectedRunID: selectedRunBinding,
                        statusBuckets: model.summary?.statusDistribution ?? [],
                        note: model.dataGapMessage
                    )
                }
                .padding(Spacing.lg)
            }
            .frame(minWidth: 760)

            Group {
                if let run = model.selectedRun {
                    RunDetailView(
                        run: run,
                        metrics: model.selectedMetrics,
                        isMutating: model.isMutatingRun,
                        onPause: { Task { await model.pauseSelectedRun() } },
                        onResume: { Task { await model.resumeSelectedRun() } }
                    )
                } else {
                    ContentUnavailableView(
                        "Select a run",
                        systemImage: "clock.arrow.trianglehead.counterclockwise.rotate.90",
                        description: Text("Pick a run from the history table to inspect cost, task progress, and control state.")
                    )
                }
            }
            .frame(minWidth: 340, idealWidth: 380)
            .padding(Spacing.lg)
        }
        .task { await model.load() }
        .overlay(alignment: .topTrailing) {
            if let errorMessage = model.errorMessage {
                Text(errorMessage)
                    .font(.footnote)
                    .foregroundStyle(ConvergioTokens.Status.error)
                    .padding(Spacing.sm)
            }
        }
    }

    // MARK: — KPI grid (one card per metric)

    private var kpiGrid: some View {
        LazyVGrid(columns: [GridItem(.adaptive(minimum: 180), spacing: Spacing.md)], spacing: Spacing.md) {
            ForEach(model.usageWindows()) { window in
                UsageWindowCard(window: window)
            }
        }
    }

    // MARK: — Header

    private var header: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: Spacing.xxs) {
                    Text("Cost Dashboard")
                        .font(.heroTitle)
                    Text("Native spend, model, and run observability backed by /api/metrics/* and /api/runs/*.")
                        .font(.subheadline)
                        .foregroundStyle(ConvergioTokens.Text.textMuted)
                }
                Spacer()
                if model.isLoading { ProgressView().accessibilityLabel("Loading cost data") }
            }

            HStack(spacing: Spacing.xs) {
                // cost=gialloFerrari, runs=azzurro, duration=arancioWarm, agents=verdeRacing
                summaryChip(currency(model.summary?.totalCostUsd), icon: "dollarsign.circle",
                            iconColor: ConvergioTokens.Brand.gialloFerrari)
                summaryChip("\(model.summary?.runCount ?? 0) runs", icon: "clock.badge",
                            iconColor: ConvergioTokens.Brand.azzurro)
                summaryChip(avgDurationText, icon: "timer",
                            iconColor: ConvergioTokens.Brand.arancioWarm)
                summaryChip(topAgentText, icon: "person.3.sequence",
                            iconColor: ConvergioTokens.Brand.verdeRacing)
            }
        }
    }

    private var avgDurationText: String {
        guard let avgDurationSecs = model.summary?.avgDurationSecs else { return "No duration" }
        return String(format: "%.0fm avg", avgDurationSecs / 60)
    }

    private var topAgentText: String {
        guard let topAgent = model.summary?.topAgents.first else { return "No routing yet" }
        return "\(topAgent.label) · \(topAgent.taskCount)"
    }

    private func summaryChip(_ text: String, icon: String, iconColor: Color) -> some View {
        Label {
            Text(text)
        } icon: {
            Image(systemName: icon)
                .foregroundStyle(iconColor)
        }
        .font(.caption.weight(.medium))
        .padding(.horizontal, Spacing.xs)
        .padding(.vertical, Spacing.xxs)
        .background(ConvergioTokens.Border.borderSubtle, in: Capsule())
        .accessibilityLabel("Status: \(text)")
    }

    private func currency(_ value: Double?) -> String {
        let formatter = NumberFormatter()
        formatter.numberStyle = .currency
        formatter.currencyCode = "USD"
        return formatter.string(from: NSNumber(value: value ?? 0)) ?? "$0.00"
    }

    // MARK: — Bindings

    private var selectedRunBinding: Binding<Int?> {
        Binding(
            get: { model.selectedRunID },
            set: { value in Task { await model.selectRun(value) } }
        )
    }

    private var selectedRangeBinding: Binding<CostRange> {
        Binding(
            get: { model.selectedRange },
            set: { model.selectedRange = $0 }
        )
    }
}

// MARK: — Usage window KPI card

private struct UsageWindowCard: View {
    let window: UsageWindowSummary

    var body: some View {
        VStack(alignment: .leading, spacing: Spacing.xs) {
            HStack(spacing: Spacing.xxs) {
                // cost icon colored gialloFerrari per Maranello token spec
                Image(systemName: "dollarsign.circle")
                    .foregroundStyle(ConvergioTokens.Brand.gialloFerrari)
                Text(window.range.title)
                    .font(.cardTitle)
            }

            Text(currency(window.costUsd))
                .font(.heroTitle)
                .foregroundStyle(ConvergioTokens.Text.textPrimary)

            Text("\(window.runCount) runs · \(avgDuration)")
                .font(.caption)
                .foregroundStyle(ConvergioTokens.Text.textMuted)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .modifier(ConvergioCardModifier())
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(window.range.title) cost: \(currency(window.costUsd)), \(window.runCount) runs, \(avgDuration)")
    }

    private var avgDuration: String {
        guard let avgDurationSecs = window.avgDurationSecs else { return "No duration baseline" }
        return String(format: "Avg %.0fm", avgDurationSecs / 60)
    }

    private func currency(_ value: Double) -> String {
        let formatter = NumberFormatter()
        formatter.numberStyle = .currency
        formatter.currencyCode = "USD"
        return formatter.string(from: NSNumber(value: value)) ?? "$0.00"
    }
}
