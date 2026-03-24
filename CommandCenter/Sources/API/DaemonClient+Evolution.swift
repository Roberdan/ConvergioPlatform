import Foundation

extension DaemonClient {
    func evolutionProposals() async throws -> EvolutionProposalsResponse {
        try await get("/api/evolution/proposals")
    }

    func approveEvolutionProposal(
        id: Int,
        reason: String,
        actor: String = "CommandCenter"
    ) async throws -> EvolutionProposalDecisionResponse {
        try await post(
            "/api/evolution/proposals/\(id)/approve",
            body: EvolutionProposalDecisionRequest(reason: reason, actor: actor)
        )
    }

    func rejectEvolutionProposal(
        id: Int,
        reason: String,
        actor: String = "CommandCenter"
    ) async throws -> EvolutionProposalDecisionResponse {
        try await post(
            "/api/evolution/proposals/\(id)/reject",
            body: EvolutionProposalDecisionRequest(reason: reason, actor: actor)
        )
    }

    func evolutionExperiments() async throws -> EvolutionExperimentsResponse {
        try await get("/api/evolution/experiments")
    }

    func evolutionROI() async throws -> EvolutionROIResponse {
        try await get("/api/evolution/roi")
    }

    func evolutionAudit(proposalID: Int) async throws -> EvolutionAuditResponse {
        try await get("/api/evolution/audit/\(proposalID)")
    }
}
