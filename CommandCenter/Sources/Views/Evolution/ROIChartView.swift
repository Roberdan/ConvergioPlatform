import Charts
import SwiftUI

struct ROIChartView: View {
    let roi: EvolutionROIResponse?
    let proposals: [EvolutionProposal]

    var body: some View {
        VStack(alignment: .leading, spacing: Spacing.md) {
            HStack {
                Text("ROI charts")
                    .font(.sectionTitle)
                    .foregroundStyle(ConvergioTokens.Text.textPrimary)
                Spacer()
                if let roi {
                    Label(
                        String(format: "%.1f%% success", roi.successRate),
                        systemImage: "chart.line.uptrend.xyaxis"
                    )
                    .font(.label)
                    .foregroundStyle(ConvergioTokens.Brand.verdeRacing)
                }
            }

            // Line chart: expected delta trend across proposals
            Chart(trendPoints) { point in
                LineMark(
                    x: .value("Proposal", point.sequence),
                    y: .value("Expected delta", point.expectedDelta)
                )
                .foregroundStyle(ConvergioTokens.Brand.verdeRacing)
                .interpolationMethod(.catmullRom)

                AreaMark(
                    x: .value("Proposal", point.sequence),
                    y: .value("Expected delta", point.expectedDelta)
                )
                .foregroundStyle(
                    LinearGradient(
                        colors: [
                            ConvergioTokens.Brand.verdeRacing.opacity(0.3),
                            ConvergioTokens.Brand.verdeRacing.opacity(0.0)
                        ],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                )

                PointMark(
                    x: .value("Proposal", point.sequence),
                    y: .value("Expected delta", point.expectedDelta)
                )
                .foregroundStyle(ConvergioTokens.Brand.verdeRacing)
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

            // Bar chart: proposals grouped by status with semantic token colors
            if let roi {
                Chart(roi.proposalsByStatus) { bucket in
                    BarMark(
                        x: .value("Status", bucket.statusLabel),
                        y: .value("Count", bucket.count)
                    )
                    .foregroundStyle(
                        LinearGradient(
                            colors: [barColor(for: bucket.status), barColor(for: bucket.status).opacity(0.5)],
                            startPoint: .top,
                            endPoint: .bottom
                        )
                    )
                    .cornerRadius(6)
                }
                .frame(height: 180)

                // Metric summary pills
                HStack(spacing: Spacing.sm) {
                    metricPill("Experiments", value: "\(roi.experimentsRun)")
                    metricPill("Successes",   value: "\(roi.successes)")
                    metricPill("Rollbacks",   value: "\(roi.rollbacks)")
                }
            } else {
                ContentUnavailableView("No ROI data", systemImage: "chart.bar")
            }
        }
        .convergioCard()
    }

    // MARK: - Private helpers

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

    private func barColor(for status: String) -> Color {
        switch status {
        case "approved":  ConvergioTokens.Brand.verdeRacing
        case "rejected":  ConvergioTokens.Brand.rossoCorsa
        default:          ConvergioTokens.Brand.gialloFerrari
        }
    }

    private func metricPill(_ title: String, value: String) -> some View {
        VStack(alignment: .leading, spacing: Spacing.xxs) {
            Text(title)
                .font(.caption)
                .foregroundStyle(ConvergioTokens.Text.textMuted)
            Text(value)
                .font(.system(size: 22, weight: .bold, design: .rounded))
                .foregroundStyle(ConvergioTokens.Text.textPrimary)
        }
        .padding(Spacing.sm)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            .ultraThinMaterial,
            in: RoundedRectangle(cornerRadius: CornerRadius.md, style: .continuous)
        )
        .overlay(
            RoundedRectangle(cornerRadius: CornerRadius.md, style: .continuous)
                .strokeBorder(Color.white.opacity(0.07), lineWidth: 1)
        )
        .shadow(color: .black.opacity(0.12), radius: 6, x: 0, y: 2)
    }
}

// MARK: - ProposalTrendPoint

private struct ProposalTrendPoint: Identifiable {
    let id: Int
    let sequence: Int
    let label: String
    let expectedDelta: Double
}
