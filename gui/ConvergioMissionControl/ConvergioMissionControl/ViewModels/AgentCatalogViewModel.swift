// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — ViewModel for the agent catalog with auto-refresh

import Foundation
import Observation

/// Observable view model for the agent catalog.
/// Fetches from `GET /api/agents` every 10 seconds and groups by status.
@Observable
@MainActor
final class AgentCatalogViewModel {

    // MARK: - State

    var allAgents: [DaemonAgent] = []
    var isLoading: Bool = false
    var error: String?

    // MARK: - Computed

    /// Agents currently running (status == "running")
    var activeAgents: [DaemonAgent] {
        allAgents.filter { $0.status == "running" }
    }

    /// Agents that have completed (status == "complete")
    var completedAgents: [DaemonAgent] {
        allAgents.filter { $0.status == "complete" }
    }

    /// Sum of cost_usd across all agents
    var totalCost: Double {
        allAgents.reduce(0) { $0 + $1.costUsd }
    }

    // MARK: - Private

    private let baseURL: URL
    private var refreshTask: Task<Void, Never>?

    init(baseURL: URL = URL(string: "http://localhost:8420")!) {
        self.baseURL = baseURL
    }

    // MARK: - Lifecycle

    /// Start fetching agents and schedule 10-second auto-refresh.
    func startPolling() {
        refreshTask?.cancel()
        refreshTask = Task { [weak self] in
            while !Task.isCancelled {
                await self?.fetchAgents()
                try? await Task.sleep(for: .seconds(10))
            }
        }
    }

    /// Stop auto-refresh (call on view disappear).
    func stopPolling() {
        refreshTask?.cancel()
        refreshTask = nil
    }

    // MARK: - Fetch

    /// Fetch once from the daemon REST API.
    func fetchAgents() async {
        isLoading = allAgents.isEmpty
        let url = baseURL.appendingPathComponent("api/agents")
        do {
            let (data, _) = try await URLSession.shared.data(from: url)
            let response = try JSONDecoder().decode(DaemonAgentResponse.self, from: data)
            allAgents = response.running + response.recent
            error = nil
        } catch {
            // Warn loudly — silent null returns are bugs
            let message = "Failed to fetch agents: \(error.localizedDescription)"
            self.error = message
            print("[AgentCatalogViewModel] WARN: \(message)")
        }
        isLoading = false
    }
}
