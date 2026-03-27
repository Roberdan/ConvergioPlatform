// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — ViewModel for mesh topology with auto-refresh

import Foundation
import Observation

/// Observable view model for mesh network topology.
/// Fetches from `GET /api/peers` every 15 seconds.
@Observable
@MainActor
final class MeshTopologyViewModel {

    // MARK: - State

    var peers: [MeshPeer] = []
    var isLoading: Bool = false
    var error: String?

    // MARK: - Computed

    var onlinePeers: [MeshPeer] { peers.filter { $0.isOnline } }
    var offlinePeers: [MeshPeer] { peers.filter { !$0.isOnline } }
    var localPeer: MeshPeer? { peers.first { $0.isLocal } }
    var onlineCount: Int { onlinePeers.count }
    var totalCount: Int { peers.count }

    // MARK: - Private

    private let baseURL: URL
    private var refreshTask: Task<Void, Never>?

    init(baseURL: URL = URL(string: "http://localhost:8420")!) {
        self.baseURL = baseURL
    }

    // MARK: - Lifecycle

    func startPolling() {
        refreshTask?.cancel()
        refreshTask = Task { [weak self] in
            while !Task.isCancelled {
                await self?.fetchPeers()
                try? await Task.sleep(for: .seconds(15))
            }
        }
    }

    func stopPolling() {
        refreshTask?.cancel()
        refreshTask = nil
    }

    // MARK: - Fetch

    func fetchPeers() async {
        isLoading = peers.isEmpty
        let url = baseURL.appendingPathComponent("api/peers")
        do {
            let (data, _) = try await URLSession.shared.data(from: url)
            let response = try JSONDecoder().decode(PeersResponse.self, from: data)
            peers = response.peers
            error = nil
        } catch {
            let message = "Failed to fetch peers: \(error.localizedDescription)"
            self.error = message
            print("[MeshTopologyViewModel] WARN: \(message)")
        }
        isLoading = false
    }
}
