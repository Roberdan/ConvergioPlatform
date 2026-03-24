import Foundation
import Observation

@MainActor
@Observable
final class EvolutionDashboardModel {
    private let client: DaemonClient

    var proposals: [EvolutionProposal] = []
    var experiments: [EvolutionExperiment] = []
    var roi: EvolutionROIResponse?
    var audit: [EvolutionAuditEntry] = []
    var selectedProposalID: Int?
    var lastRefresh: Date?
    var isLoading = false
    var isSubmitting = false
    var errorMessage: String?

    init(client: DaemonClient = DaemonClient()) {
        self.client = client
    }

    var selectedProposal: EvolutionProposal? {
        if let selectedProposalID, let proposal = proposals.first(where: { $0.id == selectedProposalID }) {
            return proposal
        }
        return proposals.first
    }

    var selectedExperiments: [EvolutionExperiment] {
        guard let selectedProposal else { return experiments }
        let filtered = experiments.filter { $0.proposalId == selectedProposal.id }
        return filtered.isEmpty ? experiments : filtered
    }

    func load() async {
        await refresh()
    }

    func refresh() async {
        isLoading = true
        defer { isLoading = false }

        do {
            async let proposalsResponse = client.evolutionProposals()
            async let experimentsResponse = client.evolutionExperiments()
            async let roiResponse = client.evolutionROI()
            let (proposalData, experimentData, roiData) = try await (
                proposalsResponse,
                experimentsResponse,
                roiResponse
            )
            proposals = proposalData.proposals.sorted { $0.id > $1.id }
            experiments = experimentData.experiments.sorted { $0.id > $1.id }
            roi = roiData
            if let selectedProposalID, proposals.contains(where: { $0.id == selectedProposalID }) {
                self.selectedProposalID = selectedProposalID
            } else {
                self.selectedProposalID = proposals.first?.id
            }
            lastRefresh = Date()
            await loadAudit()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func selectProposal(_ proposal: EvolutionProposal) async {
        selectedProposalID = proposal.id
        await loadAudit()
    }

    func approveSelected(reason: String) async {
        await actOnSelected(reason: reason, approve: true)
    }

    func rejectSelected(reason: String) async {
        await actOnSelected(reason: reason, approve: false)
    }

    private func loadAudit() async {
        guard let selectedProposalID else {
            audit = []
            return
        }

        do {
            let response = try await client.evolutionAudit(proposalID: selectedProposalID)
            audit = response.audit
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func actOnSelected(reason: String, approve: Bool) async {
        guard let proposal = selectedProposal else { return }
        isSubmitting = true
        defer { isSubmitting = false }

        let trimmed = reason.trimmingCharacters(in: .whitespacesAndNewlines)
        let fallback = approve ? "approved via CommandCenter" : "rejected via CommandCenter"

        do {
            if approve {
                _ = try await client.approveEvolutionProposal(
                    id: proposal.id,
                    reason: trimmed.isEmpty ? fallback : trimmed
                )
            } else {
                _ = try await client.rejectEvolutionProposal(
                    id: proposal.id,
                    reason: trimmed.isEmpty ? fallback : trimmed
                )
            }
            await refresh()
        } catch {
            errorMessage = error.localizedDescription
        }
    }
}
