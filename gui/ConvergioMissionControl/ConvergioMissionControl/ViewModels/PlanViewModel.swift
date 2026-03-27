// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Plan hierarchy view model

import SwiftUI

// MARK: - Domain Models

/// A project grouping master and sub-plans.
struct ProjectItem: Identifiable {
    let id: String
    let name: String
    var plans: [PlanItem]
}

/// A plan with its child sub-plans (master plan = has children).
struct PlanItem: Identifiable, Hashable {
    let id: Int
    var name: String
    var status: PlanStatus
    var tasksDone: Int
    var tasksTotal: Int
    var subPlans: [PlanItem]

    var progress: Double {
        guard tasksTotal > 0 else { return 0 }
        return Double(tasksDone) / Double(tasksTotal)
    }
}

enum PlanStatus: String, Hashable {
    case todo, doing, done, cancelled

    var color: Color {
        switch self {
        case .todo: return .secondary
        case .doing: return .blue
        case .done: return .green
        case .cancelled: return .red
        }
    }

    var label: String { rawValue.capitalized }
}

// MARK: - View Model

/// Observable class that fetches plan hierarchy from the daemon REST API.
/// Auto-refreshes every 30 seconds. Errors surface via `errorMessage`.
@Observable
final class PlanViewModel {

    private(set) var projects: [ProjectItem] = []
    private(set) var isLoading: Bool = false
    private(set) var errorMessage: String?

    private let baseURL: URL
    private var refreshTask: Task<Void, Never>?

    init(baseURL: URL = URL(string: "http://localhost:8420")!) {
        self.baseURL = baseURL
    }

    // MARK: - Lifecycle

    func startRefreshing() {
        Task { await fetchPlans() }
        refreshTask = Task {
            while !Task.isCancelled {
                try? await Task.sleep(nanoseconds: 30_000_000_000)
                guard !Task.isCancelled else { break }
                await fetchPlans()
            }
        }
    }

    func stopRefreshing() {
        refreshTask?.cancel()
        refreshTask = nil
    }

    // MARK: - Fetch

    @MainActor
    func fetchPlans() async {
        isLoading = true
        errorMessage = nil
        defer { isLoading = false }

        do {
            let raw = try await loadPlans()
            projects = buildHierarchy(from: raw)
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    // MARK: - Private

    private func loadPlans() async throws -> [[String: Any]] {
        let url = baseURL.appendingPathComponent("/api/plan-db/list")
        let (data, response) = try await URLSession.shared.data(from: url)
        guard let http = response as? HTTPURLResponse,
              (200...299).contains(http.statusCode) else {
            throw URLError(.badServerResponse)
        }
        guard let json = try JSONSerialization.jsonObject(with: data) as? [String: Any],
              let plans = json["plans"] as? [[String: Any]] else {
            return []
        }
        return plans
    }

    /// Build a two-level hierarchy: projects -> master plans -> sub-plans.
    /// Groups all plans under the "convergio" project slug.
    private func buildHierarchy(from raw: [[String: Any]]) -> [ProjectItem] {
        let plans: [PlanItem] = raw.map { parsePlan($0) }
        guard !plans.isEmpty else { return [] }

        // Group by project slug; daemon does not yet expose project_id in /api/plans,
        // so we default to "convergio" and will refine when the API exposes it.
        let grouped = Dictionary(grouping: plans) { _ in "convergio" }

        return grouped.map { key, items in
            ProjectItem(
                id: key,
                name: key.capitalized,
                plans: items.sorted { $0.id < $1.id }
            )
        }.sorted { $0.name < $1.name }
    }

    private func parsePlan(_ dict: [String: Any]) -> PlanItem {
        let status = PlanStatus(rawValue: dict["status"] as? String ?? "todo") ?? .todo
        return PlanItem(
            id: dict["id"] as? Int ?? 0,
            name: dict["name"] as? String ?? "Unnamed Plan",
            status: status,
            tasksDone: dict["tasks_done"] as? Int ?? 0,
            tasksTotal: dict["tasks_total"] as? Int ?? 0,
            subPlans: []
        )
    }
}
