import Foundation
import Observation

enum CostRange: String, CaseIterable, Identifiable {
    case daily
    case weekly
    case monthly

    var id: String { rawValue }

    var days: Int {
        switch self {
        case .daily: return 1
        case .weekly: return 7
        case .monthly: return 30
        }
    }

    var title: String { rawValue.capitalized }
}

struct UsageWindowSummary: Identifiable {
    let range: CostRange
    let costUsd: Double
    let runCount: Int
    let avgDurationSecs: Double?

    var id: CostRange { range }
}

struct ModelCostDatum: Identifiable {
    let family: String
    let costUsd: Double
    let calls: Int

    var id: String { family }
}

struct CostTrendDatum: Identifiable {
    let date: Date
    let costUsd: Double

    var id: Date { date }
}

@MainActor
@Observable
final class CostDashboardModel {
    private let client: DaemonClient
    private let dashboardSocket: WebSocketManager
    private var subscribed = false

    var summary: CostSummaryResponse?
    var runs: [RunSummary] = []
    var selectedRunID: Int?
    var selectedRun: RunDetail?
    var selectedMetrics: RunMetricsResponse?
    var selectedRange: CostRange = .weekly
    var windowBreakdowns: [CostRange: CostBreakdownResponse] = [:]
    var isLoading = false
    var isMutatingRun = false
    var errorMessage: String?

    let dataGapMessage = "Per-run token counts are not currently exposed by the daemon API. This view surfaces cost, agents, and task progress until a dedicated token endpoint lands."

    init(client: DaemonClient = DaemonClient()) {
        self.client = client
        dashboardSocket = WebSocketManager(client: client)
    }

    func load() async {
        await subscribeIfNeeded()
        await refresh()
    }

    func refresh() async {
        isLoading = true
        defer { isLoading = false }

        do {
            async let summaryTask = client.metricsSummary()
            async let dailyTask = client.costBreakdown(days: CostRange.daily.days)
            async let weeklyTask = client.costBreakdown(days: CostRange.weekly.days)
            async let monthlyTask = client.costBreakdown(days: CostRange.monthly.days)
            async let runsTask = client.runHistory(limit: 40)

            summary = try await summaryTask
            windowBreakdowns[.daily] = try await dailyTask
            windowBreakdowns[.weekly] = try await weeklyTask
            windowBreakdowns[.monthly] = try await monthlyTask
            runs = try await runsTask

            if let selectedRunID, !runs.contains(where: { $0.id == selectedRunID }) {
                self.selectedRunID = runs.first?.id
            }
            if selectedRunID == nil {
                selectedRunID = runs.first?.id
            }
            await loadSelectedRun()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func selectRun(_ id: Int?) async {
        selectedRunID = id
        await loadSelectedRun()
    }

    func pauseSelectedRun() async {
        guard let selectedRunID else { return }
        isMutatingRun = true
        defer { isMutatingRun = false }

        do {
            selectedRun = try await client.pauseRun(runID: selectedRunID)
            await refreshRunsKeepingSelection()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func resumeSelectedRun() async {
        guard let selectedRunID else { return }
        isMutatingRun = true
        defer { isMutatingRun = false }

        do {
            selectedRun = try await client.resumeRun(runID: selectedRunID)
            await refreshRunsKeepingSelection()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func usageWindows() -> [UsageWindowSummary] {
        CostRange.allCases.map { range in
            let breakdown = windowBreakdowns[range]
            return UsageWindowSummary(
                range: range,
                costUsd: breakdown?.byDate.reduce(0) { $0 + $1.costUsd } ?? 0,
                runCount: runCount(for: range.days),
                avgDurationSecs: summary?.avgDurationSecs
            )
        }
    }

    func modelCosts(for range: CostRange) -> [ModelCostDatum] {
        let families = ["Opus", "Sonnet", "Haiku", "Codex"]
        let buckets = windowBreakdowns[range]?.byModel ?? []
        let grouped = Dictionary(grouping: buckets, by: { family(for: $0.label) })

        return families.map { family in
            let group = grouped[family] ?? []
            return ModelCostDatum(
                family: family,
                costUsd: group.reduce(0) { $0 + $1.costUsd },
                calls: group.reduce(0) { $0 + $1.calls }
            )
        }
    }

    func trend(for range: CostRange) -> [CostTrendDatum] {
        (windowBreakdowns[range]?.byDate ?? [])
            .compactMap { point in
                guard let date = point.parsedDate else { return nil }
                return CostTrendDatum(date: date, costUsd: point.costUsd)
            }
            .sorted { $0.date < $1.date }
    }

    private func loadSelectedRun() async {
        guard let selectedRunID else {
            selectedRun = nil
            selectedMetrics = nil
            return
        }

        do {
            async let detailTask = client.runDetail(runID: selectedRunID)
            async let metricsTask = client.runMetrics(runID: selectedRunID)
            selectedRun = try await detailTask
            selectedMetrics = try await metricsTask
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func refreshRunsKeepingSelection() async {
        do {
            runs = try await client.runHistory(limit: 40)
            await loadSelectedRun()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func subscribeIfNeeded() async {
        guard !subscribed else { return }
        subscribed = true

        try? await dashboardSocket.connect(.dashboard) { [weak self] event in
            guard let self else { return }
            switch event {
            case .json(_, let envelope):
                guard envelope.type == "run_update" || envelope.type == "notification" else { return }
                Task { @MainActor in await self.refresh() }
            case .text:
                Task { @MainActor in await self.refresh() }
            case .binary, .disconnected:
                break
            }
        }
    }

    private func runCount(for days: Int) -> Int {
        let cutoff = Date().addingTimeInterval(-Double(days) * 86_400)
        return runs.filter { ($0.startedAt ?? .distantPast) >= cutoff }.count
    }

    private func family(for model: String) -> String {
        let lowercased = model.lowercased()
        if lowercased.contains("opus") { return "Opus" }
        if lowercased.contains("sonnet") { return "Sonnet" }
        if lowercased.contains("haiku") { return "Haiku" }
        if lowercased.contains("codex") { return "Codex" }
        return "Other"
    }
}
