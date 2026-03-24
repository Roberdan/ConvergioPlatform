import SwiftUI

struct ProposalDetailView: View {
    let proposal: EvolutionProposal
    let isSubmitting: Bool
    let onApprove: (String) -> Void
    let onReject: (String) -> Void

    @State private var reviewReason = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 6) {
                    Text("Proposal #\(proposal.id)")
                        .font(.caption.monospaced())
                        .foregroundStyle(.secondary)
                    Text(proposal.hypothesis)
                        .font(.title2.weight(.semibold))
                    Text(proposal.targetMetric)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Text(proposal.statusLabel)
                    .font(.caption.weight(.semibold))
                    .padding(.horizontal, 10)
                    .padding(.vertical, 6)
                    .background(.quaternary, in: Capsule())
            }

            HStack(spacing: 12) {
                summaryTile("Expected Δ", value: String(format: "%.1f%%", proposal.expectedDeltaPercent))
                summaryTile("Blast radius", value: proposal.blastRadius ?? "Unknown")
                summaryTile("Reviewer", value: proposal.reviewer ?? "Pending")
            }

            TextField("Review note", text: $reviewReason, axis: .vertical)
                .textFieldStyle(.roundedBorder)

            HStack(spacing: 12) {
                Button {
                    onApprove(reviewReason)
                } label: {
                    Label("Approve", systemImage: "checkmark.circle.fill")
                }
                .disabled(!isPending)

                Button(role: .destructive) {
                    onReject(reviewReason)
                } label: {
                    Label("Reject", systemImage: "xmark.circle.fill")
                }
                .disabled(!isPending)
            }

            if let reviewReason = proposal.reviewReason, !reviewReason.isEmpty {
                Text(reviewReason)
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }

            HStack {
                Text("Created \(proposal.createdAt ?? "—")")
                Spacer()
                Text("Updated \(proposal.updatedAt ?? "—")")
            }
            .font(.caption)
            .foregroundStyle(.secondary)
        }
        .padding(20)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 24, style: .continuous))
    }

    private var isPending: Bool {
        proposal.status == "pending" && !isSubmitting
    }

    private func summaryTile(_ title: String, value: String) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title)
                .font(.caption)
                .foregroundStyle(.secondary)
            Text(value)
                .font(.headline)
        }
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.quaternary.opacity(0.12), in: RoundedRectangle(cornerRadius: 18, style: .continuous))
    }
}
