import SwiftUI

struct SessionMonitor: View {
    let runningAgents: [AgentRuntime]

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Session monitor")
                .font(.headline)

            ForEach(runningAgents.prefix(6)) { agent in
                HStack {
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
        .padding(20)
        .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 22, style: .continuous))
    }
}
