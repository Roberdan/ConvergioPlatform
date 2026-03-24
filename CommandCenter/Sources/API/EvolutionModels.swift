import Foundation

struct EvolutionProposalsResponse: Codable, Sendable {
    let proposals: [EvolutionProposal]
}

struct EvolutionProposal: Codable, Identifiable, Hashable, Sendable {
    let id: Int
    let hypothesis: String
    let targetMetric: String
    let expectedDelta: Double?
    let blastRadius: String?
    let status: String
    let reviewer: String?
    let reviewedAt: String?
    let reviewReason: String?
    let createdAt: String?
    let updatedAt: String?

    var expectedDeltaPercent: Double {
        (expectedDelta ?? 0) * 100
    }

    var statusLabel: String {
        status.replacingOccurrences(of: "_", with: " ").capitalized
    }
}

struct EvolutionProposalDecisionRequest: Encodable, Sendable {
    let reason: String
    let actor: String
}

struct EvolutionProposalDecisionResponse: Codable, Sendable {
    let ok: Bool
    let id: Int
    let status: String
}

struct EvolutionExperimentsResponse: Codable, Sendable {
    let experiments: [EvolutionExperiment]
}

struct EvolutionExperiment: Codable, Identifiable, Hashable, Sendable {
    let id: Int
    let proposalId: Int
    let mode: String
    let beforeMetrics: String?
    let afterMetrics: String?
    let result: String
    let startedAt: String?
    let completedAt: String?
    let hypothesis: String?
    let targetMetric: String?

    var resultLabel: String {
        result.replacingOccurrences(of: "_", with: " ").capitalized
    }

    var beforeValues: [MetricValue] {
        MetricValue.decode(from: beforeMetrics)
    }

    var afterValues: [MetricValue] {
        MetricValue.decode(from: afterMetrics)
    }
}

struct MetricValue: Identifiable, Hashable, Sendable {
    let key: String
    let value: String

    var id: String { key }

    static func decode(from raw: String?) -> [MetricValue] {
        guard
            let raw,
            let data = raw.data(using: .utf8),
            let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else {
            return []
        }

        return object.keys.sorted().compactMap { key in
            guard let value = object[key] else { return nil }
            return MetricValue(
                key: key.replacingOccurrences(of: "_", with: " ").capitalized,
                value: stringify(value)
            )
        }
    }

    private static func stringify(_ value: Any) -> String {
        if let bool = value as? Bool {
            return bool ? "true" : "false"
        }
        if let number = value as? NSNumber {
            return numberFormatter.string(from: number) ?? number.stringValue
        }
        if let string = value as? String {
            return string
        }
        return String(describing: value)
    }

    private static let numberFormatter: NumberFormatter = {
        let formatter = NumberFormatter()
        formatter.minimumFractionDigits = 0
        formatter.maximumFractionDigits = 2
        return formatter
    }()
}

struct EvolutionROIResponse: Codable, Sendable {
    let experimentsRun: Int
    let successes: Int
    let rollbacks: Int
    let successRate: Double
    let proposalsByStatus: [EvolutionStatusCount]
}

struct EvolutionStatusCount: Codable, Identifiable, Hashable, Sendable {
    let status: String
    let count: Int

    var id: String { status }

    var statusLabel: String {
        status.replacingOccurrences(of: "_", with: " ").capitalized
    }
}

struct EvolutionAuditResponse: Codable, Sendable {
    let audit: [EvolutionAuditEntry]
    let proposalId: Int
}

struct EvolutionAuditEntry: Codable, Identifiable, Hashable, Sendable {
    let id: Int
    let proposalId: Int
    let action: String
    let actor: String?
    let reason: String?
    let createdAt: String?

    var actorLabel: String {
        actor ?? "system"
    }
}
