import Foundation
import Observation
import SwiftUI

struct MeshNodeDetail: View {
    @Bindable var model: MeshTopologyModel
    let peer: MeshPeer?

    @State private var planID = ""

    var body: some View {
        ScrollView {
            if let peer {
                details(for: peer)
                    .id(peer.id)
            } else {
                ContentUnavailableView("Select a node", systemImage: "point.3.connected.trianglepath.dotted")
                    .frame(maxWidth: .infinity, minHeight: 400)
            }
        }
        .padding(24)
        .background(Color(nsColor: .windowBackgroundColor))
    }

    private func details(for peer: MeshPeer) -> some View {
        VStack(alignment: .leading, spacing: 20) {
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 6) {
                    Text(peer.displayName)
                        .font(.largeTitle.weight(.bold))
                    Text(peer.peerName)
                        .font(.callout.monospaced())
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Text(statusLabel(for: peer))
                    .font(.caption.weight(.semibold))
                    .padding(.horizontal, 10)
                    .padding(.vertical, 6)
                    .background(statusTint(for: peer).opacity(0.16), in: Capsule())
            }

            HStack(spacing: 12) {
                metricTile("CPU", value: percent(peer.primaryCPU))
                metricTile("Memory", value: percent((peer.memoryRatio ?? 0) * 100))
                metricTile("Tasks", value: "\(peer.activeTasks ?? 0)")
            }

            VStack(alignment: .leading, spacing: 10) {
                Text("Node details").font(.headline)
                detailRow("Role", peer.role?.capitalized ?? "Worker")
                detailRow("Platform", peer.os?.capitalized ?? "Unknown")
                detailRow("Network", peer.tailscaleIp ?? peer.dnsName ?? "No address published")
                detailRow("Aliases", peer.hostnameAliases?.joined(separator: ", ") ?? "—")
                detailRow("Capabilities", peer.capabilityList.joined(separator: ", ").isEmpty ? "—" : peer.capabilityList.joined(separator: ", "))
                detailRow("Last seen", relativeLastSeen(for: peer))
            }
            .modifier(MeshDetailCard())

            VStack(alignment: .leading, spacing: 14) {
                Text("Actions").font(.headline)

                HStack(spacing: 10) {
                    actionButton("Heartbeat", icon: "waveform.path.ecg") {
                        Task { await model.runMeshAction("heartbeat", for: peer) }
                    }
                    actionButton("Status", icon: "list.bullet.rectangle") {
                        Task { await model.runMeshAction("status", for: peer) }
                    }
                    actionButton("Sync", icon: "arrow.triangle.2.circlepath") {
                        Task { await model.runMeshAction("sync", for: peer) }
                    }
                }

                HStack(spacing: 10) {
                    TextField("Plan ID", text: $planID)
                        .textFieldStyle(.roundedBorder)

                    Button {
                        Task { await model.delegate(planIDText: planID, to: peer) }
                    } label: {
                        Label("Delegate", systemImage: "paperplane.fill")
                    }
                    .disabled(model.isRunningAction)

                    Button {
                        Task { await model.checkProvision() }
                    } label: {
                        Label("Provision", systemImage: "checklist")
                    }
                    .disabled(model.isRunningAction)
                }

                Text("Delegation posts to /api/mesh/delegate and streams stages from /api/plan/delegate.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            .modifier(MeshDetailCard())

            if let provision = model.provision(for: peer) {
                VStack(alignment: .leading, spacing: 10) {
                    Text("Provision readiness").font(.headline)
                    detailRow("SSH", provision.sshOk ? "Ready" : "Unavailable")
                    detailRow("tmux", provision.tmuxOk ? "Ready" : "Missing")
                    detailRow("Convergio session", provision.sessionOk ? "Ready" : "Missing")
                    if let error = provision.error, !error.isEmpty {
                        Text(error)
                            .font(.footnote)
                            .foregroundStyle(.red)
                    }
                }
                .modifier(MeshDetailCard())
            }

            VStack(alignment: .leading, spacing: 10) {
                HStack {
                    Text("Live log").font(.headline)
                    Spacer()
                    if model.isRunningAction {
                        ProgressView().controlSize(.small)
                    }
                }

                if model.actionLogs.isEmpty {
                    Text("Run a probe, provision check, or delegation to stream daemon events here.")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                } else {
                    ForEach(Array(model.actionLogs.enumerated()), id: \.offset) { _, line in
                        Text(line)
                            .font(.system(.caption, design: .monospaced))
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                }
            }
            .modifier(MeshDetailCard())
        }
    }

    private func statusLabel(for peer: MeshPeer) -> String {
        model.heartbeat(for: peer)?.status.capitalized ?? (peer.isOnline ? "Healthy" : "Offline")
    }

    private func statusTint(for peer: MeshPeer) -> Color {
        switch model.heartbeat(for: peer)?.status ?? (peer.isOnline ? "healthy" : "offline") {
        case "healthy": return .green
        case "stale": return .orange
        default: return .secondary
        }
    }

    private func relativeLastSeen(for peer: MeshPeer) -> String {
        guard let timestamp = peer.lastSeen else { return "Unknown" }
        let date = Date(timeIntervalSince1970: timestamp)
        return RelativeDateTimeFormatter().localizedString(for: date, relativeTo: .now)
    }

    private func percent(_ value: Double) -> String {
        String(format: "%.0f%%", value)
    }

    private func metricTile(_ title: String, value: String) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title).font(.caption).foregroundStyle(.secondary)
            Text(value).font(.title3.weight(.semibold))
        }
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .modifier(MeshDetailCard())
    }

    private func detailRow(_ title: String, _ value: String) -> some View {
        HStack(alignment: .top) {
            Text(title).font(.headline)
            Spacer(minLength: 16)
            Text(value)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.trailing)
        }
    }

    private func actionButton(_ title: String, icon: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Label(title, systemImage: icon)
                .frame(maxWidth: .infinity)
        }
        .disabled(model.isRunningAction)
    }
}

private struct MeshDetailCard: ViewModifier {
    func body(content: Content) -> some View {
        if #available(macOS 26.0, *) {
            content
                .padding(16)
                .glassEffect()
        } else {
            content
                .padding(16)
                .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 22, style: .continuous))
        }
    }
}
