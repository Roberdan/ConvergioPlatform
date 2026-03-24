import Foundation

enum SidebarItem: String, CaseIterable, Hashable, Identifiable {
    case plans
    case agents
    case mesh
    case evolution
    case costs
    case terminal
    case brain

    var id: String { rawValue }

    var title: String { rawValue.capitalized }

    var icon: String {
        switch self {
        case .plans: return "list.bullet.rectangle"
        case .agents: return "person.3.sequence"
        case .mesh: return "point.3.connected.trianglepath.dotted"
        case .evolution: return "chart.line.uptrend.xyaxis"
        case .costs: return "dollarsign.gauge.chart.leftthird.topthird.rightthird"
        case .terminal: return "terminal"
        case .brain: return "brain"
        }
    }

    var summary: String {
        switch self {
        case .plans: return "Wave timelines, kanban tasks, and validation state."
        case .agents: return "Catalog, triage, sessions, and routing controls."
        case .mesh: return "Peers, topology, heartbeat, and delegation."
        case .evolution: return "Experiments, ROI, proposals, and audit history."
        case .costs: return "Usage, model spend, run history, and trend views."
        case .terminal: return "Ghostty-backed terminal sessions across the mesh."
        case .brain: return "Realtime graph of plans, tasks, and agent relationships."
        }
    }
}
