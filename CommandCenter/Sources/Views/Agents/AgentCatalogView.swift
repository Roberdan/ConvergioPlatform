import Observation
import SwiftUI

struct AgentCatalogView: View {
    @State private var model: AgentCatalogModel

    init(client: DaemonClient = DaemonClient()) {
        _model = State(initialValue: AgentCatalogModel(client: client))
    }

    var body: some View {
        HSplitView {
            VStack(spacing: 16) {
                TextField("Search agents", text: $model.searchText)
                    .textFieldStyle(.roundedBorder)

                ScrollView {
                    LazyVGrid(columns: [GridItem(.adaptive(minimum: 220), spacing: 12)], spacing: 12) {
                        ForEach(model.filteredCatalog) { agent in
                            Button {
                                model.selectedAgentName = agent.name
                            } label: {
                                agentCard(agent)
                            }
                            .buttonStyle(.plain)
                            .accessibilityLabel(
                                "Agent \(agent.name), \(agent.isEnabled ? "enabled" : "disabled")"
                            )
                        }
                    }
                }
            }
            .padding(24)
            .frame(minWidth: 360)

            VStack(spacing: 20) {
                if let agent = model.selectedAgent {
                    AgentDetailView(agent: agent) {
                        Task { await model.toggleSelectedAgent() }
                    }
                } else {
                    ContentUnavailableView("Select an agent", systemImage: "person.3.sequence")
                }

                AgentTriageView(
                    problemDescription: $model.problemDescription,
                    suggestions: model.triageSuggestions
                ) {
                    Task { await model.runTriage() }
                }

                SessionMonitor(runningAgents: model.runningAgents)
            }
            .padding(24)
        }
        .task { await model.load() }
        .overlay(alignment: .topTrailing) {
            if let errorMessage = model.errorMessage {
                Text(errorMessage)
                    .font(.footnote)
                    .foregroundStyle(.red)
                    .padding(12)
            }
        }
    }

    private func agentCard(_ agent: CatalogAgent) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 10) {
                // Avatar circle: first 2 chars of agent name, role-colored background
                let avatarColor = ConvergioTokens.Roles.color(for: roleIndex(for: agent.name))
                Circle()
                    .fill(avatarColor)
                    .frame(width: 36, height: 36)
                    .overlay(
                        Text(initials(for: agent.name))
                            .font(.label)
                            .foregroundStyle(.white)
                    )
                    .accessibilityHidden(true)

                Text(agent.name)
                    .font(.cardTitle)
                    .lineLimit(1)

                Spacer()

                // LED status dot
                Circle()
                    .fill(agent.isEnabled ? ConvergioTokens.Status.success : ConvergioTokens.Roles.role10)
                    .frame(width: 10, height: 10)
                    .accessibilityLabel("Status: \(agent.isEnabled ? "Enabled" : "Disabled")")
            }

            Text(agent.description ?? "No description available.")
                .font(.bodyMedium)
                .foregroundStyle(.secondary)
                .lineLimit(3)

            // Model badge
            if let modelName = agent.model {
                modelBadge(modelName)
                    .accessibilityLabel("Model: \(modelName)")
            }

            // Category tags as StatusBadge with distinct role colors
            HStack {
                ForEach(Array(agent.badges.enumerated()), id: \.offset) { idx, badge in
                    StatusBadge(label: badge, color: categoryColor(for: badge, index: idx))
                        .accessibilityLabel(badge)
                }
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .modifier(ConvergioCardModifier())
        .accessibilityElement(children: .combine)
        .accessibilityLabel(
            "\(agent.name), \(agent.isEnabled ? "enabled" : "disabled")" +
            (agent.description.map { ", \($0)" } ?? "") +
            (agent.model.map { ", model \($0)" } ?? "")
        )
    }

    // MARK: - Private helpers

    private func initials(for name: String) -> String {
        let words = name.split(separator: "-").map(String.init)
        if words.count >= 2 {
            return (words[0].prefix(1) + words[1].prefix(1)).uppercased()
        }
        return String(name.prefix(2)).uppercased()
    }

    /// Hash agent name to a role index 1–12 for consistent avatar color.
    private func roleIndex(for name: String) -> Int {
        let hash = name.unicodeScalars.reduce(0) { $0 &+ Int($1.value) }
        return (hash % 12) + 1
    }

    /// Assign a distinct role color per category badge by badge text hash.
    private func categoryColor(for badge: String, index: Int) -> Color {
        let hash = badge.unicodeScalars.reduce(0) { $0 &+ Int($1.value) }
        return ConvergioTokens.Roles.color(for: (hash % 12) + 1)
    }

    @ViewBuilder
    private func modelBadge(_ modelName: String) -> some View {
        let lower = modelName.lowercased()
        let (bg, fg): (Color, Color) = {
            if lower.contains("opus") {
                return (ConvergioTokens.Brand.gialloFerrari, Color(hex: 0x111111))
            } else if lower.contains("haiku") {
                return (ConvergioTokens.Brand.arancioWarm, .white)
            } else {
                // sonnet and everything else: Pietra (role10)
                return (ConvergioTokens.Roles.role10, .white)
            }
        }()
        Text(modelName)
            .font(.label)
            .foregroundStyle(fg)
            .padding(.horizontal, Spacing.xs)
            .padding(.vertical, Spacing.xxs)
            .background(Capsule().fill(bg))
    }
}

// Color(hex:) is defined in ThemeManager.swift as an internal extension — no redeclaration needed here.
