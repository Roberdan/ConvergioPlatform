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
                        modelBadge(modelName)
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

            Text(agent.description ?? "No description available.")
                .foregroundStyle(.secondary)

            detailRow("Category", agent.category ?? "—")
            detailRow("Domain", agent.domain ?? "—")
            detailRow("Tools", (agent.tools ?? []).joined(separator: ", ").ifEmpty("—"))
            detailRow("Path", agent.path ?? "Not deployed")
        }
        .modifier(ConvergioCardModifier())
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

    @ViewBuilder
    private func modelBadge(_ modelName: String) -> some View {
        let lower = modelName.lowercased()
        let (bg, fg): (Color, Color) = {
            if lower.contains("opus") {
                return (ConvergioTokens.Brand.gialloFerrari, Color(red: 0.067, green: 0.067, blue: 0.067))
            } else if lower.contains("haiku") {
                return (ConvergioTokens.Brand.arancioWarm, .white)
            } else {
                return (ConvergioTokens.Roles.role10, .white)
            }
        }()
        Text(modelName)
            .font(.label)
            .foregroundStyle(fg)
            .padding(.horizontal, Spacing.xs)
            .padding(.vertical, Spacing.xxs)
            .background(Capsule().fill(bg))
    }
}

private extension String {
    func ifEmpty(_ fallback: String) -> String {
        isEmpty ? fallback : self
    }
}
