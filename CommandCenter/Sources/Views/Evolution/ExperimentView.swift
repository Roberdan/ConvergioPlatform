import SwiftUI

struct ExperimentView: View {
    let experiments: [EvolutionExperiment]

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Text("Experiments")
                    .font(.title3.weight(.semibold))
                Spacer()
                Text("\(experiments.count)")
                    .font(.caption.weight(.semibold))
                    .padding(.horizontal, 10)
                    .padding(.vertical, 6)
                    .background(.quaternary, in: Capsule())
            }

            if experiments.isEmpty {
                ContentUnavailableView("No experiments", systemImage: "flask")
            } else {
                ForEach(experiments) { experiment in
                    VStack(alignment: .leading, spacing: 12) {
                        HStack(alignment: .top) {
                            VStack(alignment: .leading, spacing: 4) {
                                Text(experiment.hypothesis ?? "Experiment #\(experiment.id)")
                                    .font(.headline)
                                Text(experiment.targetMetric ?? experiment.mode.capitalized)
                                    .font(.subheadline)
                                    .foregroundStyle(.secondary)
                            }
                            Spacer()
                            Text(experiment.resultLabel)
                                .font(.caption.weight(.semibold))
                                .padding(.horizontal, 10)
                                .padding(.vertical, 6)
                                .background(.quaternary, in: Capsule())
                        }

                        HStack(alignment: .top, spacing: 16) {
                            metricColumn("Before", values: experiment.beforeValues)
                            metricColumn("After", values: experiment.afterValues)
                        }

                        HStack {
                            Label(experiment.mode.capitalized, systemImage: "flask")
                            Spacer()
                            Text(experiment.startedAt ?? "No start time")
                        }
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    }
                    .padding(16)
                    .background(.quaternary.opacity(0.12), in: RoundedRectangle(cornerRadius: 18, style: .continuous))
                }
            }
        }
        .padding(20)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 24, style: .continuous))
    }

    private func metricColumn(_ title: String, values: [MetricValue]) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(.subheadline.weight(.semibold))

            if values.isEmpty {
                Text("No metrics captured")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            } else {
                ForEach(values) { value in
                    HStack {
                        Text(value.key)
                        Spacer()
                        Text(value.value).foregroundStyle(.secondary)
                    }
                    .font(.footnote)
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}
