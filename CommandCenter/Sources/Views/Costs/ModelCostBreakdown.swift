import Charts
import SwiftUI

struct ModelCostBreakdown: View {
    let models: [ModelCostDatum]
    let trend: [CostTrendDatum]
    let rangeTitle: String

    // Chart color per model family: opus=gialloFerrari, sonnet=role10(Pietra),
    // haiku=arancioWarm, codex=azzurro
    private func barColor(for family: String) -> Color {
        switch family {
        case "Opus":   return ConvergioTokens.Brand.gialloFerrari
        case "Sonnet": return ConvergioTokens.Roles.role10
        case "Haiku":  return ConvergioTokens.Brand.arancioWarm
        case "Codex":  return ConvergioTokens.Brand.azzurro
        default:       return ConvergioTokens.Text.textMuted
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: Spacing.md) {
            sectionHeader
            chartRow
        }
        .modifier(ConvergioCardModifier())
    }

    // MARK: — Section header

    private var sectionHeader: some View {
        HStack {
            VStack(alignment: .leading, spacing: Spacing.xxs) {
                Text("Model Cost Breakdown")
                    .font(.sectionTitle)
                Text("\(rangeTitle) spend grouped into Opus, Sonnet, Haiku, and Codex families.")
                    .font(.subheadline)
                    .foregroundStyle(ConvergioTokens.Text.textMuted)
            }
            Spacer()
        }
    }

    // MARK: — Chart row

    private var chartRow: some View {
        HStack(alignment: .top, spacing: Spacing.lg) {
            barChart
            if !trend.isEmpty {
                areaChart
            }
        }
    }

    // Bar chart: each family bar uses gradient fill with rounded corners
    private var barChart: some View {
        Chart(models) { datum in
            BarMark(
                x: .value("Family", datum.family),
                y: .value("Cost", datum.costUsd)
            )
            .foregroundStyle(
                LinearGradient(
                    colors: [barColor(for: datum.family), barColor(for: datum.family).opacity(0.5)],
                    startPoint: .top,
                    endPoint: .bottom
                )
            )
            .cornerRadius(6)
            .annotation(position: .top) {
                Text("\(datum.calls)x")
                    .font(.caption2.weight(.semibold))
                    .foregroundStyle(ConvergioTokens.Text.textMuted)
                    .padding(.horizontal, 4)
                    .padding(.vertical, 2)
                    .background(.ultraThinMaterial, in: Capsule())
            }
        }
        .frame(height: 240)
    }

    // Area chart: gradient from azzurro to azzurro.opacity(0.1), line in azzurro
    private var areaChart: some View {
        Chart(trend) { point in
            AreaMark(
                x: .value("Date", point.date),
                y: .value("Cost", point.costUsd)
            )
            .foregroundStyle(
                LinearGradient(
                    colors: [
                        ConvergioTokens.Brand.azzurro,
                        ConvergioTokens.Brand.azzurro.opacity(0.1)
                    ],
                    startPoint: .top,
                    endPoint: .bottom
                )
            )

            LineMark(
                x: .value("Date", point.date),
                y: .value("Cost", point.costUsd)
            )
            .foregroundStyle(ConvergioTokens.Brand.azzurro)
        }
        .frame(height: 240)
    }
}
