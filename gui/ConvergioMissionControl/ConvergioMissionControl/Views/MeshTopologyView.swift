// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Mesh topology view with delegation status

import SwiftUI

/// Mesh network topology showing peer nodes grouped by online/offline status.
struct MeshTopologyView: View {
    @State private var viewModel = MeshTopologyViewModel()

    var body: some View {
        VStack(spacing: 0) {
            toolbar
            Divider()
            content
        }
        .onAppear { viewModel.startPolling() }
        .onDisappear { viewModel.stopPolling() }
    }

    // MARK: - Toolbar

    private var toolbar: some View {
        HStack(spacing: 8) {
            Image(systemName: "network")
                .foregroundStyle(.blue)
            Text("Mesh Topology")
                .font(.headline)
            Spacer()
            statusBadge
            Button {
                Task { await viewModel.fetchPeers() }
            } label: {
                Image(systemName: "arrow.clockwise")
            }
            .help("Refresh peers")
            .accessibilityLabel("Refresh mesh peers")
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    private var statusBadge: some View {
        Text("\(viewModel.onlineCount)/\(viewModel.totalCount) online")
            .font(.caption2.bold())
            .padding(.horizontal, 8)
            .padding(.vertical, 3)
            .background(viewModel.onlineCount > 0 ? .green.opacity(0.15) : .red.opacity(0.15),
                        in: Capsule())
            .foregroundStyle(viewModel.onlineCount > 0 ? .green : .red)
    }

    // MARK: - Content

    @ViewBuilder
    private var content: some View {
        if viewModel.isLoading && viewModel.peers.isEmpty {
            loadingView
        } else if let error = viewModel.error, viewModel.peers.isEmpty {
            errorView(error)
        } else {
            peerList
        }
    }

    private var peerList: some View {
        List {
            if !viewModel.onlinePeers.isEmpty {
                Section {
                    ForEach(viewModel.onlinePeers) { peer in
                        MeshPeerCard(peer: peer)
                    }
                } header: {
                    Label("Online (\(viewModel.onlinePeers.count))", systemImage: "circle.fill")
                        .foregroundStyle(.green)
                        .accessibilityLabel("\(viewModel.onlinePeers.count) online peers")
                }
            }
            if !viewModel.offlinePeers.isEmpty {
                Section {
                    ForEach(viewModel.offlinePeers) { peer in
                        MeshPeerCard(peer: peer)
                    }
                } header: {
                    Label("Offline (\(viewModel.offlinePeers.count))", systemImage: "circle")
                        .foregroundStyle(.secondary)
                        .accessibilityLabel("\(viewModel.offlinePeers.count) offline peers")
                }
            }
            if viewModel.peers.isEmpty {
                Text("No peers discovered")
                    .foregroundStyle(.tertiary)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 20)
            }
        }
        .listStyle(.sidebar)
        .accessibilityLabel("Mesh network peers")
    }

    // MARK: - States

    private var loadingView: some View {
        VStack(spacing: 8) {
            ProgressView()
            Text("Discovering peers...")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .accessibilityLabel("Loading mesh topology")
    }

    private func errorView(_ message: String) -> some View {
        VStack(spacing: 8) {
            Image(systemName: "exclamationmark.triangle")
                .font(.title2)
                .foregroundStyle(.orange)
            Text("Could not load peers")
                .font(.subheadline.bold())
            Text(message)
                .font(.caption)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
        }
        .padding()
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .accessibilityLabel("Error loading mesh: \(message)")
    }
}
