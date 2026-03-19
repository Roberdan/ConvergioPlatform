// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — WebSocket client for daemon real-time updates

import Foundation

/// WS client for daemon real-time updates with exponential-backoff reconnect (max 30 s).
final class DaemonConnection: NSObject, URLSessionWebSocketDelegate {
    private let state: DaemonState
    private let notifications: NotificationManager
    private let endpoint: URL
    private var session: URLSession?
    private var task: URLSessionWebSocketTask?
    private var retryDelay: TimeInterval = 1
    private var isIntentionalDisconnect = false
    private static let maxRetryDelay: TimeInterval = 30

    init(
        state: DaemonState,
        notifications: NotificationManager = .shared,
        endpoint: URL = URL(string: "ws://localhost:8420/ws/dashboard")!
    ) {
        self.state = state
        self.notifications = notifications
        self.endpoint = endpoint
        super.init()
        self.session = URLSession(
            configuration: .default,
            delegate: self,
            delegateQueue: .main
        )
    }

    func connect() {
        isIntentionalDisconnect = false
        let wsTask = session!.webSocketTask(with: endpoint)
        self.task = wsTask
        wsTask.resume()
        receiveNextMessage()
    }

    func disconnect() {
        isIntentionalDisconnect = true
        task?.cancel(with: .goingAway, reason: nil)
        task = nil
        DispatchQueue.main.async { self.state.isConnected = false }
    }

    func urlSession(
        _ session: URLSession,
        webSocketTask: URLSessionWebSocketTask,
        didOpenWithProtocol proto: String?
    ) {
        retryDelay = 1
        DispatchQueue.main.async { self.state.isConnected = true }
    }

    func urlSession(
        _ session: URLSession,
        webSocketTask: URLSessionWebSocketTask,
        didCloseWith code: URLSessionWebSocketTask.CloseCode,
        reason: Data?
    ) {
        DispatchQueue.main.async { self.state.isConnected = false }
        scheduleReconnect()
    }

    private func receiveNextMessage() {
        task?.receive { [weak self] result in
            guard let self else { return }
            switch result {
            case .success(let message):
                self.handleMessage(message)
                self.receiveNextMessage()
            case .failure:
                DispatchQueue.main.async { self.state.isConnected = false }
                self.scheduleReconnect()
            }
        }
    }

    private func handleMessage(_ message: URLSessionWebSocketTask.Message) {
        let data: Data
        switch message {
        case .string(let text):
            guard let encoded = text.data(using: .utf8) else { return }
            data = encoded
        case .data(let raw):
            data = raw
        @unknown default:
            return
        }

        guard let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            return
        }

        let eventType = json["type"] as? String ?? json["kind"] as? String ?? "unknown"
        DispatchQueue.main.async {
            self.state.lastMessageAt = Date()
            self.dispatch(eventType: eventType, json: json)
        }
    }

    private func dispatch(eventType: String, json: [String: Any]) {
        switch eventType {
        case "refresh":
            // Initial connection ack -- no action beyond marking connected
            break

        case "heartbeat_snapshot":
            applyHeartbeat(json)

        case "notification":
            applyNotification(json)

        case "agent_registered":
            applyAgentRegistered(json)

        case "agent_unregistered":
            applyAgentUnregistered(json)

        case "coordinator_event":
            applyCoordinatorEvent(json)

        case "brain_event":
            // Future: feed neural viz
            break

        default:
            break
        }
    }

    private func applyHeartbeat(_ json: [String: Any]) {
        guard let payload = json["payload"] as? [String: Any],
              let nodes = payload["nodes"] as? [String: [String: Any]] else { return }

        var updated: [MeshNode] = []
        for (nodeId, info) in nodes {
            let status = MeshNode.NodeStatus(
                rawValue: info["status"] as? String ?? "offline"
            ) ?? .offline
            let node = MeshNode(
                id: nodeId,
                name: info["name"] as? String ?? nodeId,
                status: status,
                cpu: info["cpu"] as? Double ?? 0,
                memory: info["memory"] as? Double ?? 0,
                lastSeen: Date()
            )
            updated.append(node)

            // Trigger critical notification when a previously-online node drops
            if status == .offline,
               let existing = state.meshNodes.first(where: { $0.id == nodeId }),
               existing.status == .online {
                notifications.notifyNodeOffline(nodeName: node.name)
                state.pushNotification(DaemonNotification(
                    level: .critical,
                    title: "Node Offline",
                    body: "\(node.name) is unreachable",
                    eventType: "node_offline"
                ))
            }
        }
        state.meshNodes = updated
        state.meshOnlineCount = updated.filter { $0.status == .online }.count
    }

    private func applyNotification(_ json: [String: Any]) {
        let message = json["message"] as? String ?? ""
        let severity = json["severity"] as? String ?? "info"

        // Detect specific notification patterns
        if message.lowercased().contains("plan") && message.lowercased().contains("complete") {
            let planName = json["plan_name"] as? String ?? "Plan"
            notifications.notifyPlanComplete(planName: planName)
            state.pushNotification(DaemonNotification(
                level: .info, title: "Plan Complete",
                body: "\(planName) finished", eventType: "plan_complete"
            ))
        } else if message.lowercased().contains("budget") {
            notifications.notifyBudgetAlert(message: message)
            state.pushNotification(DaemonNotification(
                level: .warning, title: "Budget Alert",
                body: message, eventType: "budget_alert"
            ))
        } else if message.lowercased().contains("blocked") {
            let agent = json["agent"] as? String ?? "Agent"
            notifications.notifyAgentBlocked(agentName: agent, reason: message)
            state.pushNotification(DaemonNotification(
                level: .warning, title: "Agent Blocked",
                body: message, eventType: "agent_blocked"
            ))
        } else if severity != "info" {
            state.pushNotification(DaemonNotification(
                level: severity == "critical" ? .critical : .warning,
                title: "Daemon Alert",
                body: message, eventType: "notification"
            ))
        }
    }

    private func applyAgentRegistered(_ json: [String: Any]) {
        let agentId = json["agent_id"] as? String ?? UUID().uuidString
        let name = json["name"] as? String ?? agentId
        let project = json["project"] as? String
        let agent = AgentInfo(
            id: agentId, name: name, status: "running", project: project
        )
        if !state.runningAgents.contains(where: { $0.id == agentId }) {
            state.runningAgents.append(agent)
        }
    }

    private func applyAgentUnregistered(_ json: [String: Any]) {
        let agentId = json["agent_id"] as? String ?? ""
        state.runningAgents.removeAll { $0.id == agentId }
    }

    private func applyCoordinatorEvent(_ json: [String: Any]) {
        guard let payload = json["payload"] as? [String: Any] else { return }
        let action = payload["action"] as? String ?? ""
        if action == "agent_blocked" {
            let agent = payload["agent"] as? String ?? "Agent"
            let reason = payload["reason"] as? String ?? "Unknown reason"
            notifications.notifyAgentBlocked(agentName: agent, reason: reason)
            state.pushNotification(DaemonNotification(
                level: .warning, title: "Agent Blocked",
                body: "\(agent): \(reason)", eventType: "agent_blocked"
            ))
        }
    }

    private func scheduleReconnect() {
        guard !isIntentionalDisconnect else { return }
        let delay = retryDelay
        retryDelay = min(retryDelay * 2, Self.maxRetryDelay)
        DispatchQueue.main.asyncAfter(deadline: .now() + delay) { [weak self] in
            self?.connect()
        }
    }
}
