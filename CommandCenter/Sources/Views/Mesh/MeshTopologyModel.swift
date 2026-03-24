import Foundation
import Observation

@MainActor
@Observable
final class MeshTopologyModel {
    private let client: DaemonClient
    private let brainSocket: WebSocketManager
    private var subscribed = false

    var topology: MeshTopologyResponse?
    var heartbeatByPeer: [String: HeartbeatPeer] = [:]
    var provisionByPeer: [String: MeshProvisionPeer] = [:]
    var selectedPeerID: String?
    var actionLogs: [String] = []
    var lastRefresh: Date?
    var isLoading = false
    var isRunningAction = false
    var errorMessage: String?

    init(client: DaemonClient = DaemonClient()) {
        self.client = client
        brainSocket = WebSocketManager(client: client)
    }

    var peers: [MeshPeer] {
        (topology?.peers ?? []).sorted { lhs, rhs in
            if score(lhs) == score(rhs) {
                return lhs.displayName < rhs.displayName
            }
            return score(lhs) > score(rhs)
        }
    }

    var selectedPeer: MeshPeer? {
        if let selectedPeerID, let match = peers.first(where: { $0.id == selectedPeerID }) {
            return match
        }
        return peers.first
    }

    func load() async {
        await subscribeIfNeeded()
        await refresh()
    }

    func refreshLoop() async {
        while !Task.isCancelled {
            try? await Task.sleep(for: .seconds(20))
            await refresh()
        }
    }

    func refresh() async {
        isLoading = true
        defer { isLoading = false }

        do {
            async let topologyResponse = client.meshTopology()
            async let heartbeatResponse = client.heartbeatStatus()
            let (mesh, heartbeat) = try await (topologyResponse, heartbeatResponse)
            topology = mesh
            heartbeatByPeer = Dictionary(uniqueKeysWithValues: heartbeat.peers.map { ($0.peerName, $0) })
            if let selectedPeerID, mesh.peers.contains(where: { $0.id == selectedPeerID }) {
                self.selectedPeerID = selectedPeerID
            } else {
                self.selectedPeerID = preferredPeer(from: mesh.peers)?.id
            }
            lastRefresh = Date()
            if actionLogs.isEmpty, let selectedPeer {
                appendLog("Topology loaded for \(selectedPeer.displayName).")
            }
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func select(_ peer: MeshPeer) {
        selectedPeerID = peer.id
    }

    func heartbeat(for peer: MeshPeer) -> HeartbeatPeer? {
        heartbeatByPeer[peer.peerName]
    }

    func provision(for peer: MeshPeer) -> MeshProvisionPeer? {
        provisionByPeer[peer.peerName]
    }

    func runMeshAction(_ action: String, for peer: MeshPeer) async {
        await runStream { [self] in
            self.appendLog("▶ \(action.capitalized) probe for \(peer.displayName)")
            for try await event in self.client.meshActionStream(action: action, peer: peer.peerName) {
                self.consume(event)
            }
        }
    }

    func checkProvision() async {
        await runStream { [self] in
            self.appendLog("▶ Checking mesh provision readiness")
            let response = try await self.client.meshProvision()
            self.provisionByPeer = Dictionary(uniqueKeysWithValues: response.peers.map { ($0.peer, $0) })
            self.appendLog(response.ok ? "Provision checks completed." : "Provision gaps detected.")
            if let selectedPeer = self.selectedPeer,
               let provision = self.provisionByPeer[selectedPeer.peerName] {
                self.appendLog(self.summary(for: provision))
            }
        }
    }

    func delegate(planIDText: String, to peer: MeshPeer) async {
        let trimmed = planIDText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let planID = Int(trimmed), planID > 0 else {
            errorMessage = "Enter a valid plan ID before delegating."
            return
        }

        await runStream { [self] in
            let response = try await self.client.delegatePlan(planID: planID, peer: peer.peerName)
            self.appendLog("▶ Delegating plan #\(response.planId) to \(response.delegatedTo)")
            for try await event in self.client.planDelegateStream(planID: planID, target: peer.peerName) {
                self.consume(event)
            }
        }
    }

    private func subscribeIfNeeded() async {
        guard !subscribed else { return }
        subscribed = true

        try? await brainSocket.connect(.brain) { [weak self] event in
            guard let self else { return }
            switch event {
            case .json(_, let envelope):
                guard envelope.kind == "brain_event" else { return }
                Task { @MainActor in
                    self.appendLog("↻ Live update: \(envelope.eventType ?? "event")")
                    await self.refresh()
                }
            case .text, .binary, .disconnected:
                break
            }
        }
    }

    private func runStream(_ work: @escaping () async throws -> Void) async {
        guard !isRunningAction else { return }
        isRunningAction = true
        errorMessage = nil
        actionLogs.removeAll(keepingCapacity: true)
        defer { isRunningAction = false }

        do {
            try await work()
        } catch {
            errorMessage = error.localizedDescription
            appendLog("ERROR \(error.localizedDescription)")
        }
    }

    private func consume(_ event: SSEEvent) {
        if event.event == "log" {
            event.data
                .split(whereSeparator: \.isNewline)
                .map(String.init)
                .filter { !$0.isEmpty }
                .forEach(appendLog)
            return
        }

        if let payload = try? event.decode(MeshStreamPayload.self) {
            appendLog(render(payload, fallbackEvent: event.event))
        } else if !event.data.isEmpty {
            appendLog(event.data)
        }
    }

    private func render(_ payload: MeshStreamPayload, fallbackEvent: String) -> String {
        if let id = payload.id { return "Delegation \(id)" }
        if let reason = payload.reason { return "ERROR \(reason)" }
        if let detail = payload.detail { return "→ \(payload.stage ?? payload.type ?? fallbackEvent): \(detail)" }
        if let message = payload.message { return "→ \(payload.type ?? fallbackEvent): \(message)" }
        if let taskID = payload.taskId { return "Task \(taskID): \(payload.status ?? "updated")" }
        if let status = payload.status { return "✓ \(status.capitalized)" }
        if let peer = payload.peer { return "Peer \(peer)" }
        if let ok = payload.ok { return ok ? "Completed successfully." : "Completed with issues." }
        return fallbackEvent.capitalized
    }

    private func summary(for provision: MeshProvisionPeer) -> String {
        provision.isReady
            ? "✓ \(provision.peer) is ready for delegation."
            : "⚠︎ \(provision.peer): \(provision.error ?? "provisioning incomplete")"
    }

    private func appendLog(_ message: String) {
        actionLogs.append(message)
        if actionLogs.count > 120 {
            actionLogs.removeFirst(actionLogs.count - 120)
        }
    }

    private func preferredPeer(from peers: [MeshPeer]) -> MeshPeer? {
        peers.max(by: { score($0) < score($1) })
    }

    private func score(_ peer: MeshPeer) -> Int {
        (peer.isLocal ? 10 : 0) + (peer.isOnline ? 5 : 0) + (peer.role == "coordinator" ? 2 : 0)
    }
}

private struct MeshStreamPayload: Codable {
    let id: String?
    let type: String?
    let stage: String?
    let peer: String?
    let message: String?
    let detail: String?
    let status: String?
    let ok: Bool?
    let reason: String?
    let taskId: String?
}
