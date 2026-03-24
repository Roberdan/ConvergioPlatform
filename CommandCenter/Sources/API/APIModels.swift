import Foundation

struct HealthResponse: Codable, Sendable {
    let ok: Bool
    let db: Bool
    let tables: Int
    let agentActivity: Bool
    let peers: Int
    let uptimeSecs: Int
    let version: String
}

struct DaemonAgentsResponse: Codable, Sendable {
    let running: [AgentRuntime]
    let recent: [AgentRuntime]
    let stats: AgentStats
}

struct AgentRuntime: Codable, Identifiable, Sendable {
    let agentId: String
    let type: String
    let model: String?
    let description: String?
    let taskDbId: Int?
    let planId: Int?
    let host: String?
    let region: String?
    let parentSession: String?
    let tokensIn: Int?
    let tokensOut: Int?
    let tokensTotal: Int?
    let costUsd: Double?
    let startedAt: Date?
    let durationS: Double?

    var id: String { agentId }
}

struct AgentStats: Codable, Sendable {
    let activeCount: Int
    let byModel: [ModelTokenUsage]
}

struct ModelTokenUsage: Codable, Identifiable, Sendable {
    let model: String
    let tokens: Int
    let count: Int?

    var id: String { model }
}

struct PeersResponse: Codable, Sendable {
    let peers: [PeerSummary]
}

struct PeerSummary: Codable, Identifiable, Sendable {
    let peerName: String
    let lastSeen: Double?
    let loadJson: JSONValue?
    let capabilities: String?
    let isOnline: Bool
    let isLocal: Bool
    let role: String?

    var id: String { peerName }
}

struct HeartbeatStatusResponse: Codable, Sendable {
    let ok: Bool
    let peers: [HeartbeatPeer]
}

struct HeartbeatPeer: Codable, Identifiable, Sendable {
    let peerName: String
    let status: String
    let ageSecs: Int?
    let lastSeen: Double?

    var id: String { peerName }
}

struct RunSummary: Codable, Identifiable, Sendable {
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
}

struct RunMetricsResponse: Codable, Sendable {
    let ok: Bool
    let run: RunSummary
    let durationSecs: Int?
    let costUsd: Double?
    let agentsUsed: Int?
    let tasksDone: Int?
    let tasksTotal: Int?
}

struct BrainSnapshotResponse: Codable, Sendable {
    let sessions: [BrainSession]
    let agents: [AgentRuntime]
    let recent: [BrainRecentAgent]
    let plans: [PlanSummary]
    let tasks: [BrainTask]
    let commits: [CommitSummary]
    let tokenSummary: [ModelTokenUsage]
}

struct BrainSession: Codable, Identifiable, Sendable {
    let agentId: String
    let type: String
    let description: String?
    let status: String?
    let metadata: JSONValue?
    let startedAt: Date?
    let tokensIn: Int?
    let tokensOut: Int?
    let tokensTotal: Int?
    let costUsd: Double?
    let model: String?
    let durationS: Double?

    var id: String { agentId }
}

struct BrainRecentAgent: Codable, Identifiable, Sendable {
    let agentId: String
    let type: String
    let model: String?
    let description: String?
    let parentSession: String?
    let status: String?
    let tokensTotal: Int?
    let costUsd: Double?
    let durationS: Double?
    let completedAt: Date?

    var id: String { agentId }
}

struct PlanSummary: Codable, Identifiable, Sendable {
    let id: Int
    let name: String
    let status: String
    let executionHost: String?
    let tasksDone: Int?
    let tasksTotal: Int?
    let progressPct: Double?
}

struct BrainTask: Codable, Identifiable, Sendable {
    let id: Int
    let title: String
    let status: String
    let taskId: String?
    let assignee: String?
    let priority: String?
    let taskType: String?
    let tokens: Int?
    let startedAt: Date?
    let executorSessionId: String?
    let executorHost: String?
    let model: String?
    let waveId: String?
    let outputData: JSONValue?
    let planName: String?
    let planId: Int?
    let waveName: String?
}

struct CommitSummary: Codable, Identifiable, Sendable {
    let commitSha: String
    let commitMessage: String?
    let linesAdded: Int?
    let linesRemoved: Int?
    let filesChanged: Int?
    let planId: Int?
    let authoredAt: Date?

    var id: String { commitSha }
}
