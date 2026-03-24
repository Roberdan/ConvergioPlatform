import Foundation
import Observation

@MainActor
@Observable
final class AgentCatalogModel {
    private let client: DaemonClient

    var catalog: [CatalogAgent] = []
    var runningAgents: [AgentRuntime] = []
    var selectedAgentName: String?
    var searchText = ""
    var problemDescription = ""
    var triageSuggestions: [AgentSuggestion] = []
    var isLoading = false
    var errorMessage: String?

    init(client: DaemonClient = DaemonClient()) {
        self.client = client
    }

    var filteredCatalog: [CatalogAgent] {
        guard !searchText.isEmpty else { return catalog }
        let needle = searchText.lowercased()
        return catalog.filter {
            $0.name.lowercased().contains(needle)
                || ($0.description?.lowercased().contains(needle) ?? false)
                || ($0.domain?.lowercased().contains(needle) ?? false)
                || ($0.category?.lowercased().contains(needle) ?? false)
        }
    }

    var selectedAgent: CatalogAgent? {
        filteredCatalog.first(where: { $0.name == selectedAgentName })
            ?? catalog.first(where: { $0.name == selectedAgentName })
    }

    func load() async {
        isLoading = true
        defer { isLoading = false }

        do {
            async let catalogResponse = client.agentCatalog()
            async let agentsResponse = client.agents()
            let (catalogData, agentsData) = try await (catalogResponse, agentsResponse)
            catalog = catalogData.agents.sorted { $0.name < $1.name }
            runningAgents = agentsData.running.sorted { $0.agentId < $1.agentId }
            if selectedAgentName == nil {
                selectedAgentName = catalog.first?.name
            }
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func toggleSelectedAgent() async {
        guard let selectedAgent else { return }
        do {
            if selectedAgent.isEnabled {
                _ = try await client.disableAgent(name: selectedAgent.name, targetDirectory: targetDirectory)
            } else {
                _ = try await client.enableAgent(name: selectedAgent.name, targetDirectory: targetDirectory)
            }
            await load()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func runTriage() async {
        guard !problemDescription.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return }
        do {
            let response = try await client.triageAgents(problemDescription: problemDescription)
            triageSuggestions = response.suggestions.sorted { ($0.score ?? 0) > ($1.score ?? 0) }
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private var targetDirectory: String {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("GitHub/ConvergioPlatform/.github/agents")
            .path
    }
}
