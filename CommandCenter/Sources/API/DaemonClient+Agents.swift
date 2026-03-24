import Foundation

extension DaemonClient {
    func agentCatalog() async throws -> AgentCatalogResponse {
        try await get("/api/agents/catalog")
    }

    func enableAgent(name: String, targetDirectory: String) async throws -> AgentToggleResponse {
        try await post(
            "/api/agents/enable",
            body: AgentToggleRequest(name: name, targetDir: targetDirectory)
        )
    }

    func disableAgent(name: String, targetDirectory: String) async throws -> AgentToggleResponse {
        try await post(
            "/api/agents/disable",
            body: AgentToggleRequest(name: name, targetDir: targetDirectory)
        )
    }

    func triageAgents(problemDescription: String, domain: String? = nil) async throws -> AgentTriageResponse {
        try await post(
            "/api/agents/triage",
            body: AgentTriageRequest(problemDescription: problemDescription, domain: domain)
        )
    }
}
