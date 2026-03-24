import SwiftUI

struct SessionMonitor: View {
    let runningAgents: [AgentRuntime]

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Session monitor")
                .font(.headline)

            ForEach(runningAgents.prefix(6)) { agent in
                HStack {
                    // LED dot: verde racing when running, rosso corsa when stopped
                    Circle()
                        .fill(ledColor(for: agent))
                        .frame(width: 8, height: 8)

                    VStack(alignment: .leading, spacing: 2) {
                        Text(agent.agentId)
                            .font(.caption.monospaced())
                        Text(agent.model ?? agent.type)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                    Spacer()
                    VStack(alignment: .trailing, spacing: 2) {
                        Text("\(agent.tokensTotal ?? 0) tokens")
                            .font(.caption.monospacedDigit())
                        Text(String(format: "%.1fs", agent.durationS ?? 0))
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
                .padding(.vertical, 4)
            }
        }
        .modifier(ConvergioCardModifier())
    }

    // MARK: - Private

    /// Green LED when the agent has an active duration (running), red when idle/stopped.
    private func ledColor(for agent: AgentRuntime) -> Color {
        // An agent in the runningAgents list is by definition active —
        // use rossoCorsa only if durationS is nil (stale entry with no timing data).
        agent.durationS != nil
            ? ConvergioTokens.Status.success
            : ConvergioTokens.Brand.rossoCorsa
    }
}
