import SwiftUI

struct RunHistoryView: View {
    let runs: [RunSummary]
    @Binding var selectedRunID: Int?
    let statusBuckets: [RunStatusBucket]
    let note: String

    var body: some View {
        VStack(alignment: .leading, spacing: Spacing.md) {
            Text("Run History")
                .font(.sectionTitle)

            Label(note, systemImage: "info.circle")
                .font(.footnote)
                .foregroundStyle(ConvergioTokens.Text.textMuted)

            Table(runs, selection: $selectedRunID) {
                TableColumn("Run") { run in
                    Text("#\(run.id)")
                        .font(.mono)
                }
                .width(min: 70, ideal: 80)

                TableColumn("Goal") { run in
                    Text(run.goal)
                        .lineLimit(1)
                }

                TableColumn("Status") { run in
                    // StatusBadge with semantic brand colors per status
                    StatusBadge(
                        label: (run.status ?? "unknown").capitalized,
                        color: badgeColor(for: run.status)
                    )
                    .accessibilityLabel("Status: \((run.status ?? "unknown").capitalized)")
                }
                .width(min: 110, ideal: 120)

                TableColumn("Duration") { run in
                    Text(durationLabel(run.durationMinutes))
                }
                .width(min: 90, ideal: 100)

                TableColumn("Cost") { run in
                    Text(currency(run.costUsd))
                }
                .width(min: 90, ideal: 100)

                TableColumn("Plan") { run in
                    Text(run.planName ?? "—")
                        .lineLimit(1)
                }
            }
            .frame(minHeight: 320)
            .accessibilityLabel("Run history table")

            if !statusBuckets.isEmpty {
                statusBucketRow
            }
        }
        .modifier(ConvergioCardModifier())
    }

    // MARK: — Status bucket chips

    private var statusBucketRow: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: Spacing.xs) {
                ForEach(statusBuckets) { bucket in
                    StatusBadge(
                        label: "\(bucket.label) · \(bucket.count)",
                        color: badgeColor(for: bucket.label.lowercased())
                    )
                }
            }
        }
    }

    // MARK: — Status → brand color mapping
    // completed=verdeRacing, running=azzurro, failed=rossoCorsa, paused=arancioWarm

    private func badgeColor(for status: String?) -> Color {
        switch status?.lowercased() {
        case "completed", "done", "success":
            return ConvergioTokens.Brand.verdeRacing
        case "running", "in_progress", "active":
            return ConvergioTokens.Brand.azzurro
        case "failed", "error":
            return ConvergioTokens.Brand.rossoCorsa
        case "paused", "pending", "waiting":
            return ConvergioTokens.Brand.arancioWarm
        default:
            return ConvergioTokens.Text.textMuted
        }
    }

    // MARK: — Formatters

    private func currency(_ value: Double?) -> String {
        let formatter = NumberFormatter()
        formatter.numberStyle = .currency
        formatter.currencyCode = "USD"
        return formatter.string(from: NSNumber(value: value ?? 0)) ?? "$0.00"
    }

    private func durationLabel(_ minutes: Double?) -> String {
        guard let minutes else { return "—" }
        if minutes >= 60 {
            return String(format: "%.1fh", minutes / 60)
        }
        return String(format: "%.0fm", minutes)
    }
}
