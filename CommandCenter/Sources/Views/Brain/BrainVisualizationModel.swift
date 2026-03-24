import CoreGraphics
import Foundation
import Observation

@MainActor
@Observable
final class BrainVisualizationModel {
    private let client: DaemonClient
    private let brainSocket: WebSocketManager
    private var subscribed = false
    private var simulationTask: Task<Void, Never>?

    var nodes: [BrainGraphNode] = []
    var edges: [BrainGraphEdge] = []
    var layout: [String: BrainNodeState] = [:]
    var selectedNodeID: String?
    var viewportSize: CGSize = .zero
    var shaderReady = false
    var isLoading = false
    var errorMessage: String?

    init(client: DaemonClient = DaemonClient()) {
        self.client = client
        brainSocket = WebSocketManager(client: client)
    }

    var selectedNode: BrainGraphNode? {
        nodes.first { $0.id == selectedNodeID }
    }

    var planCount: Int { nodes.filter { $0.kind == .plan }.count }
    var taskCount: Int { nodes.filter { $0.kind == .task }.count }
    var agentCount: Int { nodes.filter { $0.kind == .agent }.count }

    func load() async {
        shaderReady = BrainMetalSupport.isShaderAvailable()
        await subscribeIfNeeded()
        await refresh()
        startSimulationIfNeeded()
    }

    func refresh() async {
        isLoading = true
        defer { isLoading = false }

        do {
            let snapshot = try await client.brainSnapshot()
            let graph = BrainGraphFactory.makeGraph(from: snapshot)
            nodes = graph.nodes
            edges = graph.edges
            ensureLayout()
            updateSelection()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func setViewport(_ size: CGSize) {
        viewportSize = size
        ensureLayout()
    }

    func select(_ nodeID: String) {
        selectedNodeID = nodeID
    }

    func position(for nodeID: String) -> CGPoint? {
        layout[nodeID]?.position
    }

    private func subscribeIfNeeded() async {
        guard !subscribed else { return }
        subscribed = true

        try? await brainSocket.connect(.brain) { [weak self] event in
            guard let self else { return }
            switch event {
            case .json(_, let envelope):
                guard envelope.kind == "brain_event" || envelope.type == "notification" else { return }
                Task { @MainActor in await self.refresh() }
            case .text:
                Task { @MainActor in await self.refresh() }
            case .binary, .disconnected:
                break
            }
        }
    }

    private func updateSelection() {
        let validIDs = Set(nodes.map(\.id))
        if let selectedNodeID, !validIDs.contains(selectedNodeID) {
            self.selectedNodeID = nodes.first?.id
        }
        if selectedNodeID == nil {
            selectedNodeID = nodes.first?.id
        }
    }

    private func ensureLayout() {
        guard !nodes.isEmpty else {
            layout.removeAll()
            return
        }

        let width = max(viewportSize.width, 900)
        let height = max(viewportSize.height, 560)
        let validIDs = Set(nodes.map(\.id))
        layout = layout.filter { validIDs.contains($0.key) }

        for node in nodes where layout[node.id] == nil {
            layout[node.id] = BrainNodeState(
                position: CGPoint(
                    x: CGFloat.random(in: 60...(width - 60)),
                    y: CGFloat.random(in: 60...(height - 60))
                ),
                velocity: CGVector(dx: .random(in: -2...2), dy: .random(in: -2...2))
            )
        }
    }

    private func startSimulationIfNeeded() {
        guard simulationTask == nil else { return }
        simulationTask = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(for: .milliseconds(16))
                await MainActor.run {
                    self?.advancePhysics()
                }
            }
        }
    }

    private func advancePhysics() {
        guard viewportSize.width > 0, viewportSize.height > 0, nodes.count > 1 else { return }
        ensureLayout()

        let center = CGPoint(x: viewportSize.width / 2, y: viewportSize.height / 2)
        var forces = Dictionary(uniqueKeysWithValues: nodes.map { ($0.id, CGVector(dx: 0, dy: 0)) })
        applyCenteringForces(to: &forces, center: center)
        applyRepulsionForces(to: &forces)
        applyEdgeForces(to: &forces)
        updateLayout(using: forces)
    }

    private func applyCenteringForces(to forces: inout [String: CGVector], center: CGPoint) {
        for node in nodes {
            guard let state = layout[node.id] else { continue }
            forces[node.id] = add(
                forces[node.id],
                dx: (center.x - state.position.x) * 0.0006,
                dy: (center.y - state.position.y) * 0.0006
            )
        }
    }

    private func applyRepulsionForces(to forces: inout [String: CGVector]) {
        for lhsIndex in nodes.indices {
            for rhsIndex in nodes.indices where rhsIndex > lhsIndex {
                let lhsID = nodes[lhsIndex].id
                let rhsID = nodes[rhsIndex].id
                guard let lhs = layout[lhsID], let rhs = layout[rhsID] else { continue }

                let deltaX = rhs.position.x - lhs.position.x
                let deltaY = rhs.position.y - lhs.position.y
                let distanceSquared = max(deltaX * deltaX + deltaY * deltaY, 24)
                let distance = sqrt(distanceSquared)
                let strength = 1_600 / distanceSquared
                let pushX = (deltaX / distance) * strength
                let pushY = (deltaY / distance) * strength

                forces[lhsID] = add(forces[lhsID], dx: -pushX, dy: -pushY)
                forces[rhsID] = add(forces[rhsID], dx: pushX, dy: pushY)
            }
        }
    }

    private func applyEdgeForces(to forces: inout [String: CGVector]) {
        for edge in edges {
            guard let fromState = layout[edge.from], let toState = layout[edge.to] else { continue }
            let deltaX = toState.position.x - fromState.position.x
            let deltaY = toState.position.y - fromState.position.y
            let distance = max(sqrt(deltaX * deltaX + deltaY * deltaY), 1)
            let desiredLength: CGFloat = edge.kind == "plan-task" ? 130 : 95
            let stretch = distance - desiredLength
            let spring = stretch * 0.0022
            let pullX = (deltaX / distance) * spring
            let pullY = (deltaY / distance) * spring

            forces[edge.from] = add(forces[edge.from], dx: pullX, dy: pullY)
            forces[edge.to] = add(forces[edge.to], dx: -pullX, dy: -pullY)
        }
    }

    private func updateLayout(using forces: [String: CGVector]) {
        for node in nodes {
            guard var state = layout[node.id] else { continue }
            let force = forces[node.id] ?? .init(dx: 0, dy: 0)
            state.velocity.dx = (state.velocity.dx + force.dx) * 0.82
            state.velocity.dy = (state.velocity.dy + force.dy) * 0.82

            let maxVelocity: CGFloat = 18
            state.velocity.dx = min(max(state.velocity.dx, -maxVelocity), maxVelocity)
            state.velocity.dy = min(max(state.velocity.dy, -maxVelocity), maxVelocity)
            state.position.x = min(max(40, state.position.x + state.velocity.dx), viewportSize.width - 40)
            state.position.y = min(max(40, state.position.y + state.velocity.dy), viewportSize.height - 40)
            layout[node.id] = state
        }
    }

    private func add(_ vector: CGVector?, dx: CGFloat, dy: CGFloat) -> CGVector {
        CGVector(dx: (vector?.dx ?? 0) + dx, dy: (vector?.dy ?? 0) + dy)
    }
}
