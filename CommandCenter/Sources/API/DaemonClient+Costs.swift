import Foundation

private struct EmptyRequestBody: Encodable {}

extension DaemonClient {
    func metricsSummary() async throws -> CostSummaryResponse {
        try await get("/api/metrics/summary")
    }

    func costBreakdown(days: Int, project: String? = nil) async throws -> CostBreakdownResponse {
        var queryItems = [URLQueryItem(name: "days", value: String(max(1, min(days, 365))))]
        if let project, !project.isEmpty {
            queryItems.append(URLQueryItem(name: "project", value: project))
        }
        return try await get("/api/metrics/cost", queryItems: queryItems)
    }

    func runHistory(status: String? = nil, limit: Int = 40) async throws -> [RunSummary] {
        var queryItems = [URLQueryItem(name: "limit", value: String(max(1, min(limit, 100))))]
        if let status, !status.isEmpty {
            queryItems.append(URLQueryItem(name: "status", value: status))
        }
        return try await get("/api/runs", queryItems: queryItems)
    }

    func runDetail(runID: Int) async throws -> RunDetail {
        try await get("/api/runs/\(runID)")
    }

    func pauseRun(runID: Int) async throws -> RunDetail {
        try await post("/api/runs/\(runID)/pause", body: EmptyRequestBody())
    }

    func resumeRun(runID: Int) async throws -> RunDetail {
        try await post("/api/runs/\(runID)/resume", body: EmptyRequestBody())
    }
}
