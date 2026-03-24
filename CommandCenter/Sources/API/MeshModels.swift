import Foundation

struct MeshTopologyResponse: Codable, Sendable {
    let peers: [MeshPeer]
    let daemonWs: String?
    let localNode: String?
}

struct MeshPeer: Codable, Identifiable, Hashable, Sendable {
    let peerName: String
    let os: String?
    let role: String?
    let capabilities: String?
    let tailscaleIp: String?
    let dnsName: String?
    let sshAlias: String?
    let macAddress: String?
    let isLocal: Bool
    let lastSeen: Double?
    let isOnline: Bool
    let cpuUsage: Double?
    let cpu: Double?
    let totalMemoryMb: Double?
    let usedMemoryMb: Double?
    let activeTasks: Int?
    let hostnameAliases: [String]?

    var id: String { peerName }

    var displayName: String {
        sshAlias ?? peerName
    }

    var primaryCPU: Double {
        cpuUsage ?? cpu ?? 0
    }

    var memoryRatio: Double? {
        guard let totalMemoryMb, totalMemoryMb > 0, let usedMemoryMb else {
            return nil
        }
        return usedMemoryMb / totalMemoryMb
    }

    var capabilityList: [String] {
        capabilities?
            .split(separator: ",")
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty } ?? []
    }
}

struct MeshProvisionResponse: Codable, Sendable {
    let ok: Bool
    let peers: [MeshProvisionPeer]
}

struct MeshProvisionPeer: Codable, Identifiable, Hashable, Sendable {
    let peer: String
    let ip: String?
    let user: String?
    let online: Bool
    let sshOk: Bool
    let tmuxOk: Bool
    let sessionOk: Bool
    let error: String?

    var id: String { peer }

    var isReady: Bool {
        online && sshOk && tmuxOk && sessionOk && error == nil
    }
}

struct MeshDelegateRequest: Encodable, Sendable {
    let planId: Int
    let peer: String
}

struct MeshDelegateResponse: Codable, Sendable {
    let ok: Bool
    let planId: Int
    let delegatedTo: String
    let streamUrl: String?
}
