// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — HealthResponse Codable model

import Foundation

/// Response shape for GET /api/health.
struct HealthResponse: Codable {
    let ok: Bool
    let db: Bool
    let version: String
    let uptimeSecs: Int
    let peers: Int
    let agentActivity: Bool
    let tables: Int

    enum CodingKeys: String, CodingKey {
        case ok, db, version, peers, tables
        case uptimeSecs = "uptime_secs"
        case agentActivity = "agent_activity"
    }
}
