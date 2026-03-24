import Foundation
import Observation

@MainActor
@Observable
final class PlanDashboardModel {
    private let client: DaemonClient
    private let dashboardSocket: WebSocketManager
    private var subscribed = false

    var plans: [PlanListItem] = []
    var selectedPlanID: Int?
    var selectedContext: PlanContextResponse?
    var isLoading = false
    var errorMessage: String?

    init(client: DaemonClient = DaemonClient()) {
        self.client = client
        dashboardSocket = WebSocketManager(client: client)
    }

    func load() async {
        await subscribeIfNeeded()
        await refresh()
    }

    func selectPlan(_ id: Int) async {
        selectedPlanID = id
        await loadSelectedPlan()
    }

    func chartData() -> [WaveProgressDatum] {
        guard let selectedContext else { return [] }
        return selectedContext.waves.map { wave in
            let tasks = selectedContext.tasks.filter { $0.waveIdFk == wave.id }
            let done = tasks.filter { $0.status == "done" }.count
            return WaveProgressDatum(id: wave.id, wave: wave.waveLabel, done: done, total: max(tasks.count, 1))
        }
    }

    func tasks(for status: String) -> [PlanContextTask] {
        selectedContext?.tasks.filter { $0.status == status } ?? []
    }

    func moveTask(_ taskID: Int, to status: String) async {
        do {
            _ = try await client.updateTaskStatus(taskID: taskID, status: status)
            await loadSelectedPlan()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func refresh() async {
        isLoading = true
        defer { isLoading = false }

        do {
            let response = try await client.planList()
            plans = response.plans.sorted { $0.id > $1.id }
            if selectedPlanID == nil {
                selectedPlanID = plans.first?.id
            }
            await loadSelectedPlan()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func loadSelectedPlan() async {
        guard let selectedPlanID else { return }
        do {
            selectedContext = try await client.planContext(planID: selectedPlanID)
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func subscribeIfNeeded() async {
        guard !subscribed else { return }
        subscribed = true

        try? await dashboardSocket.connect(.dashboard) { [weak self] event in
            guard let self else { return }
            switch event {
            case .json, .text:
                Task { @MainActor in
                    await self.refresh()
                }
            case .binary, .disconnected:
                break
            }
        }
    }
}

struct WaveProgressDatum: Identifiable {
    let id: Int
    let wave: String
    let done: Int
    let total: Int

    var completion: Double {
        Double(done) / Double(total)
    }
}
