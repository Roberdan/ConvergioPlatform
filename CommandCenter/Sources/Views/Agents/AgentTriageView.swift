import SwiftUI

struct AgentTriageView: View {
    @Binding var problemDescription: String
    let suggestions: [AgentSuggestion]
    let onRun: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Agent triage")
                .font(.headline)

            TextEditor(text: $problemDescription)
                .frame(minHeight: 100)
                .padding(8)
                .background(.quaternary.opacity(0.15), in: RoundedRectangle(cornerRadius: 16, style: .continuous))

            HStack {
                Spacer()
                Button("Recommend agents") {
                    onRun()
                }
                .buttonStyle(.borderedProminent)
            }

            ForEach(suggestions.prefix(5)) { suggestion in
                suggestionCard(suggestion)
            }
        }
        .modifier(ConvergioCardModifier())
    }

    // MARK: - Private

    private func suggestionCard(_ suggestion: AgentSuggestion) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text(suggestion.name)
                    .font(.subheadline.weight(.semibold))
                Spacer()
                // Score displayed as a StatusBadge: color reflects confidence level
                StatusBadge(
                    label: String(format: "%.2f", suggestion.score ?? 0),
                    color: scoreColor(suggestion.score ?? 0)
                )
            }
            Text(suggestion.reason ?? suggestion.description ?? "No rationale provided.")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(Spacing.xs)
        .modifier(ConvergioCardModifier())
    }

    /// Maps score (0–1) to a semantic color: success, warning, or error.
    private func scoreColor(_ score: Double) -> Color {
        if score >= 0.7 { return ConvergioTokens.Status.success }
        if score >= 0.4 { return ConvergioTokens.Status.warning }
        return ConvergioTokens.Status.error
    }
}
