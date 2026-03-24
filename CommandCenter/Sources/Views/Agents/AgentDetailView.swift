import SwiftUI

struct AgentDetailView: View {
    let agent: CatalogAgent
    let onToggle: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack {
                VStack(alignment: .leading, spacing: 6) {
                    Text(agent.name)
                        .modifier(SectionHeaderModifier())

                    if let modelName = agent.model {
                        PremiumModelBadge(modelName: modelName)
                    } else {
                        Text("Model not declared")
                            .foregroundStyle(.secondary)
                    }
                }
                Spacer()
                Button(agent.isEnabled ? "Disable" : "Enable") {
                    onToggle()
                }
                .buttonStyle(.borderedProminent)
            }

            // Accent divider
            Rectangle()
                .fill(accentGradient)
                .frame(height: 1)

            Text(agent.description ?? "No description available.")
                .foregroundStyle(.secondary)

            detailRow("Category", agent.category ?? "---")
            detailRow("Domain", agent.domain ?? "---")
            detailRow("Tools", toolsText)
            detailRow("Path", agent.path ?? "Not deployed")
        }
        .modifier(ConvergioCardModifier())
        .shadow(color: .black.opacity(0.15), radius: 12, y: 4)
        .background(
            RoundedRectangle(cornerRadius: CornerRadius.lg, style: .continuous)
                .fill(.ultraThinMaterial)
        )
    }

    private var toolsText: String {
        (agent.tools ?? []).joined(separator: ", ").ifEmpty("---")
    }

    private var accentGradient: LinearGradient {
        LinearGradient(
            colors: [
                ConvergioTokens.Brand.gialloFerrari.opacity(0.6),
                ConvergioTokens.Brand.gialloFerrari.opacity(0.0)
            ],
            startPoint: .leading,
            endPoint: .trailing
        )
    }

    private func detailRow(_ title: String, _ value: String) -> some View {
        HStack(alignment: .top) {
            Text(title)
                .font(.headline)
            Spacer()
            Text(value)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.trailing)
        }
    }
}

private extension String {
    func ifEmpty(_ fallback: String) -> String {
        isEmpty ? fallback : self
    }
}
