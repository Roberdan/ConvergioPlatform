import SwiftUI

struct ProposalDetailView: View {
    let proposal: EvolutionProposal
    let isSubmitting: Bool
    let onApprove: (String) -> Void
    let onReject: (String) -> Void

    @State private var reviewReason = ""

    var body: some View {
        VStack(alignment: .leading, spacing: Spacing.md) {
            // Header row: id + hypothesis + status badge
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 6) {
                    Text("Proposal #\(proposal.id)")
                        .font(.mono)
                        .foregroundStyle(ConvergioTokens.Text.textMuted)
                    Text(proposal.hypothesis)
                        .font(.title2.weight(.semibold))
                        .foregroundStyle(ConvergioTokens.Text.textPrimary)
                    Text(proposal.targetMetric)
                        .foregroundStyle(ConvergioTokens.Text.textMuted)
                }
                Spacer()
                StatusBadge(
                    label: proposal.statusLabel,
                    color: statusColor(for: proposal.status)
                )
            }

            // Summary tiles
            HStack(spacing: Spacing.sm) {
                summaryTile(
                    "Expected Δ",
                    value: String(format: "%.1f%%", proposal.expectedDeltaPercent),
                    icon: "chart.line.uptrend.xyaxis",
                    iconColor: ConvergioTokens.Brand.verdeRacing
                )
                summaryTile(
                    "Blast radius",
                    value: proposal.blastRadius ?? "Unknown",
                    icon: "bolt.circle",
                    iconColor: ConvergioTokens.Brand.arancioWarm
                )
                summaryTile(
                    "Reviewer",
                    value: proposal.reviewer ?? "Pending",
                    icon: "person.circle",
                    iconColor: ConvergioTokens.Brand.azzurro
                )
            }

            TextField("Review note", text: $reviewReason, axis: .vertical)
                .textFieldStyle(.roundedBorder)

            // Action buttons with semantic colors
            HStack(spacing: Spacing.sm) {
                Button {
                    onApprove(reviewReason)
                } label: {
                    Label("Approve", systemImage: "checkmark.circle.fill")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(ProposalActionButtonStyle(color: ConvergioTokens.Brand.verdeRacing))
                .disabled(!isPending)

                Button {
                    onReject(reviewReason)
                } label: {
                    Label("Reject", systemImage: "xmark.circle.fill")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(ProposalActionButtonStyle(color: ConvergioTokens.Brand.rossoCorsa))
                .disabled(!isPending)
            }

            if let note = proposal.reviewReason, !note.isEmpty {
                Text(note)
                    .font(.footnote)
                    .foregroundStyle(ConvergioTokens.Text.textMuted)
            }

            HStack {
                Text("Created \(proposal.createdAt ?? "—")")
                Spacer()
                Text("Updated \(proposal.updatedAt ?? "—")")
            }
            .font(.caption)
            .foregroundStyle(ConvergioTokens.Text.textMuted)
        }
        .convergioCard()
    }

    // MARK: - Private

    private var isPending: Bool {
        proposal.status == "pending" && !isSubmitting
    }

    private func statusColor(for status: String) -> Color {
        switch status {
        case "approved":  ConvergioTokens.Brand.verdeRacing
        case "rejected":  ConvergioTokens.Brand.rossoCorsa
        default:          ConvergioTokens.Brand.gialloFerrari
        }
    }

    private func summaryTile(
        _ title: String,
        value: String,
        icon: String,
        iconColor: Color
    ) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Label(title, systemImage: icon)
                .font(.caption)
                .foregroundStyle(iconColor)
            Text(value)
                .font(.headline)
                .foregroundStyle(ConvergioTokens.Text.textPrimary)
        }
        .padding(Spacing.sm)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            ConvergioTokens.Border.borderSubtle,
            in: RoundedRectangle(cornerRadius: CornerRadius.md, style: .continuous)
        )
    }
}

// MARK: - ProposalActionButtonStyle

/// Simple filled button style for Approve/Reject with a custom background color.
private struct ProposalActionButtonStyle: ButtonStyle {
    let color: Color

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.label)
            .foregroundStyle(.white)
            .padding(.horizontal, Spacing.md)
            .padding(.vertical, Spacing.xs)
            .background(
                RoundedRectangle(cornerRadius: CornerRadius.sm, style: .continuous)
                    .fill(configuration.isPressed ? color.opacity(0.75) : color)
            )
            .animation(.easeInOut(duration: 0.12), value: configuration.isPressed)
    }
}
