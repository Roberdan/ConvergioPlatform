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
        .background(.ultraThinMaterial)
    }

    private func details(for peer: MeshPeer) -> some View {
        VStack(alignment: .leading, spacing: 20) {
            // Premium header with gradient background
            MeshNodeHeader(peer: peer, statusLabel: statusLabel(for: peer), statusTint: statusTint(for: peer))

            // Metric tiles with colored icon circles
            HStack(spacing: 12) {
                metricTile("CPU", value: percent(peer.primaryCPU), icon: "cpu", iconColor: ConvergioTokens.Brand.azzurro)
                metricTile("Memory", value: percent((peer.memoryRatio ?? 0) * 100), icon: "memorychip", iconColor: ConvergioTokens.Brand.arancioWarm)
                metricTile("Tasks", value: "\(peer.activeTasks ?? 0)", icon: "list.bullet", iconColor: ConvergioTokens.Brand.verdeRacing)
            }

            nodeDetailsCard(for: peer)
            actionsCard(for: peer)

            if let provision = model.provision(for: peer) {
                provisionCard(provision)
            }

            liveLogCard
        }
    }

    private func nodeDetailsCard(for peer: MeshPeer) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Node details").font(.headline)
            detailRow("Role", peer.role?.capitalized ?? "Worker")
            detailRow("Platform", peer.os?.capitalized ?? "Unknown")
            detailRow("Network", peer.tailscaleIp ?? peer.dnsName ?? "No address published")
            detailRow("Aliases", peer.hostnameAliases?.joined(separator: ", ") ?? "---")
            detailRow("Capabilities", capabilitiesText(for: peer))
            detailRow("Last seen", relativeLastSeen(for: peer))
        }
        .modifier(ConvergioCardModifier())
    }

    private func actionsCard(for peer: MeshPeer) -> some View {
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
                TextField("Plan ID", text: $planID).textFieldStyle(.roundedBorder)
                Button {
                    Task { await model.delegate(planIDText: planID, to: peer) }
                } label: { Label("Delegate", systemImage: "paperplane.fill") }
                .disabled(model.isRunningAction)
                Button {
                    Task { await model.checkProvision() }
                } label: { Label("Provision", systemImage: "checklist") }
                .disabled(model.isRunningAction)
            }
            Text("Delegation posts to /api/mesh/delegate and streams stages from /api/plan/delegate.")
                .font(.caption).foregroundStyle(.secondary)
        }
        .modifier(ConvergioCardModifier())
    }

    private func provisionCard(_ provision: MeshProvisionPeer) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Provision readiness").font(.headline)
            detailRow("SSH", provision.sshOk ? "Ready" : "Unavailable")
            detailRow("tmux", provision.tmuxOk ? "Ready" : "Missing")
            detailRow("Convergio session", provision.sessionOk ? "Ready" : "Missing")
            if let error = provision.error, !error.isEmpty {
                Text(error).font(.footnote).foregroundStyle(ConvergioTokens.Status.error)
            }
        }
        .modifier(ConvergioCardModifier())
    }

    private var liveLogCard: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("Live log").font(.headline)
                Spacer()
                if model.isRunningAction { ProgressView().controlSize(.small) }
            }
            if model.actionLogs.isEmpty {
                Text("Run a probe, provision check, or delegation to stream daemon events here.")
                    .font(.footnote).foregroundStyle(.secondary)
            } else {
                ForEach(Array(model.actionLogs.enumerated()), id: \.offset) { _, line in
                    Text(line).font(.system(.caption, design: .monospaced))
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
        }
        .modifier(ConvergioCardModifier())
    }

    // MARK: - Premium metric tile

    private func metricTile(
        _ title: String, value: String, icon: String, iconColor: Color
    ) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(title).font(.caption).foregroundStyle(.secondary)
                Spacer()
                ZStack {
                    Circle().fill(iconColor.opacity(0.15)).frame(width: 24, height: 24)
                    Image(systemName: icon).font(.system(size: 11)).foregroundStyle(iconColor)
                }
            }
            Text(value).font(.title2.weight(.bold))
        }
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .modifier(ConvergioCardModifier())
        .shadow(color: iconColor.opacity(0.1), radius: 6, y: 2)
    }

    // MARK: - Helpers

    private func statusLabel(for peer: MeshPeer) -> String {
        model.heartbeat(for: peer)?.status.capitalized ?? (peer.isOnline ? "Healthy" : "Offline")
    }

    private func statusTint(for peer: MeshPeer) -> Color {
        switch model.heartbeat(for: peer)?.status ?? (peer.isOnline ? "healthy" : "offline") {
        case "healthy": return ConvergioTokens.Status.success
        case "stale":   return ConvergioTokens.Brand.arancioWarm
        default:        return ConvergioTokens.Text.textMuted
        }
    }

    private func relativeLastSeen(for peer: MeshPeer) -> String {
        guard let timestamp = peer.lastSeen else { return "Unknown" }
        return RelativeDateTimeFormatter().localizedString(
            for: Date(timeIntervalSince1970: timestamp), relativeTo: .now
        )
    }

    private func percent(_ value: Double) -> String { String(format: "%.0f%%", value) }

    private func capabilitiesText(for peer: MeshPeer) -> String {
        let caps = peer.capabilityList.joined(separator: ", ")
        return caps.isEmpty ? "---" : caps
    }

    private func detailRow(_ title: String, _ value: String) -> some View {
        HStack(alignment: .top) {
            Text(title).font(.headline)
            Spacer(minLength: 16)
            Text(value).foregroundStyle(.secondary).multilineTextAlignment(.trailing)
        }
    }

    private func actionButton(_ title: String, icon: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Label(title, systemImage: icon).frame(maxWidth: .infinity)
        }
        .disabled(model.isRunningAction)
    }
}
