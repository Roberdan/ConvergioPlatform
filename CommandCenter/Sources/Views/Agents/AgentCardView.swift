import SwiftUI

/// Premium agent card with gradient accent strip, glow avatar, and model badge.
struct AgentCardView: View {
    let agent: CatalogAgent

    @State private var isHovered = false

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            // Gradient accent strip at top
            accentStrip

            HStack(spacing: 12) {
                avatar
                Text(agent.name)
                    .font(.cardTitle)
                    .lineLimit(1)
                Spacer()
                statusDot
            }

            Text(agent.description ?? "No description available.")
                .font(.bodyMedium)
                .foregroundStyle(.secondary)
                .lineLimit(3)

            if let modelName = agent.model {
                PremiumModelBadge(modelName: modelName)
                    .accessibilityLabel("Model: \(modelName)")
            }

            HStack {
                ForEach(Array(agent.badges.enumerated()), id: \.offset) { idx, badge in
                    StatusBadge(label: badge, color: categoryColor(for: badge, index: idx))
                        .accessibilityLabel(badge)
                }
            }
        }
        .padding(.horizontal, 16)
        .padding(.bottom, 16)
        .padding(.top, 4)
        .frame(maxWidth: .infinity, alignment: .leading)
        .modifier(ConvergioCardModifier())
        .shadow(
            color: avatarColor.opacity(isHovered ? 0.25 : 0.08),
            radius: isHovered ? 16 : 8,
            y: isHovered ? 6 : 3
        )
        .scaleEffect(isHovered ? 1.02 : 1.0)
        .animation(.easeOut(duration: 0.2), value: isHovered)
        .onHover { isHovered = $0 }
        .accessibilityElement(children: .combine)
        .accessibilityLabel(accessibilityText)
    }

    // MARK: - Sub-views

    private var accentStrip: some View {
        RoundedRectangle(cornerRadius: 2)
            .fill(
                LinearGradient(
                    colors: [avatarColor, avatarColor.opacity(0.4)],
                    startPoint: .leading,
                    endPoint: .trailing
                )
            )
            .frame(height: 3)
    }

    private var avatar: some View {
        ZStack {
            Circle()
                .fill(avatarColor.opacity(0.25))
                .frame(width: 44, height: 44)
            Circle()
                .fill(avatarColor)
                .frame(width: 38, height: 38)
                .overlay(
                    Text(initials(for: agent.name))
                        .font(.label)
                        .foregroundStyle(.white)
                )
        }
        .shadow(color: avatarColor.opacity(0.5), radius: 6, y: 0)
        .accessibilityHidden(true)
    }

    private var statusDot: some View {
        Circle()
            .fill(agent.isEnabled ? ConvergioTokens.Status.success : ConvergioTokens.Roles.role10)
            .frame(width: 10, height: 10)
            .accessibilityLabel("Status: \(agent.isEnabled ? "Enabled" : "Disabled")")
    }

    // MARK: - Helpers

    private var avatarColor: Color {
        ConvergioTokens.Roles.color(for: roleIndex(for: agent.name))
    }

    private var accessibilityText: String {
        "\(agent.name), \(agent.isEnabled ? "enabled" : "disabled")" +
        (agent.description.map { ", \($0)" } ?? "") +
        (agent.model.map { ", model \($0)" } ?? "")
    }

    private func initials(for name: String) -> String {
        let words = name.split(separator: "-").map(String.init)
        if words.count >= 2 {
            return (words[0].prefix(1) + words[1].prefix(1)).uppercased()
        }
        return String(name.prefix(2)).uppercased()
    }

    private func roleIndex(for name: String) -> Int {
        let hash = name.unicodeScalars.reduce(0) { $0 &+ Int($1.value) }
        return (hash % 12) + 1
    }

    private func categoryColor(for badge: String, index: Int) -> Color {
        let hash = badge.unicodeScalars.reduce(0) { $0 &+ Int($1.value) }
        return ConvergioTokens.Roles.color(for: (hash % 12) + 1)
    }
}
