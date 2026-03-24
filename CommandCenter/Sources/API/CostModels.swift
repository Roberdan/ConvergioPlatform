import Foundation

struct CostSummaryResponse: Codable, Sendable {
    let ok: Bool
    let runCount: Int
    let avgDurationSecs: Double?
    let totalCostUsd: Double
    let statusDistribution: [RunStatusBucket]
    let topAgents: [TopAgentUsage]
}

struct RunStatusBucket: Codable, Identifiable, Sendable {
    let status: String?
    let count: Int

    var id: String { status ?? "unknown" }
    var label: String { (status ?? "unknown").replacingOccurrences(of: "_", with: " ").capitalized }
}

struct TopAgentUsage: Codable, Identifiable, Sendable {
    let executorAgent: String?
    let taskCount: Int

    var id: String { executorAgent ?? "unassigned" }
    var label: String { executorAgent ?? "Unassigned" }
}

struct CostBreakdownResponse: Codable, Sendable {
    let ok: Bool
    let days: Int
    let projectFilter: String?
    let byModel: [CostModelBucket]
    let byProject: [CostProjectBucket]
    let byDate: [CostByDatePoint]
}

struct CostModelBucket: Codable, Identifiable, Sendable {
    let model: String?
    let calls: Int
    let costUsd: Double

    var id: String { model ?? "unknown-model" }
    var label: String { model ?? "Unknown" }
}

struct CostProjectBucket: Codable, Identifiable, Sendable {
    let projectId: String?
    let calls: Int
    let costUsd: Double

    var id: String { projectId ?? "unknown-project" }
    var label: String { projectId ?? "Unscoped" }
}

struct CostByDatePoint: Codable, Identifiable, Sendable {
    let date: String
    let costUsd: Double

    var id: String { date }

    var parsedDate: Date? {
        let formatter = DateFormatter()
        formatter.calendar = Calendar(identifier: .gregorian)
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter.date(from: date)
    }
}

struct RunDetail: Codable, Identifiable, Sendable {
    let id: Int
    let goal: String
    let team: String?
    let status: String?
    let result: String?
    let costUsd: Double?
    let agentsUsed: Int?
    let planId: Int?
    let planName: String?
    let startedAt: Date?
    let completedAt: Date?
    let durationMinutes: Double?
    let contextPath: String?
    let pausedAt: Date?
    let delegationCost: Double?
}
