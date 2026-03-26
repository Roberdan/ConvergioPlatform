// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — DaemonAgent Codable model

import Foundation

/// A running agent session as returned by /api/agents.
/// Named DaemonAgent to avoid collision with the generic concept of "agent".
struct DaemonAgent: Codable, Identifiable {
    let agentId: String
    let type: String
    let host: String?
    let model: String?
    let planId: Int?
    let taskDbId: Int?
    let startedAt: String?
    let costUsd: Double?
    let tokensTotal: Int?

    /// Identifiable conformance — agentId is the stable identity.
    var id: String { agentId }

    enum CodingKeys: String, CodingKey {
        case agentId = "agent_id"
        case type, host, model
        case planId = "plan_id"
        case taskDbId = "task_db_id"
        case startedAt = "started_at"
        case costUsd = "cost_usd"
        case tokensTotal = "tokens_total"
    }
}

/// Wrapper for the /api/agents response envelope.
struct AgentsResponse: Codable {
    let running: [DaemonAgent]
    let recent: [DaemonAgent]
}
