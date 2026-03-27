// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Kanban task card

import SwiftUI

/// Single task card displayed in a kanban column.
/// Accessibility: full VoiceOver label, keyboard focusable.
struct KanbanCardView: View {
    let task: DaemonTask
    @State private var isHovered = false

    var body: some View {
        HStack(spacing: 0) {
            statusBorder
            content
        }
        .background(Color(nsColor: .controlBackgroundColor), in: RoundedRectangle(cornerRadius: 8))
        .shadow(color: .black.opacity(isHovered ? 0.18 : 0.08), radius: isHovered ? 6 : 2, y: 2)
        .onHover { isHovered = $0 }
        .accessibilityElement(children: .combine)
        .accessibilityLabel(accessibilityLabel)
        .accessibilityAddTraits(.isStaticText)
    }

    // MARK: - Subviews

    private var statusBorder: some View {
        Rectangle()
            .fill(statusColor)
            .frame(width: 4)
            .clipShape(UnevenRoundedRectangle(
                topLeadingRadius: 8, bottomLeadingRadius: 8,
                bottomTrailingRadius: 0, topTrailingRadius: 0
            ))
    }

    private var content: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 4) {
                taskIdBadge
                Spacer()
                priorityPill
            }
            Text(task.title)
                .font(.subheadline)
                .lineLimit(2)
                .truncationMode(.tail)
                .foregroundStyle(.primary)
            if let wave = task.waveCode {
                Text(wave)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 8)
    }

    private var taskIdBadge: some View {
        Text("T\(task.id)")
            .font(.caption2.monospaced())
            .padding(.horizontal, 5)
            .padding(.vertical, 2)
            .background(.quaternary, in: RoundedRectangle(cornerRadius: 4))
            .foregroundStyle(.secondary)
    }

    private var priorityPill: some View {
        let (label, color) = priorityInfo
        return Text(label)
            .font(.caption2.bold())
            .padding(.horizontal, 5)
            .padding(.vertical, 2)
            .background(color.opacity(0.18), in: Capsule())
            .foregroundStyle(color)
    }

    // MARK: - Helpers

    private var statusColor: Color {
        switch task.kanbanColumn {
        case .pending: return .gray
        case .inProgress: return .blue
        case .done: return .green
        }
    }

    private var priorityInfo: (String, Color) {
        switch task.priority {
        case "P0": return ("P0", .red)
        case "P1": return ("P1", .orange)
        case "P2": return ("P2", .blue)
        default:   return ("P3", .secondary)
        }
    }

    private var accessibilityLabel: String {
        let wave = task.waveCode.map { ", wave \($0)" } ?? ""
        return "Task \(task.id), \(task.title), \(task.status), priority \(task.priority)\(wave)"
    }
}
