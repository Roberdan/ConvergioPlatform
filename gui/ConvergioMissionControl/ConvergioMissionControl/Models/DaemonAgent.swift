// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Daemon agent model decoded from GET /api/agents

import Foundation

/// A single agent entry from the daemon REST API.
/// Maps the JSON shape returned by `GET /api/agents`.
struct DaemonAgent: Identifiable, Equatable, Decodable {
    let agentId: String
    let type: String
    let model: String
    let planId: Int?
    let costUsd: Double
    let durationS: Double
    let startedAt: String
    /// Derived from position in response: "running" or "complete"
    var status: String

    var id: String { agentId }

    /// Human-readable duration string (e.g. "5m 30s")
    var durationDisplay: String {
        let total = Int(durationS)
        let hours = total / 3600
        let minutes = (total % 3600) / 60
        let seconds = total % 60
        if hours > 0 { return "\(hours)h \(minutes)m" }
        if minutes > 0 { return "\(minutes)m \(seconds)s" }
        return "\(seconds)s"
    }

    enum CodingKeys: String, CodingKey {
        case agentId = "agent_id"
        case type, model
        case planId = "plan_id"
        case costUsd = "cost_usd"
        case durationS = "duration_s"
        case startedAt = "started_at"
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        agentId = try c.decode(String.self, forKey: .agentId)
        type = try c.decodeIfPresent(String.self, forKey: .type) ?? "unknown"
        model = try c.decodeIfPresent(String.self, forKey: .model) ?? "unknown"
        planId = try c.decodeIfPresent(Int.self, forKey: .planId)
        costUsd = try c.decodeIfPresent(Double.self, forKey: .costUsd) ?? 0.0
        durationS = try c.decodeIfPresent(Double.self, forKey: .durationS) ?? 0.0
        startedAt = try c.decodeIfPresent(String.self, forKey: .startedAt) ?? ""
        status = "running"
    }

    /// Direct initializer for tests and previews
    init(
        agentId: String,
        type: String = "claude",
        model: String = "sonnet-4.6",
        planId: Int? = nil,
        costUsd: Double = 0.0,
        durationS: Double = 0.0,
        startedAt: String = "",
        status: String = "running"
    ) {
        self.agentId = agentId
        self.type = type
        self.model = model
        self.planId = planId
        self.costUsd = costUsd
        self.durationS = durationS
        self.startedAt = startedAt
        self.status = status
    }
}

/// Top-level response from `GET /api/agents`
struct DaemonAgentResponse: Decodable {
    let running: [DaemonAgent]
    let recent: [DaemonAgent]

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        var runningAgents = try c.decodeIfPresent([DaemonAgent].self, forKey: .running) ?? []
        var recentAgents = try c.decodeIfPresent([DaemonAgent].self, forKey: .recent) ?? []
        for i in runningAgents.indices { runningAgents[i].status = "running" }
        for i in recentAgents.indices { recentAgents[i].status = "complete" }
        self.running = runningAgents
        self.recent = recentAgents
    }

    enum CodingKeys: String, CodingKey {
        case running, recent
    }
}
