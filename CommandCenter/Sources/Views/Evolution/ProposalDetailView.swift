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
        VStack(alignment: .leading, spacing: Spacing.xs) {
            Image(systemName: icon)
                .font(.callout)
                .foregroundStyle(.white)
                .frame(width: 28, height: 28)
                .background(
                    Circle().fill(
                        LinearGradient(
                            colors: [iconColor, iconColor.opacity(0.6)],
                            startPoint: .topLeading,
                            endPoint: .bottomTrailing
                        )
                    )
                )
                .shadow(color: iconColor.opacity(0.35), radius: 4, x: 0, y: 2)

            Text(value)
                .font(.system(size: 20, weight: .bold, design: .rounded))
                .foregroundStyle(ConvergioTokens.Text.textPrimary)

            Text(title)
                .font(.caption)
                .foregroundStyle(ConvergioTokens.Text.textMuted)
        }
        .padding(Spacing.sm)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: CornerRadius.md, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: CornerRadius.md, style: .continuous)
                .strokeBorder(Color.white.opacity(0.07), lineWidth: 1)
        )
        .shadow(color: .black.opacity(0.12), radius: 6, x: 0, y: 2)
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
                    .fill(
                        LinearGradient(
                            colors: [
                                color,
                                color.opacity(0.7)
                            ],
                            startPoint: .topLeading,
                            endPoint: .bottomTrailing
                        )
                    )
            )
            .shadow(color: color.opacity(configuration.isPressed ? 0.2 : 0.45), radius: 8, x: 0, y: 3)
            .scaleEffect(configuration.isPressed ? 0.97 : 1.0)
            .animation(.spring(response: 0.2, dampingFraction: 0.7), value: configuration.isPressed)
    }
}
