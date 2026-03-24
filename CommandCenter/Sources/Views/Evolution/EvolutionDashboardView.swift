import Foundation
import SwiftUI

struct EvolutionDashboardView: View {
    @State private var model: EvolutionDashboardModel

    init(client: DaemonClient = DaemonClient()) {
        _model = State(initialValue: EvolutionDashboardModel(client: client))
    }

    var body: some View {
        HSplitView {
            VStack(alignment: .leading, spacing: Spacing.md) {
                header

                ScrollView {
                    LazyVStack(spacing: Spacing.sm) {
                        ForEach(model.proposals) { proposal in
                            Button {
                                Task { await model.selectProposal(proposal) }
                            } label: {
                                proposalCard(proposal)
                            }
                            .buttonStyle(.plain)
                            .accessibilityLabel("Proposal \(proposal.id): \(proposal.hypothesis), Status: \(proposal.statusLabel)")
                        }
                    }
                }
            }
            .padding(Spacing.lg)
            .frame(minWidth: 320, idealWidth: 360)

            if let proposal = model.selectedProposal {
                ScrollView {
                    VStack(alignment: .leading, spacing: 20) {
                        ProposalDetailView(
                            proposal: proposal,
                            isSubmitting: model.isSubmitting,
                            onApprove: { reason in Task { await model.approveSelected(reason: reason) } },
                            onReject: { reason in Task { await model.rejectSelected(reason: reason) } }
                        )
                        .id(proposal.id)

                        ROIChartView(roi: model.roi, proposals: model.proposals)
                        ExperimentView(experiments: model.selectedExperiments)
                        AuditTrailView(entries: model.audit)
                    }
                    .padding(Spacing.lg)
                }
            } else {
                ContentUnavailableView("No proposals", systemImage: "chart.line.uptrend.xyaxis")
            }
        }
        .task { await model.load() }
        .overlay(alignment: .topTrailing) {
            if let errorMessage = model.errorMessage {
                Text(errorMessage)
                    .font(.footnote)
                    .foregroundStyle(ConvergioTokens.Status.error)
                    .padding(Spacing.sm)
            }
        }
    }

    // MARK: - Header

    private var header: some View {
        HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: 6) {
                Text("Evolution Metrics")
                    .font(.heroTitle)
                Text("Native dashboard backed by /api/evolution/* endpoints.")
                    .font(.subheadline)
                    .foregroundStyle(ConvergioTokens.Text.textMuted)
            }

            Spacer()

            VStack(alignment: .trailing, spacing: Spacing.xs) {
                HStack(spacing: Spacing.xs) {
                    StatusBadge(
                        label: "\(model.proposals.count) proposals",
                        color: ConvergioTokens.Brand.azzurro
                    )
                    .accessibilityLabel("Status: \(model.proposals.count) proposals")
                    StatusBadge(
                        label: "\(model.experiments.count) experiments",
                        color: ConvergioTokens.Brand.petrolio
                    )
                    .accessibilityLabel("Status: \(model.experiments.count) experiments")
                    if let lastRefresh = model.lastRefresh {
                        StatusBadge(
                            label: RelativeDateTimeFormatter().localizedString(for: lastRefresh, relativeTo: .now),
                            color: ConvergioTokens.Brand.indaco
                        )
                        .accessibilityLabel("Status: Last refreshed \(RelativeDateTimeFormatter().localizedString(for: lastRefresh, relativeTo: .now))")
                    }
                }

                Button {
                    Task { await model.refresh() }
                } label: {
                    Label("Refresh", systemImage: "arrow.clockwise")
                }
                .accessibilityLabel("Refresh evolution data")
            }
        }
    }

    // MARK: - Proposal card

    @State private var hoveredProposalID: Int?

    private func proposalCard(_ proposal: EvolutionProposal) -> some View {
        let borderColor = proposalBorderColor(proposal.status)
        let isSelected = model.selectedProposalID == proposal.id
        let isHovered = hoveredProposalID == proposal.id

        return HStack(spacing: 0) {
            // Left accent: gradient border encoding status
            RoundedRectangle(cornerRadius: 2, style: .continuous)
                .fill(
                    LinearGradient(
                        colors: [borderColor, borderColor.opacity(0.4)],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                )
                .frame(width: 4)
                .padding(.vertical, 6)

            VStack(alignment: .leading, spacing: Spacing.xs) {
                HStack {
                    Text(proposal.hypothesis)
                        .font(.cardTitle)
                        .lineLimit(2)
                        .foregroundStyle(ConvergioTokens.Text.textPrimary)
                    Spacer()
                    StatusBadge(
                        label: proposal.statusLabel,
                        color: borderColor
                    )
                    .accessibilityLabel("Status: \(proposal.statusLabel)")
                }

                Text(proposal.targetMetric)
                    .font(.subheadline)
                    .foregroundStyle(ConvergioTokens.Text.textMuted)

                HStack {
                    Label(
                        String(format: "%.1f%% \u{0394}", proposal.expectedDeltaPercent),
                        systemImage: "chart.line.uptrend.xyaxis"
                    )
                    .foregroundStyle(ConvergioTokens.Brand.verdeRacing)
                    Spacer()
                    Text("#\(proposal.id)")
                        .font(.mono)
                        .foregroundStyle(ConvergioTokens.Text.textMuted)
                }
                .font(.caption)
            }
            .padding(Spacing.md)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            isSelected
                ? ConvergioTokens.Brand.azzurro.opacity(0.12)
                : Color.clear,
            in: RoundedRectangle(cornerRadius: CornerRadius.md, style: .continuous)
        )
        .background(
            .regularMaterial,
            in: RoundedRectangle(cornerRadius: CornerRadius.md, style: .continuous)
        )
        .overlay(
            RoundedRectangle(cornerRadius: CornerRadius.md, style: .continuous)
                .strokeBorder(
                    isSelected
                        ? borderColor.opacity(0.4)
                        : Color.white.opacity(0.07),
                    lineWidth: 1
                )
        )
        .shadow(color: .black.opacity(isHovered ? 0.3 : 0.15), radius: isHovered ? 12 : 6, y: isHovered ? 4 : 2)
        .scaleEffect(isHovered ? 1.015 : 1.0)
        .animation(.spring(response: 0.25, dampingFraction: 0.8), value: isHovered)
        .onHover { hovering in hoveredProposalID = hovering ? proposal.id : nil }
    }

    // MARK: - Helpers

    private func proposalBorderColor(_ status: String) -> Color {
        switch status {
        case "approved":  ConvergioTokens.Brand.verdeRacing
        case "rejected":  ConvergioTokens.Brand.rossoCorsa
        default:          ConvergioTokens.Brand.gialloFerrari
        }
    }
}
