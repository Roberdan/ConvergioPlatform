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
                    LazyVGrid(
                        columns: [GridItem(.adaptive(minimum: 240), spacing: 18)],
                        spacing: 18
                    ) {
                        ForEach(model.filteredCatalog) { agent in
                            Button {
                                model.selectedAgentName = agent.name
                            } label: {
                                AgentCardView(agent: agent)
                            }
                            .buttonStyle(.plain)
                            .accessibilityLabel(
                                "Agent \(agent.name), \(agent.isEnabled ? "enabled" : "disabled")"
                            )
                        }
                    }
                }
            }
            .padding(24)
            .frame(minWidth: 380)

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
}
