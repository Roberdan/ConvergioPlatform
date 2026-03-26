// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Task Codable model

import Foundation

/// A plan task as returned by /api/plan-db/execution-tree/:plan_id.
/// Named DaemonTask to avoid collision with Swift Concurrency's Task.
struct DaemonTask: Codable, Identifiable {
    let id: Int
    let taskId: String
    let title: String
    let status: String
    let priority: String
    let type: String
    let planId: Int?
    let waveId: String?
    let assignee: String?
    let startedAt: String?
    let completedAt: String?

    enum CodingKeys: String, CodingKey {
        case id
        case taskId = "task_id"
        case title, status, priority, type
        case planId = "plan_id"
        case waveId = "wave_id"
        case assignee
        case startedAt = "started_at"
        case completedAt = "completed_at"
    }
}
