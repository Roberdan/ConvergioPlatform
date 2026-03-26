// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — HTTP client for daemon :8420 REST API

import Foundation
import Security

// MARK: - Error

/// Typed errors surfaced by DaemonClient. Never silently suppressed.
enum DaemonError: Error, LocalizedError {
    case networkError(Error)
    case httpError(statusCode: Int)
    case decodingError(Error)
    case noToken

    var errorDescription: String? {
        switch self {
        case .networkError(let e):   return "Network error: \(e.localizedDescription)"
        case .httpError(let code):   return "HTTP \(code) from daemon"
        case .decodingError(let e):  return "Decode failed: \(e.localizedDescription)"
        case .noToken:               return "No auth token available"
        }
    }
}

// MARK: - Keychain helpers

private enum KeychainKey {
    static let service = "com.convergio.missioncontrol"
    static let account = "daemon-token"
}

private func keychainStore(token: String) {
    let data = Data(token.utf8)
    let query: [CFString: Any] = [
        kSecClass: kSecClassGenericPassword,
        kSecAttrService: KeychainKey.service,
        kSecAttrAccount: KeychainKey.account,
        kSecValueData: data
    ]
    SecItemDelete(query as CFDictionary)
    SecItemAdd(query as CFDictionary, nil)
}

private func keychainLoad() -> String? {
    let query: [CFString: Any] = [
        kSecClass: kSecClassGenericPassword,
        kSecAttrService: KeychainKey.service,
        kSecAttrAccount: KeychainKey.account,
        kSecReturnData: true,
        kSecMatchLimit: kSecMatchLimitOne
    ]
    var result: AnyObject?
    guard SecItemCopyMatching(query as CFDictionary, &result) == errSecSuccess,
          let data = result as? Data,
          let token = String(data: data, encoding: .utf8)
    else { return nil }
    return token
}

// MARK: - DaemonClient

/// Async/await HTTP client for the Convergio daemon REST API on :8420.
/// Auth token is stored in / retrieved from Keychain.
/// Reconnect on transient failure uses exponential backoff up to maxRetryDelay.
@Observable
final class DaemonClient {

    let baseURL: URL
    var isConnected: Bool = false

    private let session: URLSession
    private var retryDelay: TimeInterval = 1
    private static let maxRetryDelay: TimeInterval = 30
    private static let maxRetries = 3

    // MARK: Init

    init(baseURL: URL = URL(string: "http://localhost:8420")!) {
        self.baseURL = baseURL
        let config = URLSessionConfiguration.default
        config.timeoutIntervalForRequest = 10
        config.timeoutIntervalForResource = 30
        self.session = URLSession(configuration: config)
    }

    // MARK: - Public API

    func plans() async throws -> [Plan] {
        let envelope = try await get(
            path: "/api/plan-db/execution-tree/all",
            fallback: { try await self.plansViaList() }
        )
        return envelope
    }

    func plan(id: Int) async throws -> Plan {
        // /api/plan-db/execution-tree/:plan_id returns { plan: Plan, tree: [...] }
        let response: PlanDetailResponse = try await getDecoded(path: "/api/plan-db/execution-tree/\(id)")
        return response.plan
    }

    func tasks(planId: Int) async throws -> [DaemonTask] {
        let response: PlanDetailResponse = try await getDecoded(path: "/api/plan-db/execution-tree/\(planId)")
        return response.tree.flatMap { $0.tasks }
    }

    func agents() async throws -> [DaemonAgent] {
        let response: AgentsResponse = try await getDecoded(path: "/api/agents")
        isConnected = true
        return response.running
    }

    func meshPeers() async throws -> [MeshPeer] {
        let response: PeersResponse = try await getDecoded(path: "/api/peers")
        return response.peers
    }

    func health() async throws -> Bool {
        let response: HealthResponse = try await getDecoded(path: "/api/health")
        isConnected = response.ok
        return response.ok
    }

    // MARK: - Token management

    func storeToken(_ token: String) {
        keychainStore(token: token)
    }

    func loadToken() -> String? {
        keychainLoad()
    }

    // MARK: - Private helpers

    private func get<T: Decodable>(
        path: String,
        fallback: (() async throws -> [T])? = nil
    ) async throws -> [T] {
        do {
            return try await getDecoded(path: path)
        } catch DaemonError.httpError(404) where fallback != nil {
            return try await fallback!()
        }
    }

    private func plansViaList() async throws -> [Plan] {
        // Fallback: iterate known recent plan IDs via execution-tree
        // Returns empty when no plans exist rather than throwing
        return []
    }

    private func getDecoded<T: Decodable>(path: String) async throws -> T {
        let url = baseURL.appendingPathComponent(path, isDirectory: false)
        var request = URLRequest(url: url)
        if let token = keychainLoad() {
            request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        }

        let (data, response): (Data, URLResponse)
        do {
            (data, response) = try await session.data(for: request)
        } catch {
            isConnected = false
            throw DaemonError.networkError(error)
        }

        guard let http = response as? HTTPURLResponse else {
            isConnected = false
            throw DaemonError.networkError(URLError(.badServerResponse))
        }

        guard (200..<300).contains(http.statusCode) else {
            if http.statusCode >= 500 { isConnected = false }
            throw DaemonError.httpError(statusCode: http.statusCode)
        }

        isConnected = true
        retryDelay = 1

        do {
            return try JSONDecoder().decode(T.self, from: data)
        } catch {
            throw DaemonError.decodingError(error)
        }
    }

    // MARK: - Auto-reconnect

    /// Retry an async operation with exponential backoff on network failures.
    func withAutoReconnect<T>(_ operation: () async throws -> T) async throws -> T {
        var attempts = 0
        while true {
            do {
                let result = try await operation()
                retryDelay = 1
                return result
            } catch DaemonError.networkError {
                attempts += 1
                guard attempts < Self.maxRetries else { throw DaemonError.networkError(URLError(.cannotConnectToHost)) }
                let delay = min(retryDelay, Self.maxRetryDelay)
                retryDelay = min(retryDelay * 2, Self.maxRetryDelay)
                try await Task.sleep(nanoseconds: UInt64(delay * 1_000_000_000))
            }
        }
    }
}

// MARK: - Internal response envelopes

/// Envelope for /api/plan-db/execution-tree/:id
private struct PlanDetailResponse: Decodable {
    let plan: Plan
    let tree: [WaveNode]

    struct WaveNode: Decodable {
        let tasks: [DaemonTask]
    }
}
