// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Native macOS notification dispatch

import Foundation
import UserNotifications

/// Manages UNUserNotificationCenter authorization and dispatch.
/// Maps daemon WebSocket events to native macOS notifications with
/// appropriate urgency levels.
final class NotificationManager: NSObject, UNUserNotificationCenterDelegate {

    static let shared = NotificationManager()

    private override init() {
        super.init()
        UNUserNotificationCenter.current().delegate = self
    }

    // MARK: - Authorization

    /// Request notification permission on first launch.
    func requestAuthorization() {
        let center = UNUserNotificationCenter.current()
        center.requestAuthorization(options: [.alert, .sound, .badge]) { granted, error in
            if let error {
                print("[NotificationManager] auth error: \(error.localizedDescription)")
            }
            if !granted {
                print("[NotificationManager] notifications denied by user")
            }
        }
    }

    // MARK: - Dispatch

    /// Send a native notification for a daemon event.
    /// - Parameters:
    ///   - title: Notification title (e.g. "Node Offline").
    ///   - body: Descriptive message.
    ///   - category: Event category for grouping.
    ///   - level: Interruption level controlling urgency.
    func send(
        title: String,
        body: String,
        category: String,
        level: UNNotificationInterruptionLevel = .active
    ) {
        let content = UNMutableNotificationContent()
        content.title = title
        content.body = body
        content.sound = soundForLevel(level)
        content.categoryIdentifier = category
        content.interruptionLevel = level

        let request = UNNotificationRequest(
            identifier: UUID().uuidString,
            content: content,
            trigger: nil // deliver immediately
        )

        UNUserNotificationCenter.current().add(request) { error in
            if let error {
                print("[NotificationManager] delivery error: \(error.localizedDescription)")
            }
        }
    }

    // MARK: - Event Handlers

    /// Node went offline -- critical alert.
    func notifyNodeOffline(nodeName: String) {
        send(
            title: "Node Offline",
            body: "\(nodeName) is unreachable. Check mesh connectivity.",
            category: "node_offline",
            level: .critical
        )
    }

    /// Plan completed -- informational.
    func notifyPlanComplete(planName: String) {
        send(
            title: "Plan Complete",
            body: "\(planName) finished successfully.",
            category: "plan_complete",
            level: .active
        )
    }

    /// Agent blocked -- warning.
    func notifyAgentBlocked(agentName: String, reason: String) {
        send(
            title: "Agent Blocked",
            body: "\(agentName): \(reason)",
            category: "agent_blocked",
            level: .timeSensitive
        )
    }

    /// Budget threshold exceeded -- warning.
    func notifyBudgetAlert(message: String) {
        send(
            title: "Budget Alert",
            body: message,
            category: "budget_alert",
            level: .timeSensitive
        )
    }

    // MARK: - Delegate

    /// Show notifications even when the app is in the foreground.
    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification
    ) async -> UNNotificationPresentationOptions {
        [.banner, .sound]
    }

    // MARK: - Private

    private func soundForLevel(_ level: UNNotificationInterruptionLevel) -> UNNotificationSound {
        switch level {
        case .critical:
            return .defaultCritical
        case .timeSensitive:
            return .default
        default:
            return .default
        }
    }
}
