import SwiftUI

struct AuditTrailView: View {
    let entries: [EvolutionAuditEntry]

    var body: some View {
        VStack(alignment: .leading, spacing: Spacing.md) {
            Text("Audit trail")
                .font(.sectionTitle)
                .foregroundStyle(ConvergioTokens.Text.textPrimary)

            if entries.isEmpty {
                ContentUnavailableView(
                    "No audit events",
                    systemImage: "clock.arrow.trianglehead.counterclockwise.rotate.90"
                )
            } else {
                ForEach(Array(entries.enumerated()), id: \.element.id) { index, entry in
                    let dotColor = actionColor(entry.action)
                    HStack(alignment: .top, spacing: Spacing.sm) {
                        // Timeline column: glowing dot with ring + connector
                        VStack(spacing: 0) {
                            ZStack {
                                Circle()
                                    .fill(dotColor.opacity(0.2))
                                    .frame(width: 22, height: 22)
                                Circle()
                                    .strokeBorder(dotColor.opacity(0.5), lineWidth: 2)
                                    .frame(width: 18, height: 18)
                                Circle()
                                    .fill(dotColor)
                                    .frame(width: 10, height: 10)
                                    .shadow(color: dotColor.opacity(0.6), radius: 4, x: 0, y: 0)
                            }
                            .padding(.top, 4)

                            if index < entries.count - 1 {
                                Rectangle()
                                    .fill(
                                        LinearGradient(
                                            colors: [dotColor.opacity(0.3), ConvergioTokens.Border.borderSubtle],
                                            startPoint: .top,
                                            endPoint: .bottom
                                        )
                                    )
                                    .frame(width: 2)
                                    .frame(maxHeight: .infinity)
                            }
                        }
                        .frame(width: 22)

                        // Entry card
                        auditCard(entry)
                            .padding(.bottom, index < entries.count - 1 ? Spacing.xs : 0)
                    }
                }
            }
        }
        .convergioCard()
    }

    // MARK: - Private

    private func auditCard(_ entry: EvolutionAuditEntry) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text(entry.action.capitalized)
                    .font(.cardTitle)
                    .foregroundStyle(ConvergioTokens.Text.textPrimary)
                Spacer()
                Text(entry.actorLabel)
                    .font(.caption)
                    .foregroundStyle(ConvergioTokens.Text.textMuted)
            }

            if let reason = entry.reason, !reason.isEmpty {
                Text(reason)
                    .font(.footnote)
                    .foregroundStyle(ConvergioTokens.Text.textPrimary)
            }

            Text(entry.createdAt ?? "Unknown time")
                .font(.caption)
                .foregroundStyle(ConvergioTokens.Text.textMuted)
        }
        .padding(Spacing.sm)
        .frame(maxWidth: .infinity, alignment: .leading)
        .modifier(ConvergioCardModifier())
    }

    /// Maps an audit action keyword to a brand token color for the timeline dot.
    private func actionColor(_ action: String) -> Color {
        let lower = action.lowercased()
        if lower.contains("approv") {
            return ConvergioTokens.Brand.verdeRacing
        } else if lower.contains("reject") {
            return ConvergioTokens.Brand.rossoCorsa
        } else if lower.contains("creat") || lower.contains("submit") {
            return ConvergioTokens.Brand.gialloFerrari
        } else {
            return ConvergioTokens.Brand.azzurro
        }
    }
}
