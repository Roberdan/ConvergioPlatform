// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Daemon state model

import Foundation

/// Observable state populated by the WebSocket connection to the Convergio daemon.
/// Views bind to @Published properties for real-time updates.
final class DaemonState: ObservableObject {

    // MARK: - Connection

    @Published var isConnected: Bool = false
    @Published var lastMessageAt: Date?

    // MARK: - Mesh

    @Published var meshNodes: [MeshNode] = []
    @Published var meshOnlineCount: Int = 0

    // MARK: - Plans

    @Published var activePlans: [PlanSummary] = []

    // MARK: - Agents

    @Published var runningAgents: [AgentInfo] = []

    // MARK: - Notifications

    /// Recent notification events for in-app display (newest first, capped at 50).
    @Published var notifications: [DaemonNotification] = []

    /// Append a notification, keeping the queue bounded.
    func pushNotification(_ notification: DaemonNotification) {
        notifications.insert(notification, at: 0)
        if notifications.count > 50 {
            notifications.removeLast()
        }
    }
}

// MARK: - Supporting Types

struct MeshNode: Identifiable, Equatable {
    let id: String
    var name: String
    var status: NodeStatus
    var cpu: Double
    var memory: Double
    var lastSeen: Date

    enum NodeStatus: String, Codable {
        case online, offline, degraded
    }
}

struct PlanSummary: Identifiable, Equatable {
    let id: Int
    var name: String
    var status: String
    var tasksDone: Int
    var tasksTotal: Int
}

struct AgentInfo: Identifiable, Equatable {
    let id: String
    var name: String
    var status: String
    var project: String?
}

struct DaemonNotification: Identifiable {
    let id: UUID
    let timestamp: Date
    let level: Level
    let title: String
    let body: String
    let eventType: String

    enum Level: String {
        case info, warning, critical
    }

    init(level: Level, title: String, body: String, eventType: String) {
        self.id = UUID()
        self.timestamp = Date()
        self.level = level
        self.title = title
        self.body = body
        self.eventType = eventType
    }
}
