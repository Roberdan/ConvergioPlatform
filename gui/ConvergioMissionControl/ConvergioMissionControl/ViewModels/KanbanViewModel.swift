// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Kanban board view model

import Foundation

/// Fetches tasks for a plan and groups them into kanban columns.
/// Auto-refreshes every 15 seconds.
@Observable
final class KanbanViewModel {
    private(set) var tasks: [DaemonTask] = []
    private(set) var isLoading = false
    private(set) var errorMessage: String?

    let planId: Int
    private let baseURL: URL
    private var refreshTask: Task<Void, Never>?

    init(planId: Int, baseURL: URL = URL(string: "http://localhost:8420")!) {
        self.planId = planId
        self.baseURL = baseURL
    }

    // MARK: - Lifecycle

    func start() {
        Task { await refresh() }
        scheduleAutoRefresh()
    }

    func stop() {
        refreshTask?.cancel()
        refreshTask = nil
    }

    // MARK: - Computed columns

    var pendingTasks: [DaemonTask] {
        tasks.filter { $0.kanbanColumn == .pending }
    }

    var inProgressTasks: [DaemonTask] {
        tasks.filter { $0.kanbanColumn == .inProgress }
    }

    var doneTasks: [DaemonTask] {
        tasks.filter { $0.kanbanColumn == .done }
    }

    // MARK: - Data

    @MainActor
    func refresh() async {
        isLoading = true
        errorMessage = nil
        defer { isLoading = false }
        do {
            tasks = try await fetchTasks()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    // MARK: - Private

    private func fetchTasks() async throws -> [DaemonTask] {
        let url = baseURL.appendingPathComponent("/api/plan-db/execution-tree/\(planId)")
        let (data, response) = try await URLSession.shared.data(from: url)
        guard let http = response as? HTTPURLResponse,
              (200...299).contains(http.statusCode) else {
            throw URLError(.badServerResponse)
        }
        let json = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        let tree = json?["tree"] as? [[String: Any]] ?? []
        // Flatten tasks from all waves and decode via JSONDecoder
        var tasks: [DaemonTask] = []
        let decoder = JSONDecoder()
        for wave in tree {
            let waveTasks = wave["tasks"] as? [[String: Any]] ?? []
            let waveId = wave["wave_id"] as? String
            for var t in waveTasks {
                t["plan_id"] = planId
                t["wave_id"] = waveId as Any
                if let taskData = try? JSONSerialization.data(withJSONObject: t),
                   let task = try? decoder.decode(DaemonTask.self, from: taskData) {
                    tasks.append(task)
                }
            }
        }
        return tasks
    }

    private func scheduleAutoRefresh() {
        refreshTask = Task {
            while !Task.isCancelled {
                try? await Task.sleep(nanoseconds: 15_000_000_000)
                guard !Task.isCancelled else { break }
                await refresh()
            }
        }
    }
}

// Tasks extracted manually from execution-tree JSON (not Codable).
