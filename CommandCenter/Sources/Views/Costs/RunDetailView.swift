import SwiftUI

struct RunDetailView: View {
    let run: RunDetail
    let metrics: RunMetricsResponse?
    let isMutating: Bool
    let onPause: () -> Void
    let onResume: () -> Void

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: Spacing.lg) {
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
            .padding(Spacing.lg)
        }
        .modifier(ConvergioCardModifier())
    }

    // MARK: — Header

    private var header: some View {
        VStack(alignment: .leading, spacing: Spacing.xs) {
            Text("Run #\(run.id)")
                .font(.heroTitle)
                .foregroundStyle(ConvergioTokens.Text.textPrimary)
            Text(run.goal)
                .font(.cardTitle)
                .foregroundStyle(ConvergioTokens.Text.textPrimary)
            HStack(spacing: Spacing.xs) {
                chip(
                    (run.status ?? "unknown").capitalized,
                    systemImage: "bolt.horizontal.circle",
                    color: statusColor(for: run.status)
                )
                if let planName = run.planName {
                    chip(planName, systemImage: "list.bullet.rectangle",
                         color: ConvergioTokens.Brand.azzurro)
                }
                if let pausedAt = run.pausedAt {
                    chip(relativeTime(for: pausedAt), systemImage: "pause.circle",
                         color: ConvergioTokens.Brand.arancioWarm)
                }
            }
        }
    }

    // MARK: — Action bar (accent-colored buttons)

    private var actionBar: some View {
        HStack(spacing: Spacing.sm) {
            Button {
                onPause()
            } label: {
                Label("Pause", systemImage: "pause.fill")
                    .foregroundStyle(ConvergioTokens.Brand.arancioWarm)
            }
            .disabled(isMutating || run.status == "paused")
            .accessibilityLabel("Pause run \(run.id)")

            Button {
                onResume()
            } label: {
                Label("Resume", systemImage: "play.fill")
                    .foregroundStyle(ConvergioTokens.Brand.verdeRacing)
            }
            .disabled(isMutating || run.status != "paused")
            .accessibilityLabel("Resume run \(run.id)")

            if isMutating {
                ProgressView()
                    .tint(ConvergioTokens.Brand.azzurro)
                    .accessibilityLabel("Applying run control action")
            }
        }
    }

    // MARK: — Metrics grid

    private var metricsGrid: some View {
        let columns = [GridItem(.flexible()), GridItem(.flexible())]
        return LazyVGrid(columns: columns, spacing: Spacing.sm) {
            metricCard("Primary Cost",    value: currency(run.costUsd),
                       accent: ConvergioTokens.Brand.gialloFerrari)
            metricCard("Delegation Cost", value: currency(run.delegationCost),
                       accent: ConvergioTokens.Brand.gialloFerrari)
            metricCard("Agents Used",
                       value: String(run.agentsUsed ?? metrics?.agentsUsed ?? 0),
                       accent: ConvergioTokens.Brand.verdeRacing)
            metricCard("Task Progress",
                       value: "\(metrics?.tasksDone ?? 0)/\(metrics?.tasksTotal ?? 0)",
                       accent: ConvergioTokens.Brand.azzurro)
            metricCard("Duration",
                       value: durationLabel(run.durationMinutes, fallbackSeconds: metrics?.durationSecs),
                       accent: ConvergioTokens.Brand.arancioWarm)
            metricCard("Team", value: run.team ?? "—",
                       accent: ConvergioTokens.Roles.role10)
        }
    }

    private func metricCard(_ title: String, value: String, accent: Color) -> some View {
        VStack(alignment: .leading, spacing: Spacing.xxs) {
            Text(title)
                .font(.label)
                .foregroundStyle(ConvergioTokens.Text.textMuted)
            Text(value)
                .font(.title3.weight(.semibold))
                .foregroundStyle(accent)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(Spacing.md)
        .background(
            RoundedRectangle(cornerRadius: CornerRadius.md, style: .continuous)
                .fill(.regularMaterial)
                .overlay(
                    RoundedRectangle(cornerRadius: CornerRadius.md, style: .continuous)
                        .strokeBorder(ConvergioTokens.Border.borderSubtle, lineWidth: 1)
                )
        )
    }

    // MARK: — Detail section

    private func detailSection(title: String, value: String) -> some View {
        VStack(alignment: .leading, spacing: Spacing.xs) {
            Text(title)
                .font(.cardTitle)
                .foregroundStyle(ConvergioTokens.Text.textPrimary)
            Text(value)
                .font(.mono)
                .textSelection(.enabled)
                .foregroundStyle(ConvergioTokens.Text.textMuted)
        }
        .padding(Spacing.md)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            RoundedRectangle(cornerRadius: CornerRadius.md, style: .continuous)
                .fill(.regularMaterial)
                .overlay(
                    RoundedRectangle(cornerRadius: CornerRadius.md, style: .continuous)
                        .strokeBorder(ConvergioTokens.Border.borderSubtle, lineWidth: 1)
                )
        )
    }

    // MARK: — Chip helper

    private func chip(_ text: String, systemImage: String, color: Color) -> some View {
        Label {
            Text(text)
        } icon: {
            Image(systemName: systemImage)
                .foregroundStyle(color)
        }
        .font(.label)
        .padding(.horizontal, Spacing.xs)
        .padding(.vertical, Spacing.xxs)
        .background(ConvergioTokens.Border.borderSubtle, in: Capsule())
        .accessibilityLabel("Status: \(text)")
    }

    // MARK: — Status color (mirrors RunHistoryView mapping)

    private func statusColor(for status: String?) -> Color {
        switch status?.lowercased() {
        case "completed", "done", "success": return ConvergioTokens.Brand.verdeRacing
        case "running", "in_progress":       return ConvergioTokens.Brand.azzurro
        case "failed", "error":              return ConvergioTokens.Brand.rossoCorsa
        case "paused":                       return ConvergioTokens.Brand.arancioWarm
        default:                             return ConvergioTokens.Text.textMuted
        }
    }

    // MARK: — Formatters

    private func currency(_ value: Double?) -> String {
        let formatter = NumberFormatter()
        formatter.numberStyle = .currency
        formatter.currencyCode = "USD"
        return formatter.string(from: NSNumber(value: value ?? 0)) ?? "$0.00"
    }

    private func durationLabel(_ minutes: Double?, fallbackSeconds: Int?) -> String {
        if let minutes {
            if minutes >= 60 { return String(format: "%.1fh", minutes / 60) }
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
