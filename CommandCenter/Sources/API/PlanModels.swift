import Foundation

struct PlanListResponse: Codable, Sendable {
    let ok: Bool
    let plans: [PlanListItem]
}

struct PlanListItem: Codable, Identifiable, Sendable {
    let id: Int
    let name: String
    let status: String
    let tasksDone: Int?
    let tasksTotal: Int?
    let wavesMerged: Int?
    let wavesTotal: Int?
    let mergePct: Double?
    let executionHost: String?
    let createdAt: Date?
    let startedAt: Date?

    var mergeProgress: Double {
        if let wavesTotal, wavesTotal > 0 {
            return Double(wavesMerged ?? 0) / Double(wavesTotal)
        }
        if let mergePct {
            return min(max(mergePct / 100, 0), 1)
        }
        return 0
    }
}

struct PlanContextResponse: Codable, Sendable {
    let ok: Bool
    let plan: PlanContextPlan
    let waves: [PlanContextWave]
    let tasks: [PlanContextTask]
}

struct PlanContextPlan: Codable, Sendable {
    let id: Int
    let name: String
    let status: String
    let tasksDone: Int?
    let tasksTotal: Int?
    let worktreePath: String?
}

struct PlanContextWave: Codable, Identifiable, Sendable {
    let id: Int
    let waveId: String
    let name: String
    let status: String
    let position: Int?

    var waveLabel: String { waveId }
}

struct PlanContextTask: Codable, Identifiable, Sendable {
    let id: Int
    let taskId: String?
    let title: String
    let status: String
    let waveIdFk: Int?
    let waveId: String?
    let type: String?
    let priority: String?
    let executorAgent: String?
    let model: String?
    let effort: Int?
    let testCriteria: String?
    let verify: [String]?
    let notes: String?
    let outputData: JSONValue?
}

struct TaskStatusUpdateRequest: Encodable, Sendable {
    let taskId: Int
    let status: String
    let notes: String?
}

struct TaskStatusUpdateResponse: Codable, Sendable {
    let ok: Bool
    let taskId: Int
    let status: String
    let rowsChanged: Int?
}
