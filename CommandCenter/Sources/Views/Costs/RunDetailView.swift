import SwiftUI

struct RunDetailView: View {
    let run: RunDetail
    let metrics: RunMetricsResponse?
    let isMutating: Bool
    let onPause: () -> Void
    let onResume: () -> Void

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                header
                actionBar
                metricsGrid

                if let result = run.result, !result.isEmpty {
                    detailSection(title: "Result", value: result)
                }
                if let contextPath = run.contextPath, !contextPath.isEmpty {
                    detailSection(title: "Context Path", value: contextPath)
                }
            }
            .padding(24)
        }
        .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 24, style: .continuous))
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Run #\(run.id)")
                .font(.title.weight(.bold))
            Text(run.goal)
                .font(.headline)
            HStack(spacing: 10) {
                chip((run.status ?? "unknown").capitalized, systemImage: "bolt.horizontal.circle")
                if let planName = run.planName {
                    chip(planName, systemImage: "list.bullet.rectangle")
                }
                if let pausedAt = run.pausedAt {
                    chip(relativeTime(for: pausedAt), systemImage: "pause.circle")
                }
            }
        }
    }

    private var actionBar: some View {
        HStack(spacing: 12) {
            Button {
                onPause()
            } label: {
                Label("Pause", systemImage: "pause.fill")
            }
            .disabled(isMutating || run.status == "paused")

            Button {
                onResume()
            } label: {
                Label("Resume", systemImage: "play.fill")
            }
            .disabled(isMutating || run.status != "paused")

            if isMutating {
                ProgressView()
            }
        }
    }

    private var metricsGrid: some View {
        let columns = [GridItem(.flexible()), GridItem(.flexible())]
        return LazyVGrid(columns: columns, spacing: 12) {
            metricCard("Primary Cost", value: currency(run.costUsd))
            metricCard("Delegation Cost", value: currency(run.delegationCost))
            metricCard("Agents Used", value: String(run.agentsUsed ?? metrics?.agentsUsed ?? 0))
            metricCard("Task Progress", value: "\(metrics?.tasksDone ?? 0)/\(metrics?.tasksTotal ?? 0)")
            metricCard("Duration", value: durationLabel(run.durationMinutes, fallbackSeconds: metrics?.durationSecs))
            metricCard("Team", value: run.team ?? "—")
        }
    }

    private func metricCard(_ title: String, value: String) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title)
                .font(.caption)
                .foregroundStyle(.secondary)
            Text(value)
                .font(.title3.weight(.semibold))
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(16)
        .background(.quaternary, in: RoundedRectangle(cornerRadius: 18, style: .continuous))
    }

    private func detailSection(title: String, value: String) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(.headline)
            Text(value)
                .font(.body.monospaced())
                .textSelection(.enabled)
        }
        .padding(18)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.quaternary, in: RoundedRectangle(cornerRadius: 18, style: .continuous))
    }

    private func chip(_ text: String, systemImage: String) -> some View {
        Label(text, systemImage: systemImage)
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

    private func durationLabel(_ minutes: Double?, fallbackSeconds: Int?) -> String {
        if let minutes {
            if minutes >= 60 {
                return String(format: "%.1fh", minutes / 60)
            }
            return String(format: "%.0fm", minutes)
        }
        if let fallbackSeconds {
            return String(format: "%.0fm", Double(fallbackSeconds) / 60)
        }
        return "—"
    }

    private func relativeTime(for date: Date) -> String {
        RelativeDateTimeFormatter().localizedString(for: date, relativeTo: .now)
    }
}
