import Foundation

extension DaemonClient {
    func planList() async throws -> PlanListResponse {
        try await get("/api/plan-db/list")
    }

    func planContext(planID: Int) async throws -> PlanContextResponse {
        try await get("/api/plan-db/context/\(planID)")
    }

    func updateTaskStatus(
        taskID: Int,
        status: String,
        notes: String? = nil
    ) async throws -> TaskStatusUpdateResponse {
        try await post(
            "/api/plan-db/task/update",
            body: TaskStatusUpdateRequest(taskId: taskID, status: status, notes: notes)
        )
    }
}
