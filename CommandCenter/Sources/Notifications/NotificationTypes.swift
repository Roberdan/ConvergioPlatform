import Foundation

enum CommandCenterNotificationCategory: String, CaseIterable, Identifiable {
    case thorResult = "thor-result"
    case taskComplete = "task-complete"
    case meshEvent = "mesh-event"
    case planState = "plan-state"

    var id: String { rawValue }

    var title: String {
        switch self {
        case .thorResult: return "Thor Results"
        case .taskComplete: return "Task Completion"
        case .meshEvent: return "Mesh Events"
        case .planState: return "Plan State"
        }
    }
}

struct StoredDashboardNotification: Codable, Identifiable {
    let id: Int
    let type: String?
    let title: String
    let message: String
    let link: String?
    let linkType: String?
    let isRead: Int?
    let createdAt: Date?
}
