// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Brain tab: platform overview dashboard

import SwiftUI

/// Platform overview showing daemon health, plan summary, and agent activity.
struct BrainTab: View {
    @State private var health: HealthData?
    @State private var isLoading = true
    private let baseURL = URL(string: "http://localhost:8420")!

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
            if isLoading {
                ProgressView("Connecting...")
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if let h = health {
                dashboard(h)
            } else {
                errorView
            }
        }
        .onAppear { Task { await refresh() } }
    }

    private var header: some View {
        HStack {
            Image(systemName: "brain")
                .foregroundStyle(.purple)
            Text("Neural Overview")
                .font(.headline)
            Spacer()
            Button { Task { await refresh() } } label: {
                Image(systemName: "arrow.clockwise")
            }
            .accessibilityLabel("Refresh")
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    private func dashboard(_ h: HealthData) -> some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                // Status
                HStack(spacing: 8) {
                    Circle()
                        .fill(h.ok ? .green : .red)
                        .frame(width: 10, height: 10)
                    Text(h.ok ? "Online" : "Offline")
                        .font(.subheadline.bold())
                    Spacer()
                    Text("v\(h.version)")
                        .font(.caption.monospaced())
                        .foregroundStyle(.secondary)
                }

                Divider()

                // Stats grid
                LazyVGrid(columns: [GridItem(), GridItem(), GridItem()], spacing: 8) {
                    StatCard(label: "Peers", value: "\(h.peers)", icon: "network")
                    StatCard(label: "Tables", value: "\(h.tables)", icon: "tablecells")
                    StatCard(label: "Uptime", value: formatUptime(h.uptime), icon: "clock")
                }
            }
            .padding(12)
        }
    }

    private var errorView: some View {
        VStack(spacing: 8) {
            Image(systemName: "exclamationmark.triangle")
                .font(.title)
                .foregroundStyle(.orange)
            Text("Cannot reach daemon")
                .font(.subheadline)
            Text("Check that daemon is running on :8420")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func refresh() async {
        isLoading = true
        let url = baseURL.appendingPathComponent("api/health")
        do {
            let (data, _) = try await URLSession.shared.data(from: url)
            health = try JSONDecoder().decode(HealthData.self, from: data)
        } catch {
            health = nil
        }
        isLoading = false
    }

    private func formatUptime(_ secs: Int) -> String {
        let h = secs / 3600
        let m = (secs % 3600) / 60
        return h > 0 ? "\(h)h \(m)m" : "\(m)m"
    }
}

private struct HealthData: Decodable {
    let ok: Bool
    let version: String
    let peers: Int
    let tables: Int
    let uptime: Int

    enum CodingKeys: String, CodingKey {
        case ok, version, peers, tables
        case uptime = "uptime_secs"
    }
}

private struct StatCard: View {
    let label: String
    let value: String
    let icon: String

    var body: some View {
        VStack(spacing: 4) {
            Image(systemName: icon)
                .font(.title3)
                .foregroundStyle(.blue)
            Text(value)
                .font(.headline)
            Text(label)
                .font(.caption2)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity)
        .padding(8)
        .background(.quaternary, in: RoundedRectangle(cornerRadius: 8))
    }
}
