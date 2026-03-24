import Charts
import SwiftUI

struct ModelCostBreakdown: View {
    let models: [ModelCostDatum]
    let trend: [CostTrendDatum]
    let rangeTitle: String

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Model Cost Breakdown")
                        .font(.title2.weight(.semibold))
                    Text("\(rangeTitle) spend grouped into Opus, Sonnet, Haiku, and Codex families.")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }
                Spacer()
            }

            HStack(alignment: .top, spacing: 24) {
                Chart(models) { datum in
                    BarMark(
                        x: .value("Family", datum.family),
                        y: .value("Cost", datum.costUsd)
                    )
                    .foregroundStyle(.blue.gradient)
                    .annotation(position: .top) {
                        Text("\(datum.calls)x")
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                    }
                }
                .frame(height: 240)

                if !trend.isEmpty {
                    Chart(trend) { point in
                        AreaMark(
                            x: .value("Date", point.date),
                            y: .value("Cost", point.costUsd)
                        )
                        .foregroundStyle(.cyan.opacity(0.18))

                        LineMark(
                            x: .value("Date", point.date),
                            y: .value("Cost", point.costUsd)
                        )
                        .foregroundStyle(.cyan)
                    }
                    .frame(height: 240)
                }
            }
        }
        .padding(20)
        .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 24, style: .continuous))
    }
}
