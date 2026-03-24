import SwiftUI

struct AuditTrailView: View {
    let entries: [EvolutionAuditEntry]

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Audit trail")
                .font(.title3.weight(.semibold))

            if entries.isEmpty {
                ContentUnavailableView("No audit events", systemImage: "clock.arrow.trianglehead.counterclockwise.rotate.90")
            } else {
                ForEach(entries) { entry in
                    VStack(alignment: .leading, spacing: 6) {
                        HStack {
                            Text(entry.action.capitalized)
                                .font(.headline)
                            Spacer()
                            Text(entry.actorLabel)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }

                        if let reason = entry.reason, !reason.isEmpty {
                            Text(reason)
                                .font(.footnote)
                        }

                        Text(entry.createdAt ?? "Unknown time")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                    .padding(14)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(.quaternary.opacity(0.12), in: RoundedRectangle(cornerRadius: 18, style: .continuous))
                }
            }
        }
        .padding(20)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 24, style: .continuous))
    }
}
