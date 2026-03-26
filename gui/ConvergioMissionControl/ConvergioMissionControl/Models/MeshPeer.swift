// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — MeshPeer Codable model

import Foundation

/// A mesh peer node as returned by /api/peers.
struct MeshPeer: Codable, Identifiable {
    let peerName: String
    let role: String?
    let isOnline: Bool
    let isLocal: Bool
    let lastSeen: String?
    let capabilities: [String]

    /// Identifiable conformance — peerName is the stable identity.
    var id: String { peerName }

    enum CodingKeys: String, CodingKey {
        case peerName = "peer_name"
        case role
        case isOnline = "is_online"
        case isLocal = "is_local"
        case lastSeen = "last_seen"
        case capabilities
    }
}

/// Wrapper for the /api/peers response envelope.
struct PeersResponse: Codable {
    let peers: [MeshPeer]
}
