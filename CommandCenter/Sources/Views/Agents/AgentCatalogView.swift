import Observation
import SwiftUI

struct AgentCatalogView: View {
    @State private var model: AgentCatalogModel

    init(client: DaemonClient = DaemonClient()) {
        _model = State(initialValue: AgentCatalogModel(client: client))
    }

    var body: some View {
        HSplitView {
            VStack(spacing: 16) {
                TextField("Search agents", text: $model.searchText)
                    .textFieldStyle(.roundedBorder)

                ScrollView {
                    LazyVGrid(columns: [GridItem(.adaptive(minimum: 220), spacing: 12)], spacing: 12) {
                        ForEach(model.filteredCatalog) { agent in
                            Button {
                                model.selectedAgentName = agent.name
                            } label: {
                                agentCard(agent)
                            }
                            .buttonStyle(.plain)
                        }
                    }
                }
            }
            .padding(24)
            .frame(minWidth: 360)

            VStack(spacing: 20) {
                if let agent = model.selectedAgent {
                    AgentDetailView(agent: agent) {
                        Task { await model.toggleSelectedAgent() }
                    }
                } else {
                    ContentUnavailableView("Select an agent", systemImage: "person.3.sequence")
                }

                AgentTriageView(
                    problemDescription: $model.problemDescription,
                    suggestions: model.triageSuggestions
                ) {
                    Task { await model.runTriage() }
                }

                SessionMonitor(runningAgents: model.runningAgents)
            }
            .padding(24)
        }
        .task { await model.load() }
        .overlay(alignment: .topTrailing) {
            if let errorMessage = model.errorMessage {
                Text(errorMessage)
                    .font(.footnote)
                    .foregroundStyle(.red)
                    .padding(12)
            }
        }
    }

    private func agentCard(_ agent: CatalogAgent) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text(agent.name)
                    .font(.headline)
                Spacer()
                Circle()
                    .fill(agent.isEnabled ? .green : .gray)
                    .frame(width: 10, height: 10)
            }

            Text(agent.description ?? "No description available.")
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .lineLimit(3)

            HStack {
                ForEach(agent.badges, id: \.self) { badge in
                    Text(badge)
                        .font(.caption)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 4)
                        .background(.quaternary, in: Capsule())
                }
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .modifier(AgentGlassModifier())
    }
}

private struct AgentGlassModifier: ViewModifier {
    func body(content: Content) -> some View {
        if #available(macOS 26.0, *) {
            content.glassEffect()
        } else {
            content.background(.regularMaterial, in: RoundedRectangle(cornerRadius: 22, style: .continuous))
        }
    }
}
