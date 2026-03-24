import Foundation

final class DaemonClient: Sendable {
    private let baseURL: URL
    private let session: URLSession
    private let tokenStore: KeychainTokenStore
    private let decoder = JSONDecoder.convergio
    private let encoder = JSONEncoder.convergio

    init(
        baseURL: URL = URL(string: "http://localhost:8420")!,
        session: URLSession = DaemonClient.makeSession(),
        tokenStore: KeychainTokenStore = .shared
    ) {
        self.baseURL = baseURL
        self.session = session
        self.tokenStore = tokenStore
    }

    func loadStoredToken() throws -> String? {
        try tokenStore.loadToken()
    }

    func storeToken(_ token: String) throws {
        try tokenStore.saveToken(token)
    }

    func health() async throws -> HealthResponse {
        try await get("/api/health", requiresAuth: false)
    }

    func agents() async throws -> DaemonAgentsResponse {
        try await get("/api/agents")
    }

    func peers() async throws -> PeersResponse {
        try await get("/api/peers")
    }

    func heartbeatStatus() async throws -> HeartbeatStatusResponse {
        try await get("/api/heartbeat/status")
    }

    func runs() async throws -> [RunSummary] {
        try await get("/api/runs")
    }

    func runMetrics(runID: Int) async throws -> RunMetricsResponse {
        try await get("/api/metrics/run/\(runID)")
    }

    func brainSnapshot() async throws -> BrainSnapshotResponse {
        try await get("/api/brain")
    }

    func get<Response: Decodable>(
        _ path: String,
        queryItems: [URLQueryItem] = [],
        requiresAuth: Bool = true
    ) async throws -> Response {
        try await perform(path: path, method: "GET", queryItems: queryItems, body: nil, requiresAuth: requiresAuth)
    }

    func post<Request: Encodable, Response: Decodable>(
        _ path: String,
        body: Request,
        requiresAuth: Bool = true
    ) async throws -> Response {
        try await perform(
            path: path,
            method: "POST",
            body: try encoder.encode(body),
            requiresAuth: requiresAuth
        )
    }

    func put<Request: Encodable, Response: Decodable>(
        _ path: String,
        body: Request,
        requiresAuth: Bool = true
    ) async throws -> Response {
        try await perform(
            path: path,
            method: "PUT",
            body: try encoder.encode(body),
            requiresAuth: requiresAuth
        )
    }

    func delete<Response: Decodable>(
        _ path: String,
        requiresAuth: Bool = true
    ) async throws -> Response {
        try await perform(path: path, method: "DELETE", body: nil, requiresAuth: requiresAuth)
    }

    func probeHealth() async -> ConnectionProbeResult {
        do {
            return .connected(try await health())
        } catch let error as URLError where error.code == .cannotConnectToHost || error.code == .timedOut {
            return .daemonOffline
        } catch {
            return .failed(error.localizedDescription)
        }
    }

    private func perform<Response: Decodable>(
        path: String,
        method: String,
        queryItems: [URLQueryItem] = [],
        body: Data?,
        requiresAuth: Bool
    ) async throws -> Response {
        var request = try makeRequest(path: path, method: method, queryItems: queryItems, requiresAuth: requiresAuth)
        request.httpBody = body
        if body != nil {
            request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        }

        let (data, response) = try await session.data(for: request)
        guard let httpResponse = response as? HTTPURLResponse else {
            throw DaemonClientError.invalidResponse
        }

        guard (200 ... 299).contains(httpResponse.statusCode) else {
            throw DaemonClientError.httpStatus(httpResponse.statusCode, payload: String(data: data, encoding: .utf8))
        }

        do {
            return try decoder.decode(Response.self, from: data)
        } catch {
            throw DaemonClientError.decodingFailure(error.localizedDescription)
        }
    }

    func makeRequest(
        path: String,
        method: String,
        queryItems: [URLQueryItem] = [],
        requiresAuth: Bool = true
    ) throws -> URLRequest {
        guard var components = URLComponents(url: baseURL.appendingPathComponent(path), resolvingAgainstBaseURL: false) else {
            throw DaemonClientError.invalidURL(path)
        }
        if !queryItems.isEmpty {
            components.queryItems = queryItems
        }
        guard let url = components.url else {
            throw DaemonClientError.invalidURL(path)
        }

        var request = URLRequest(url: url)
        request.httpMethod = method
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        if requiresAuth, let token = try tokenStore.loadToken(), !token.isEmpty {
            request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        } else if requiresAuth {
            throw DaemonClientError.missingAuthToken
        }
        return request
    }

    private static func makeSession() -> URLSession {
        let configuration = URLSessionConfiguration.default
        configuration.waitsForConnectivity = true
        configuration.timeoutIntervalForRequest = 30
        configuration.timeoutIntervalForResource = 60
        return URLSession(configuration: configuration)
    }
}

enum ConnectionProbeResult: Sendable {
    case connected(HealthResponse)
    case daemonOffline
    case failed(String)
}

enum DaemonClientError: LocalizedError {
    case invalidURL(String)
    case missingAuthToken
    case invalidResponse
    case httpStatus(Int, payload: String?)
    case decodingFailure(String)

    var errorDescription: String? {
        switch self {
        case .invalidURL(let path):
            return "Invalid daemon URL for path \(path)."
        case .missingAuthToken:
            return "No daemon auth token was found in Keychain."
        case .invalidResponse:
            return "Daemon response was not an HTTP response."
        case .httpStatus(let code, let payload):
            return "Daemon request failed with HTTP \(code): \(payload ?? "No body")."
        case .decodingFailure(let reason):
            return "Failed to decode daemon response: \(reason)"
        }
    }
}
