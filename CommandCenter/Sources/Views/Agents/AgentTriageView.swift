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
                VStack(alignment: .leading, spacing: 4) {
                    HStack {
                        Text(suggestion.name)
                            .font(.subheadline.weight(.semibold))
                        Spacer()
                        Text(String(format: "%.2f", suggestion.score ?? 0))
                            .font(.caption.monospacedDigit())
                    }
                    Text(suggestion.reason ?? suggestion.description ?? "No rationale provided.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                .padding(.vertical, 4)
            }
        }
        .padding(20)
        .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 22, style: .continuous))
    }
}
