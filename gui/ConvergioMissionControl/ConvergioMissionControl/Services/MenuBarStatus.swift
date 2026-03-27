// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Menu bar status indicator and notification bridge

import Foundation
import UserNotifications
import Observation

/// Tracks daemon health and active agent count for menu bar badge.
/// Posts macOS notifications for key events (agent started, plan done, errors).
@Observable
@MainActor
final class MenuBarStatus {
    private(set) var isHealthy = false
    private(set) var activeAgents = 0
    private(set) var statusLabel = "Checking..."

    private let baseURL: URL
    private var pollTask: Task<Void, Never>?

    init(baseURL: URL = URL(string: "http://localhost:8420")!) {
        self.baseURL = baseURL
    }

    func startPolling() {
        requestNotificationPermission()
        pollTask?.cancel()
        pollTask = Task { [weak self] in
            while !Task.isCancelled {
                await self?.refresh()
                try? await Task.sleep(for: .seconds(10))
            }
        }
    }

    func stopPolling() {
        pollTask?.cancel()
        pollTask = nil
    }

    // MARK: - Refresh

    func refresh() async {
        // Health check
        let healthURL = baseURL.appendingPathComponent("api/health")
        do {
            let (data, _) = try await URLSession.shared.data(from: healthURL)
            let json = try JSONSerialization.jsonObject(with: data) as? [String: Any]
            let ok = json?["ok"] as? Bool ?? false
            let peers = json?["peers"] as? Int ?? 0
            let wasUnhealthy = !isHealthy
            isHealthy = ok
            statusLabel = ok ? "\(peers) peers" : "Offline"
            if ok && wasUnhealthy {
                sendNotification(title: "Convergio", body: "Daemon connected (\(peers) peers)")
            }
        } catch {
            if isHealthy {
                sendNotification(title: "Convergio", body: "Daemon connection lost")
            }
            isHealthy = false
            statusLabel = "Offline"
        }

        // Agent count
        let agentsURL = baseURL.appendingPathComponent("api/agents")
        if let (data, _) = try? await URLSession.shared.data(from: agentsURL),
           let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
           let running = json["running"] as? [[String: Any]] {
            let prev = activeAgents
            activeAgents = running.count
            if running.count > prev && prev > 0 {
                sendNotification(
                    title: "Agent Started",
                    body: "\(running.count) active agents"
                )
            }
        }
    }

    // MARK: - Notifications

    private func requestNotificationPermission() {
        UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound]) { _, _ in }
    }

    private func sendNotification(title: String, body: String) {
        let content = UNMutableNotificationContent()
        content.title = title
        content.body = body
        content.sound = .default
        let request = UNNotificationRequest(
            identifier: UUID().uuidString,
            content: content,
            trigger: nil
        )
        UNUserNotificationCenter.current().add(request)
    }

    /// Menu bar icon name based on current state.
    var menuBarIcon: String {
        if !isHealthy { return "brain.head.profile" }
        if activeAgents > 0 { return "brain.head.profile.fill" }
        return "brain.head.profile"
    }
}
