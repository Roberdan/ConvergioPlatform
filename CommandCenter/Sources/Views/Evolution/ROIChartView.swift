import Charts
import SwiftUI

struct ROIChartView: View {
    let roi: EvolutionROIResponse?
    let proposals: [EvolutionProposal]

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            HStack {
                Text("ROI charts")
                    .font(.title3.weight(.semibold))
                Spacer()
                if let roi {
                    Label(String(format: "%.1f%% success", roi.successRate), systemImage: "chart.line.uptrend.xyaxis")
                        .foregroundStyle(.secondary)
                }
            }

            Chart(trendPoints) { point in
                LineMark(
                    x: .value("Proposal", point.sequence),
                    y: .value("Expected delta", point.expectedDelta)
                )
                PointMark(
                    x: .value("Proposal", point.sequence),
                    y: .value("Expected delta", point.expectedDelta)
                )
            }
            .chartXAxis {
                AxisMarks(values: trendPoints.map(\.sequence)) { value in
                    AxisGridLine()
                    if let sequence = value.as(Int.self),
                       let point = trendPoints.first(where: { $0.sequence == sequence }) {
                        AxisValueLabel(point.label)
                    }
                }
            }
            .frame(height: 180)

            if let roi {
                Chart(roi.proposalsByStatus) { bucket in
                    BarMark(
                        x: .value("Status", bucket.statusLabel),
                        y: .value("Count", bucket.count)
                    )
                    .foregroundStyle(by: .value("Status", bucket.statusLabel))
                }
                .frame(height: 180)

                HStack(spacing: 12) {
                    metricPill("Experiments", value: "\(roi.experimentsRun)")
                    metricPill("Successes", value: "\(roi.successes)")
                    metricPill("Rollbacks", value: "\(roi.rollbacks)")
                }
            } else {
                ContentUnavailableView("No ROI data", systemImage: "chart.bar")
            }
        }
        .padding(20)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 24, style: .continuous))
    }

    private var trendPoints: [ProposalTrendPoint] {
        proposals
            .sorted { ($0.createdAt ?? "\($0.id)") < ($1.createdAt ?? "\($1.id)") }
            .enumerated()
            .map { index, proposal in
                ProposalTrendPoint(
                    id: proposal.id,
                    sequence: index + 1,
                    label: "#\(proposal.id)",
                    expectedDelta: proposal.expectedDeltaPercent
                )
            }
    }

    private func metricPill(_ title: String, value: String) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title)
                .font(.caption)
                .foregroundStyle(.secondary)
            Text(value)
                .font(.headline)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.quaternary.opacity(0.12), in: RoundedRectangle(cornerRadius: 18, style: .continuous))
    }
}

private struct ProposalTrendPoint: Identifiable {
    let id: Int
    let sequence: Int
    let label: String
    let expectedDelta: Double
}
