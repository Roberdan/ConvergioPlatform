import Foundation

extension DaemonClient {
    func meshTopology() async throws -> MeshTopologyResponse {
        try await get("/api/mesh")
    }

    func meshProvision() async throws -> MeshProvisionResponse {
        try await get("/api/mesh/provision")
    }

    func delegatePlan(planID: Int, peer: String) async throws -> MeshDelegateResponse {
        try await post(
            "/api/mesh/delegate",
            body: MeshDelegateRequest(planId: planID, peer: peer)
        )
    }

    func meshActionStream(action: String, peer: String) -> AsyncThrowingStream<SSEEvent, Error> {
        SSEParser(client: self).stream(
            .meshActionStream,
            queryItems: [
                URLQueryItem(name: "action", value: action),
                URLQueryItem(name: "peer", value: peer),
            ]
        )
    }

    func planDelegateStream(
        planID: Int,
        target: String,
        cli: String = "copilot"
    ) -> AsyncThrowingStream<SSEEvent, Error> {
        SSEParser(client: self).stream(
            .planDelegate,
            queryItems: [
                URLQueryItem(name: "plan_id", value: String(planID)),
                URLQueryItem(name: "target", value: target),
                URLQueryItem(name: "cli", value: cli),
            ]
        )
    }
}
