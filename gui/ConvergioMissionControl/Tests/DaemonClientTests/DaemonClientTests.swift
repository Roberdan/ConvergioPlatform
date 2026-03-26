// SPDX-License-Identifier: MPL-2.0
// Tests for DaemonClient Codable models and error handling

import XCTest
@testable import ConvergioMissionControl

final class DaemonClientTests: XCTestCase {

    // MARK: - Plan Codable

    func testPlanDecodesFromSnakeCaseJSON() throws {
        let json = """
        {
            "id": 721,
            "name": "Plan F2 — SwiftUI Command Center",
            "status": "doing",
            "tasks_done": 3,
            "tasks_total": 10,
            "created_at": "2026-03-24 17:10:27",
            "started_at": "2026-03-26 20:41:57",
            "completed_at": null
        }
        """.data(using: .utf8)!
        let plan = try JSONDecoder().decode(Plan.self, from: json)
        XCTAssertEqual(plan.id, 721)
        XCTAssertEqual(plan.name, "Plan F2 — SwiftUI Command Center")
        XCTAssertEqual(plan.status, "doing")
        XCTAssertEqual(plan.tasksDone, 3)
        XCTAssertEqual(plan.tasksTotal, 10)
        XCTAssertEqual(plan.createdAt, "2026-03-24 17:10:27")
        XCTAssertEqual(plan.startedAt, "2026-03-26 20:41:57")
        XCTAssertNil(plan.completedAt)
    }

    func testPlanEncodesToSnakeCaseJSON() throws {
        let plan = Plan(
            id: 1,
            name: "Test Plan",
            status: "todo",
            tasksDone: 0,
            tasksTotal: 5,
            createdAt: "2026-01-01 00:00:00",
            startedAt: nil,
            completedAt: nil
        )
        let data = try JSONEncoder().encode(plan)
        let dict = try JSONSerialization.jsonObject(with: data) as! [String: Any]
        XCTAssertNotNil(dict["tasks_done"])
        XCTAssertNotNil(dict["tasks_total"])
        XCTAssertNotNil(dict["created_at"])
    }

    // MARK: - Task Codable

    func testDaemonTaskDecodesFromJSON() throws {
        let json = """
        {
            "id": 9234,
            "task_id": "T1-02",
            "title": "DaemonClient",
            "status": "in_progress",
            "priority": "P1",
            "type": "feature",
            "plan_id": 721,
            "wave_id": "W1",
            "assignee": null,
            "started_at": "2026-03-26 20:50:00",
            "completed_at": null
        }
        """.data(using: .utf8)!
        let task = try JSONDecoder().decode(DaemonTask.self, from: json)
        XCTAssertEqual(task.id, 9234)
        XCTAssertEqual(task.taskId, "T1-02")
        XCTAssertEqual(task.title, "DaemonClient")
        XCTAssertEqual(task.status, "in_progress")
        XCTAssertEqual(task.planId, 721)
    }

    // MARK: - Wave Codable

    func testWaveDecodesFromJSON() throws {
        let json = """
        {
            "id": 2222,
            "wave_id": "W1",
            "name": "Foundation",
            "status": "in_progress",
            "plan_id": 721
        }
        """.data(using: .utf8)!
        let wave = try JSONDecoder().decode(Wave.self, from: json)
        XCTAssertEqual(wave.id, 2222)
        XCTAssertEqual(wave.waveId, "W1")
        XCTAssertEqual(wave.name, "Foundation")
        XCTAssertEqual(wave.planId, 721)
    }

    // MARK: - Agent Codable

    func testAgentDecodesFromJSON() throws {
        let json = """
        {
            "agent_id": "claude-mac-1234",
            "type": "claude",
            "host": "local",
            "model": "claude-sonnet-4-6",
            "plan_id": null,
            "task_db_id": null,
            "started_at": "2026-03-26 20:17:41",
            "cost_usd": 0.12,
            "tokens_total": 5000
        }
        """.data(using: .utf8)!
        let agent = try JSONDecoder().decode(DaemonAgent.self, from: json)
        XCTAssertEqual(agent.agentId, "claude-mac-1234")
        XCTAssertEqual(agent.type, "claude")
        XCTAssertEqual(agent.host, "local")
        XCTAssertNil(agent.planId)
        XCTAssertEqual(agent.tokensTotal, 5000)
    }

    // MARK: - MeshPeer Codable

    func testMeshPeerDecodesFromJSON() throws {
        let json = """
        {
            "peer_name": "mac-studio-1",
            "role": "coordinator",
            "is_online": true,
            "is_local": false,
            "last_seen": "2026-03-26 20:00:00",
            "capabilities": ["planning", "execution"]
        }
        """.data(using: .utf8)!
        let peer = try JSONDecoder().decode(MeshPeer.self, from: json)
        XCTAssertEqual(peer.peerName, "mac-studio-1")
        XCTAssertEqual(peer.role, "coordinator")
        XCTAssertTrue(peer.isOnline)
        XCTAssertFalse(peer.isLocal)
        XCTAssertEqual(peer.capabilities, ["planning", "execution"])
    }

    // MARK: - DaemonError

    func testDaemonErrorIsLocalizedError() {
        let err = DaemonError.networkError(URLError(.timedOut))
        XCTAssertFalse(err.localizedDescription.isEmpty)

        let decodeErr = DaemonError.decodingError(
            DecodingError.dataCorrupted(.init(codingPath: [], debugDescription: "bad JSON"))
        )
        XCTAssertFalse(decodeErr.localizedDescription.isEmpty)

        let httpErr = DaemonError.httpError(statusCode: 404)
        XCTAssertFalse(httpErr.localizedDescription.isEmpty)
    }

    // MARK: - DaemonClient init

    func testDaemonClientDefaultURL() {
        let client = DaemonClient()
        XCTAssertEqual(client.baseURL.host, "localhost")
        XCTAssertEqual(client.baseURL.port, 8420)
        XCTAssertFalse(client.isConnected)
    }

    func testDaemonClientCustomURL() {
        let url = URL(string: "http://192.168.1.100:8420")!
        let client = DaemonClient(baseURL: url)
        XCTAssertEqual(client.baseURL.host, "192.168.1.100")
    }

    // MARK: - Health response

    func testHealthResponseDecodes() throws {
        let json = """
        {"ok": true, "db": true, "version": "13.0.0", "uptime_secs": 100, "peers": 5, "agent_activity": true, "tables": 193}
        """.data(using: .utf8)!
        let health = try JSONDecoder().decode(HealthResponse.self, from: json)
        XCTAssertTrue(health.ok)
        XCTAssertEqual(health.version, "13.0.0")
        XCTAssertEqual(health.peers, 5)
    }
}
