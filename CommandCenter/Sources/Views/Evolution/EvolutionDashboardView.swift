import Foundation
import SwiftUI

struct EvolutionDashboardView: View {
    @State private var model: EvolutionDashboardModel

    init(client: DaemonClient = DaemonClient()) {
        _model = State(initialValue: EvolutionDashboardModel(client: client))
    }

    var body: some View {
        HSplitView {
            VStack(alignment: .leading, spacing: 16) {
                header

                ScrollView {
                    LazyVStack(spacing: 12) {
                        ForEach(model.proposals) { proposal in
                            Button {
                                Task { await model.selectProposal(proposal) }
                            } label: {
                                proposalCard(proposal)
                            }
                            .buttonStyle(.plain)
                        }
                    }
                }
            }
            .padding(24)
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
                    .padding(24)
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
                    .foregroundStyle(.red)
                    .padding(12)
            }
        }
    }

    private var header: some View {
        HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: 6) {
                Text("Evolution Metrics")
                    .font(.largeTitle.weight(.bold))
                Text("Native dashboard backed by /api/evolution/* endpoints.")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }

            Spacer()

            VStack(alignment: .trailing, spacing: 10) {
                HStack(spacing: 10) {
                    chip("\(model.proposals.count) proposals", icon: "lightbulb.max")
                    chip("\(model.experiments.count) experiments", icon: "flask")
                    if let lastRefresh = model.lastRefresh {
                        chip(RelativeDateTimeFormatter().localizedString(for: lastRefresh, relativeTo: .now), icon: "clock")
                    }
                }

                Button {
                    Task { await model.refresh() }
                } label: {
                    Label("Refresh", systemImage: "arrow.clockwise")
                }
            }
        }
    }

    private func chip(_ text: String, icon: String) -> some View {
        Label(text, systemImage: icon)
            .font(.caption.weight(.medium))
            .padding(.horizontal, 10)
            .padding(.vertical, 6)
            .background(.quaternary, in: Capsule())
    }

    private func proposalCard(_ proposal: EvolutionProposal) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text(proposal.hypothesis)
                    .font(.headline)
                    .lineLimit(2)
                Spacer()
                statusBadge(proposal.statusLabel)
            }

            Text(proposal.targetMetric)
                .font(.subheadline)
                .foregroundStyle(.secondary)

            HStack {
                Label(String(format: "%.1f%% Δ", proposal.expectedDeltaPercent), systemImage: "chart.line.uptrend.xyaxis")
                Spacer()
                Text("#\(proposal.id)")
                    .font(.caption.monospaced())
                    .foregroundStyle(.secondary)
            }
            .font(.caption)
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            model.selectedProposalID == proposal.id ? Color.blue.opacity(0.12) : Color.secondary.opacity(0.08),
            in: RoundedRectangle(cornerRadius: 20, style: .continuous)
        )
    }

    private func statusBadge(_ text: String) -> some View {
        Text(text)
            .font(.caption.weight(.semibold))
            .padding(.horizontal, 10)
            .padding(.vertical, 6)
            .background(.quaternary, in: Capsule())
    }
}
