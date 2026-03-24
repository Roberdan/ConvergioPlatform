import SwiftUI

struct AgentDetailView: View {
    let agent: CatalogAgent
    let onToggle: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text(agent.name)
                        .font(.title2.weight(.semibold))
                    Text(agent.model ?? "Model not declared")
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Button(agent.isEnabled ? "Disable" : "Enable") {
                    onToggle()
                }
                .buttonStyle(.borderedProminent)
            }

            Text(agent.description ?? "No description available.")
                .foregroundStyle(.secondary)

            detailRow("Category", agent.category ?? "—")
            detailRow("Domain", agent.domain ?? "—")
            detailRow("Tools", (agent.tools ?? []).joined(separator: ", ").ifEmpty("—"))
            detailRow("Path", agent.path ?? "Not deployed")
        }
        .padding(20)
        .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 22, style: .continuous))
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
