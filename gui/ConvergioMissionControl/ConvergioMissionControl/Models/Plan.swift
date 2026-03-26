// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Plan Codable model

import Foundation

/// A Convergio plan as returned by the daemon API.
struct Plan: Codable, Identifiable {
    let id: Int
    let name: String
    let status: String
    let tasksDone: Int
    let tasksTotal: Int
    let createdAt: String
    let startedAt: String?
    let completedAt: String?

    enum CodingKeys: String, CodingKey {
        case id, name, status
        case tasksDone = "tasks_done"
        case tasksTotal = "tasks_total"
        case createdAt = "created_at"
        case startedAt = "started_at"
        case completedAt = "completed_at"
    }
}
