import SwiftUI

struct ExperimentView: View {
    let experiments: [EvolutionExperiment]

    var body: some View {
        VStack(alignment: .leading, spacing: Spacing.md) {
            HStack {
                Text("Experiments")
                    .font(.sectionTitle)
                    .foregroundStyle(ConvergioTokens.Text.textPrimary)
                Spacer()
                StatusBadge(
                    label: "\(experiments.count)",
                    color: ConvergioTokens.Brand.azzurro
                )
            }

            if experiments.isEmpty {
                ContentUnavailableView("No experiments", systemImage: "flask")
            } else {
                ForEach(experiments) { experiment in
                    experimentCard(experiment)
                }
            }
        }
        .convergioCard()
    }

    // MARK: - Private

    private func experimentCard(_ experiment: EvolutionExperiment) -> some View {
        VStack(alignment: .leading, spacing: Spacing.sm) {
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(experiment.hypothesis ?? "Experiment #\(experiment.id)")
                        .font(.cardTitle)
                        .foregroundStyle(ConvergioTokens.Text.textPrimary)
                    Text(experiment.targetMetric ?? experiment.mode.capitalized)
                        .font(.subheadline)
                        .foregroundStyle(ConvergioTokens.Text.textMuted)
                }
                Spacer()
                StatusBadge(
                    label: experiment.resultLabel,
                    color: resultColor(experiment.result)
                )
            }

            HStack(alignment: .top, spacing: Spacing.md) {
                metricColumn("Before", values: experiment.beforeValues)
                metricColumn("After",  values: experiment.afterValues)
            }

            HStack {
                Label(experiment.mode.capitalized, systemImage: "flask")
                    .foregroundStyle(ConvergioTokens.Brand.petrolio)
                Spacer()
                Text(experiment.startedAt ?? "No start time")
            }
            .font(.caption)
            .foregroundStyle(ConvergioTokens.Text.textMuted)
        }
        .padding(Spacing.md)
        .background(
            ConvergioTokens.Border.borderSubtle,
            in: RoundedRectangle(cornerRadius: CornerRadius.md, style: .continuous)
        )
        .overlay(
            RoundedRectangle(cornerRadius: CornerRadius.md, style: .continuous)
                .strokeBorder(
                    resultColor(experiment.result).opacity(0.3),
                    lineWidth: 1
                )
        )
    }

    private func metricColumn(_ title: String, values: [MetricValue]) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(ConvergioTokens.Text.textPrimary)

            if values.isEmpty {
                Text("No metrics captured")
                    .font(.footnote)
                    .foregroundStyle(ConvergioTokens.Text.textMuted)
            } else {
                ForEach(values) { value in
                    HStack {
                        Text(value.key)
                        Spacer()
                        Text(value.value)
                            .foregroundStyle(ConvergioTokens.Text.textMuted)
                    }
                    .font(.footnote)
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    /// Maps experiment result string to a semantic brand token color.
    private func resultColor(_ result: String) -> Color {
        switch result {
        case "success":   ConvergioTokens.Brand.verdeRacing
        case "failure", "rollback": ConvergioTokens.Brand.rossoCorsa
        case "running":   ConvergioTokens.Brand.gialloFerrari
        default:          ConvergioTokens.Brand.azzurro
        }
    }
}
