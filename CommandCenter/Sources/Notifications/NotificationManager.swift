import Foundation
import Observation
import UserNotifications

@MainActor
@Observable
final class NotificationManager: NSObject, UNUserNotificationCenterDelegate {
    static let shared = NotificationManager()

    private let client: DaemonClient
    private let dashboardSocket: WebSocketManager
    private let center = UNUserNotificationCenter.current()
    private var started = false

    var authorizationStatusText = "Not requested"
    var errorMessage: String?
    var deliveredCount = 0
    var thorResultEnabled = true { didSet { persistPreferences() } }
    var taskCompleteEnabled = true { didSet { persistPreferences() } }
    var meshEventEnabled = true { didSet { persistPreferences() } }
    var planStateEnabled = true { didSet { persistPreferences() } }

    override init() {
        client = DaemonClient()
        dashboardSocket = WebSocketManager(client: client)
        super.init()
        center.delegate = self
        loadPreferences()
    }

    func startIfNeeded() async {
        guard !started else { return }
        started = true
        registerCategories()
        await requestPermission()
        await loadUnreadNotifications()
        await subscribe()
    }

    func requestPermission() async {
        do {
            _ = try await center.requestAuthorization(options: [.alert, .badge, .sound])
            await refreshAuthorizationStatus()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func isEnabled(_ category: CommandCenterNotificationCategory) -> Bool {
        switch category {
        case .thorResult: return thorResultEnabled
        case .taskComplete: return taskCompleteEnabled
        case .meshEvent: return meshEventEnabled
        case .planState: return planStateEnabled
        }
    }

    private func refreshAuthorizationStatus() async {
        let settings = await center.notificationSettings()
        switch settings.authorizationStatus {
        case .authorized, .provisional, .ephemeral:
            authorizationStatusText = "Allowed"
        case .denied:
            authorizationStatusText = "Denied"
        case .notDetermined:
            authorizationStatusText = "Not requested"
        @unknown default:
            authorizationStatusText = "Unknown"
        }
    }

    private func subscribe() async {
        try? await dashboardSocket.connect(.dashboard) { [weak self] event in
            guard let self else { return }
            Task { @MainActor in
                await self.handle(event: event)
            }
        }
    }

    private func handle(event: WebSocketEvent) async {
        switch event {
        case .json(_, let envelope):
            await deliver(envelope: envelope)
        case .text(_, let text):
            await schedule(
                title: "Dashboard Event",
                subtitle: CommandCenterNotificationCategory.planState.title,
                body: text,
                category: .planState
            )
        case .binary, .disconnected:
            break
        }
    }

    private func loadUnreadNotifications() async {
        do {
            let notifications: [StoredDashboardNotification] = try await client.get("/api/notifications")
            for notification in notifications.prefix(6) {
                await deliver(stored: notification)
            }
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func deliver(stored: StoredDashboardNotification) async {
        let category = category(forType: stored.type, text: stored.title + " " + stored.message)
        await schedule(title: stored.title, subtitle: category.title, body: stored.message, category: category)
    }

    private func deliver(envelope: WebSocketEnvelope) async {
        let category = category(forType: envelope.type ?? envelope.eventType, text: envelope.message ?? "")
        let title: String
        let body: String

        switch category {
        case .thorResult:
            title = "Thor Result"
            body = envelope.message ?? "Thor produced an updated validation result."
        case .taskComplete:
            title = "Task Activity"
            body = envelope.runId.map { "Run #\($0) changed to \(envelope.status ?? "updated")." }
                ?? envelope.message
                ?? "A task-related dashboard event arrived."
        case .meshEvent:
            title = "Mesh Event"
            body = envelope.message ?? "A mesh peer or transport event was received."
        case .planState:
            title = "Plan Update"
            body = envelope.message ?? "A plan state transition was received from the daemon."
        }

        await schedule(title: title, subtitle: category.title, body: body, category: category)
    }

    private func schedule(
        title: String,
        subtitle: String,
        body: String,
        category: CommandCenterNotificationCategory
    ) async {
        guard isEnabled(category) else { return }

        let content = UNMutableNotificationContent()
        content.title = title
        content.subtitle = subtitle
        content.body = body
        content.sound = .default
        content.categoryIdentifier = category.rawValue

        let request = UNNotificationRequest(
            identifier: "command-center-\(UUID().uuidString)",
            content: content,
            trigger: nil
        )

        do {
            try await center.add(request)
            deliveredCount += 1
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func registerCategories() {
        let approve = UNNotificationAction(identifier: "thor-approve", title: "Approve")
        let reject = UNNotificationAction(identifier: "thor-reject", title: "Reject", options: [.destructive])
        let thorCategory = UNNotificationCategory(
            identifier: CommandCenterNotificationCategory.thorResult.rawValue,
            actions: [approve, reject],
            intentIdentifiers: []
        )
        let plainCategories = CommandCenterNotificationCategory.allCases
            .filter { $0 != .thorResult }
            .map { UNNotificationCategory(identifier: $0.rawValue, actions: [], intentIdentifiers: []) }
        center.setNotificationCategories(Set([thorCategory] + plainCategories))
    }

    private func category(forType type: String?, text: String) -> CommandCenterNotificationCategory {
        let lowered = "\(type ?? "") \(text)".lowercased()
        if lowered.contains("thor") || lowered.contains("reject") || lowered.contains("approve") {
            return .thorResult
        }
        if lowered.contains("mesh") || lowered.contains("peer") || lowered.contains("heartbeat") {
            return .meshEvent
        }
        if lowered.contains("plan") || lowered.contains("wave") {
            return .planState
        }
        return .taskComplete
    }

    private func persistPreferences() {
        let defaults = UserDefaults.standard
        defaults.set(thorResultEnabled, forKey: "notifications.thorResult")
        defaults.set(taskCompleteEnabled, forKey: "notifications.taskComplete")
        defaults.set(meshEventEnabled, forKey: "notifications.meshEvent")
        defaults.set(planStateEnabled, forKey: "notifications.planState")
    }

    private func loadPreferences() {
        let defaults = UserDefaults.standard
        thorResultEnabled = defaults.object(forKey: "notifications.thorResult") == nil
            ? true : defaults.bool(forKey: "notifications.thorResult")
        taskCompleteEnabled = defaults.object(forKey: "notifications.taskComplete") == nil
            ? true : defaults.bool(forKey: "notifications.taskComplete")
        meshEventEnabled = defaults.object(forKey: "notifications.meshEvent") == nil
            ? true : defaults.bool(forKey: "notifications.meshEvent")
        planStateEnabled = defaults.object(forKey: "notifications.planState") == nil
            ? true : defaults.bool(forKey: "notifications.planState")
    }

    nonisolated func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification,
        withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void
    ) {
        completionHandler([.banner, .sound])
    }

    nonisolated func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse,
        withCompletionHandler completionHandler: @escaping () -> Void
    ) {
        completionHandler()
    }
}
