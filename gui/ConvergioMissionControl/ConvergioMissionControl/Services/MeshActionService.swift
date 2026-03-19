// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Mesh REST API client for daemon operations

import Foundation

/// REST client for Convergio daemon mesh operations.
/// All methods use async/await with structured concurrency and
/// propagate errors via `MeshActionError`.
final class MeshActionService {

    static let shared = MeshActionService()

    private let baseURL: URL
    private let session: URLSession

    init(
        baseURL: URL = URL(string: "http://localhost:8420")!,
        session: URLSession = .shared
    ) {
        self.baseURL = baseURL
        self.session = session
    }

    // MARK: - Mesh Sync

    /// Trigger full mesh sync across all peers.
    /// POST /api/mesh/action?action=sync-all
    func syncAll() async throws -> MeshActionResult {
        let url = baseURL.appendingPathComponent("/api/mesh/action")
        var components = URLComponents(url: url, resolvingAgainstBaseURL: false)!
        components.queryItems = [URLQueryItem(name: "action", value: "sync-all")]
        var request = URLRequest(url: components.url!)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONSerialization.data(withJSONObject: [:])
        let json = try await performRequest(request)
        return MeshActionResult(
            ok: json["ok"] as? Bool ?? false,
            message: json["message"] as? String
        )
    }

    // MARK: - Heartbeat

    /// Fetch peer heartbeat statuses.
    /// GET /api/heartbeat/status
    func heartbeatCheck() async throws -> [PeerHeartbeat] {
        let url = baseURL.appendingPathComponent("/api/heartbeat/status")
        let request = URLRequest(url: url)
        let json = try await performRequest(request)

        guard let peers = json["peers"] as? [[String: Any]] else {
            throw MeshActionError.unexpectedResponse("missing peers array")
        }
        return peers.map { peer in
            PeerHeartbeat(
                peerName: peer["peer_name"] as? String ?? "unknown",
                status: peer["status"] as? String ?? "offline",
                ageSecs: peer["age_secs"] as? Int ?? -1,
                lastSeen: peer["last_seen"] as? Double ?? 0
            )
        }
    }

    // MARK: - Delegate Task

    /// Delegate a plan's execution to a specific peer.
    /// POST /api/mesh/delegate  Body: {plan_id, peer}
    func delegateTask(
        peer: String,
        planId: Int,
        taskId: String? = nil
    ) async throws -> MeshActionResult {
        let url = baseURL.appendingPathComponent("/api/mesh/delegate")
        var body: [String: Any] = ["peer": peer, "plan_id": planId]
        if let taskId { body["task_id"] = taskId }

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONSerialization.data(withJSONObject: body)

        let json = try await performRequest(request)
        let delegatedTo = json["delegated_to"] as? String ?? peer
        return MeshActionResult(
            ok: json["ok"] as? Bool ?? false,
            message: "Delegated plan \(planId) to \(delegatedTo)"
        )
    }

    // MARK: - Provision

    /// Provision all mesh nodes (SSH check, tmux, session).
    /// GET /api/mesh/provision
    func provisionNode(peer: String? = nil) async throws -> [ProvisionResult] {
        let url = baseURL.appendingPathComponent("/api/mesh/provision")
        let request = URLRequest(url: url)
        let json = try await performRequest(request)

        guard let peers = json["peers"] as? [[String: Any]] else {
            throw MeshActionError.unexpectedResponse("missing peers array")
        }
        let results = peers.map { p in
            ProvisionResult(
                peer: p["peer"] as? String ?? "unknown",
                ip: p["ip"] as? String,
                online: p["online"] as? Bool ?? false,
                sshOk: p["ssh_ok"] as? Bool ?? false,
                tmuxOk: p["tmux_ok"] as? Bool ?? false,
                sessionOk: p["session_ok"] as? Bool ?? false,
                error: p["error"] as? String
            )
        }
        // When a specific peer is requested, filter to that peer only
        if let peer {
            return results.filter { $0.peer == peer }
        }
        return results
    }

    // MARK: - Add / Remove Peer

    /// Register a new peer.
    /// POST /api/peers  Body: {peer_name}
    func addNode(peerName: String) async throws -> MeshActionResult {
        let url = baseURL.appendingPathComponent("/api/peers")
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONSerialization.data(
            withJSONObject: ["peer_name": peerName]
        )
        let json = try await performRequest(request)
        return MeshActionResult(
            ok: json["ok"] as? Bool ?? json["peer"] != nil,
            message: "Added peer \(peerName)"
        )
    }

    /// Remove a peer by name.
    /// DELETE /api/peers/:name
    func removeNode(peerName: String) async throws -> MeshActionResult {
        let encoded = peerName.addingPercentEncoding(
            withAllowedCharacters: .urlPathAllowed
        ) ?? peerName
        let url = baseURL.appendingPathComponent("/api/peers/\(encoded)")
        var request = URLRequest(url: url)
        request.httpMethod = "DELETE"
        let json = try await performRequest(request)
        return MeshActionResult(
            ok: json["ok"] as? Bool ?? false,
            message: "Removed peer \(peerName)"
        )
    }

    // MARK: - Internal

    /// Shared request executor with error handling.
    private func performRequest(
        _ request: URLRequest
    ) async throws -> [String: Any] {
        let (data, response) = try await session.data(for: request)

        guard let httpResponse = response as? HTTPURLResponse else {
            throw MeshActionError.networkError("invalid response type")
        }

        guard let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw MeshActionError.unexpectedResponse(
                "non-JSON body (HTTP \(httpResponse.statusCode))"
            )
        }

        if let error = json["error"] as? String {
            throw MeshActionError.serverError(
                code: httpResponse.statusCode, message: error
            )
        }

        guard (200...299).contains(httpResponse.statusCode) else {
            let message = json["message"] as? String
                ?? "HTTP \(httpResponse.statusCode)"
            throw MeshActionError.serverError(
                code: httpResponse.statusCode, message: message
            )
        }

        return json
    }
}

// MARK: - Response Types

struct MeshActionResult {
    let ok: Bool
    let message: String?
}

struct PeerHeartbeat {
    let peerName: String
    let status: String
    let ageSecs: Int
    let lastSeen: Double

    var isHealthy: Bool { status == "healthy" }
}

struct ProvisionResult: Identifiable {
    var id: String { peer }
    let peer: String
    let ip: String?
    let online: Bool
    let sshOk: Bool
    let tmuxOk: Bool
    let sessionOk: Bool
    let error: String?

    var isReady: Bool { online && sshOk && sessionOk }
}

// MARK: - Errors

enum MeshActionError: LocalizedError {
    case networkError(String)
    case serverError(code: Int, message: String)
    case unexpectedResponse(String)

    var errorDescription: String? {
        switch self {
        case .networkError(let detail):
            return "Mesh network error: \(detail)"
        case .serverError(let code, let message):
            return "Mesh server error (\(code)): \(message)"
        case .unexpectedResponse(let detail):
            return "Unexpected mesh response: \(detail)"
        }
    }
}
