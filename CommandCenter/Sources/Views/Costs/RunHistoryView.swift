import SwiftUI

struct RunHistoryView: View {
    let runs: [RunSummary]
    @Binding var selectedRunID: Int?
    let statusBuckets: [RunStatusBucket]
    let note: String

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Run History")
                .font(.title2.weight(.semibold))

            Label(note, systemImage: "info.circle")
                .font(.footnote)
                .foregroundStyle(.secondary)

            Table(runs, selection: $selectedRunID) {
                TableColumn("Run") { run in
                    Text("#\(run.id)")
                        .font(.body.monospaced())
                }
                .width(min: 70, ideal: 80)

                TableColumn("Goal") { run in
                    Text(run.goal)
                        .lineLimit(1)
                }

                TableColumn("Status") { run in
                    Text((run.status ?? "unknown").capitalized)
                }
                .width(min: 100, ideal: 110)

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

            if !statusBuckets.isEmpty {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 10) {
                        ForEach(statusBuckets) { bucket in
                            Label("\(bucket.label) · \(bucket.count)", systemImage: "chart.bar.xaxis")
                                .font(.caption.weight(.medium))
                                .padding(.horizontal, 10)
                                .padding(.vertical, 6)
                                .background(.quaternary, in: Capsule())
                        }
                    }
                }
            }
        }
        .padding(20)
        .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 24, style: .continuous))
    }

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
