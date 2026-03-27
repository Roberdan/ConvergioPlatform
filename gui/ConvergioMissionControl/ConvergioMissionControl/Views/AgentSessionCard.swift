// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Individual agent session card

import SwiftUI

/// Card displaying a single daemon agent's identity, type, model, cost, and status.
struct AgentSessionCard: View {
    let agent: DaemonAgent
    @State private var isHovered: Bool = false

    var body: some View {
        HStack(spacing: 10) {
            statusDot
            VStack(alignment: .leading, spacing: 3) {
                HStack(spacing: 6) {
                    agentIdLabel
                    typeBadge
                    Spacer()
                    costLabel
                }
                HStack(spacing: 6) {
                    modelLabel
                    if let planId = agent.planId { planLink(planId) }
                    Spacer()
                    durationLabel
                }
            }
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 8)
        .background(
            isHovered ? Color.primary.opacity(0.06) : Color.clear,
            in: RoundedRectangle(cornerRadius: 8)
        )
        .onHover { isHovered = $0 }
        .accessibilityElement(children: .combine)
        .accessibilityLabel(accessibilityDescription)
    }

    // MARK: - Sub-views

    private var statusDot: some View {
        ZStack {
            if agent.status == "running" {
                Circle()
                    .fill(Color.green.opacity(0.3))
                    .frame(width: 14, height: 14)
                    // Pulsing outer ring for active agents
                    .overlay(
                        Circle()
                            .stroke(Color.green.opacity(0.5), lineWidth: 1)
                            .scaleEffect(isHovered ? 1.4 : 1.2)
                            .animation(
                                Animation.easeInOut(duration: 1.2).repeatForever(autoreverses: true),
                                value: isHovered
                            )
                    )
            }
            Circle()
                .fill(agent.status == "running" ? Color.green : Color.secondary.opacity(0.4))
                .frame(width: 8, height: 8)
        }
        .accessibilityHidden(true)
    }

    private var agentIdLabel: some View {
        Text(agent.agentId)
            .font(.system(.caption, design: .monospaced))
            .lineLimit(1)
            .truncationMode(.middle)
            .frame(maxWidth: 180, alignment: .leading)
    }

    private var typeBadge: some View {
        Text(agent.type)
            .font(.caption2.bold())
            .padding(.horizontal, 5)
            .padding(.vertical, 2)
            .background(typeBadgeColor.opacity(0.18), in: RoundedRectangle(cornerRadius: 4))
            .foregroundStyle(typeBadgeColor)
    }

    private var typeBadgeColor: Color {
        switch agent.type.lowercased() {
        case "claude": return .purple
        case "copilot": return .blue
        case "executor": return .orange
        default: return .secondary
        }
    }

    private var costLabel: some View {
        Text(formattedCost)
            .font(.caption2.monospacedDigit())
            .foregroundStyle(.secondary)
    }

    private var modelLabel: some View {
        Text(agent.model)
            .font(.caption2)
            .foregroundStyle(.tertiary)
            .lineLimit(1)
    }

    private func planLink(_ planId: Int) -> some View {
        Text("Plan #\(planId)")
            .font(.caption2)
            .foregroundStyle(Color.accentColor)
    }

    private var durationLabel: some View {
        Text(agent.durationDisplay)
            .font(.caption2.monospacedDigit())
            .foregroundStyle(.tertiary)
    }

    // MARK: - Computed

    private var formattedCost: String {
        if agent.costUsd == 0 { return "$0.00" }
        return String(format: "$%.4f", agent.costUsd)
    }

    private var accessibilityDescription: String {
        let statusWord = agent.status == "running" ? "Active" : "Completed"
        let planPart = agent.planId.map { ", linked to Plan \($0)" } ?? ""
        return "\(statusWord) agent \(agent.agentId), type \(agent.type), model \(agent.model)\(planPart), cost \(formattedCost), running for \(agent.durationDisplay)"
    }
}
