import SwiftUI

struct CostDashboardView: View {
    @State private var model: CostDashboardModel

    init(client: DaemonClient = DaemonClient()) {
        _model = State(initialValue: CostDashboardModel(client: client))
    }

    var body: some View {
        HSplitView {
            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    header

                    Picker("Window", selection: selectedRangeBinding) {
                        ForEach(CostRange.allCases) { range in
                            Text(range.title).tag(range)
                        }
                    }
                    .pickerStyle(.segmented)
                    .frame(maxWidth: 320)

                    LazyVGrid(columns: [GridItem(.adaptive(minimum: 180), spacing: 16)], spacing: 16) {
                        ForEach(model.usageWindows()) { window in
                            UsageWindowCard(window: window)
                        }
                    }

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
                .padding(24)
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
            .padding(24)
        }
        .task { await model.load() }
        .overlay(alignment: .topTrailing) {
            if let errorMessage = model.errorMessage {
                Text(errorMessage)
                    .font(.footnote)
                    .foregroundStyle(.red)
                    .padding(12)
            }
        }
    }

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

    private var header: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 6) {
                    Text("Cost Dashboard")
                        .font(.largeTitle.weight(.bold))
                    Text("Native spend, model, and run observability backed by /api/metrics/* and /api/runs/*.")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }

                Spacer()

                if model.isLoading {
                    ProgressView()
                }
            }

            HStack(spacing: 10) {
                summaryChip("\(model.summary?.runCount ?? 0) runs", icon: "clock.badge")
                summaryChip(currency(model.summary?.totalCostUsd), icon: "dollarsign.circle")
                summaryChip(avgDurationText, icon: "timer")
                summaryChip(topAgentText, icon: "person.3.sequence")
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

    private func summaryChip(_ text: String, icon: String) -> some View {
        Label(text, systemImage: icon)
            .font(.caption.weight(.medium))
            .padding(.horizontal, 10)
            .padding(.vertical, 6)
            .background(.quaternary, in: Capsule())
    }

    private func currency(_ value: Double?) -> String {
        let formatter = NumberFormatter()
        formatter.numberStyle = .currency
        formatter.currencyCode = "USD"
        return formatter.string(from: NSNumber(value: value ?? 0)) ?? "$0.00"
    }
}

private struct UsageWindowCard: View {
    let window: UsageWindowSummary

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(window.range.title)
                .font(.headline)
            Text(currency(window.costUsd))
                .font(.title2.weight(.bold))
            Text("\(window.runCount) runs")
                .font(.subheadline)
                .foregroundStyle(.secondary)
            Text(avgDuration)
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(18)
        .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 22, style: .continuous))
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
