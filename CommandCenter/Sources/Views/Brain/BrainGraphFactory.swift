import Foundation

enum BrainGraphFactory {
    static func makeGraph(from snapshot: BrainSnapshotResponse) -> (nodes: [BrainGraphNode], edges: [BrainGraphEdge]) {
        let planNodes = Array(snapshot.plans.prefix(8)).map(makePlanNode)
        let taskSlice = Array(snapshot.tasks.prefix(28))
        let taskNodes = taskSlice.map(makeTaskNode)
        let sessionNodes = snapshot.sessions.map(makeSessionNode)
        let sessionIDs = Set(snapshot.sessions.map(\.agentId))
        let agentNodes = snapshot.agents
            .filter { !sessionIDs.contains($0.agentId) }
            .prefix(12)
            .map(makeAgentNode)

        var nextNodes: [BrainGraphNode] = []
        var seenNodeIDs = Set<String>()
        for node in planNodes + taskNodes + sessionNodes + agentNodes where seenNodeIDs.insert(node.id).inserted {
            nextNodes.append(node)
        }

        let validIDs = Set(nextNodes.map(\.id))
        let agentPool = sessionNodes + agentNodes
        let nextEdges = taskSlice.flatMap { makeEdges(for: $0, agentPool: agentPool) }
            .filter { validIDs.contains($0.from) && validIDs.contains($0.to) }

        return (nextNodes, nextEdges)
    }

    private static func makePlanNode(_ plan: PlanSummary) -> BrainGraphNode {
        BrainGraphNode(
            id: "plan-\(plan.id)",
            title: plan.name,
            subtitle: plan.status.capitalized,
            detail: [
                "Plan ID: \(plan.id)",
                "Status: \(plan.status)",
                "Progress: \(Int(plan.progressPct ?? 0))%",
                "Tasks: \(plan.tasksDone ?? 0)/\(plan.tasksTotal ?? 0)",
                "Host: \(plan.executionHost ?? "unknown")",
            ].joined(separator: "\n"),
            kind: .plan
        )
    }

    private static func makeTaskNode(_ task: BrainTask) -> BrainGraphNode {
        BrainGraphNode(
            id: "task-\(task.id)",
            title: task.taskId ?? task.title,
            subtitle: task.status.capitalized,
            detail: [
                "DB ID: \(task.id)",
                "Status: \(task.status)",
                "Title: \(task.title)",
                "Plan: \(task.planName ?? "none")",
                "Wave: \(task.waveName ?? "n/a")",
                "Assignee: \(task.assignee ?? "unassigned")",
                "Model: \(task.model ?? "unknown")",
            ].joined(separator: "\n"),
            kind: .task
        )
    }

    private static func makeSessionNode(_ session: BrainSession) -> BrainGraphNode {
        BrainGraphNode(
            id: session.agentId,
            title: session.description ?? session.agentId,
            subtitle: [session.type, session.model].compactMap { $0 }.joined(separator: " · "),
            detail: [
                "Session: \(session.agentId)",
                "Status: \(session.status ?? "unknown")",
                "Type: \(session.type)",
                "Model: \(session.model ?? "n/a")",
                "Tokens: \(session.tokensTotal ?? 0)",
                "Cost: \(session.costUsd ?? 0)",
            ].joined(separator: "\n"),
            kind: .agent
        )
    }

    private static func makeAgentNode(_ agent: AgentRuntime) -> BrainGraphNode {
        BrainGraphNode(
            id: agent.agentId,
            title: agent.description ?? agent.agentId,
            subtitle: [agent.type, agent.model].compactMap { $0 }.joined(separator: " · "),
            detail: [
                "Agent: \(agent.agentId)",
                "Type: \(agent.type)",
                "Model: \(agent.model ?? "n/a")",
                "Host: \(agent.host ?? "unknown")",
                "Tokens: \(agent.tokensTotal ?? 0)",
            ].joined(separator: "\n"),
            kind: .agent
        )
    }

    private static func makeEdges(for task: BrainTask, agentPool: [BrainGraphNode]) -> [BrainGraphEdge] {
        let taskNodeID = "task-\(task.id)"
        var taskEdges: [BrainGraphEdge] = []

        if let planID = task.planId {
            taskEdges.append(BrainGraphEdge(from: "plan-\(planID)", to: taskNodeID, kind: "plan-task"))
        }
        if let executorSessionID = task.executorSessionId, !executorSessionID.isEmpty {
            taskEdges.append(BrainGraphEdge(from: executorSessionID, to: taskNodeID, kind: "execution"))
            return taskEdges
        }
        if let assignee = task.assignee, !assignee.isEmpty,
           let matchedAgent = agentPool.first(where: {
               $0.title.localizedCaseInsensitiveContains(assignee)
                   || $0.subtitle.localizedCaseInsensitiveContains(assignee)
           }) {
            taskEdges.append(BrainGraphEdge(from: matchedAgent.id, to: taskNodeID, kind: "assignment"))
        }

        return taskEdges
    }
}
