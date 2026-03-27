// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — MeshPeer Codable model

import Foundation

/// A mesh peer node as returned by /api/peers.
struct MeshPeer: Identifiable, Equatable, Decodable {
    let peerName: String
    let role: String?
    let isOnline: Bool
    let isLocal: Bool
    let lastSeen: Int?
    let capabilities: [String]

    var id: String { peerName }

    /// Human-readable last seen string.
    var lastSeenDisplay: String? {
        guard let ts = lastSeen else { return nil }
        let date = Date(timeIntervalSince1970: TimeInterval(ts))
        let fmt = RelativeDateTimeFormatter()
        fmt.unitsStyle = .short
        return fmt.localizedString(for: date, relativeTo: Date())
    }

    enum CodingKeys: String, CodingKey {
        case peerName = "peer_name"
        case role
        case isOnline = "is_online"
        case isLocal = "is_local"
        case lastSeen = "last_seen"
        case capabilities
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        peerName = try c.decode(String.self, forKey: .peerName)
        role = try c.decodeIfPresent(String.self, forKey: .role)
        isOnline = try c.decodeIfPresent(Bool.self, forKey: .isOnline) ?? false
        isLocal = try c.decodeIfPresent(Bool.self, forKey: .isLocal) ?? false
        lastSeen = try c.decodeIfPresent(Int.self, forKey: .lastSeen)
        capabilities = try c.decodeIfPresent([String].self, forKey: .capabilities) ?? []
    }
}

/// Wrapper for the /api/peers response envelope.
struct PeersResponse: Decodable {
    let peers: [MeshPeer]
}
