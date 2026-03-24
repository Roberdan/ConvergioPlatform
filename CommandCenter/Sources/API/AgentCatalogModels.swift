import Foundation

struct AgentCatalogResponse: Codable, Sendable {
    let ok: Bool
    let agents: [CatalogAgent]
}

struct CatalogAgent: Codable, Identifiable, Sendable {
    let name: String
    let description: String?
    let model: String?
    let category: String?
    let domain: String?
    let tools: [String]?
    let enabled: Bool?
    let path: String?

    var id: String { name }

    var isEnabled: Bool {
        enabled ?? (path != nil)
    }

    var badges: [String] {
        [category, domain].compactMap { $0 }
    }
}

struct AgentToggleRequest: Encodable, Sendable {
    let name: String
    let targetDir: String
}

struct AgentToggleResponse: Codable, Sendable {
    let ok: Bool
    let enabled: JSONValue?
    let disabled: JSONValue?
    let path: String?
}

struct AgentTriageRequest: Encodable, Sendable {
    let problemDescription: String
    let domain: String?
}

struct AgentTriageResponse: Codable, Sendable {
    let suggestions: [AgentSuggestion]
    let suggestCreation: Bool?
    let scaffoldHint: String?
}

struct AgentSuggestion: Codable, Identifiable, Sendable {
    let name: String
    let score: Double?
    let reason: String?
    let model: String?
    let description: String?

    var id: String { name }
}
