// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Agent catalog: active and recent sessions

import SwiftUI

/// Main agent catalog showing active and completed daemon sessions.
/// Fetches from GET /api/agents with 10-second auto-refresh.
struct AgentCatalogView: View {
    @State private var viewModel = AgentCatalogViewModel()

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
            Image(systemName: "person.3").foregroundStyle(.green)
            Text("Agents").font(.headline)
            Spacer()
            if viewModel.totalCost > 0 {
                Text(String(format: "Total: $%.4f", viewModel.totalCost))
                    .font(.caption2.monospacedDigit())
                    .foregroundStyle(.secondary)
                    .accessibilityLabel("Total cost: \(String(format: "$%.4f", viewModel.totalCost))")
            }
            refreshButton
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    private var refreshButton: some View {
        Button {
            Task { await viewModel.fetchAgents() }
        } label: {
            Image(systemName: viewModel.isLoading ? "arrow.clockwise" : "arrow.clockwise")
                .rotationEffect(.degrees(viewModel.isLoading ? 360 : 0))
                .animation(
                    viewModel.isLoading
                        ? .linear(duration: 1).repeatForever(autoreverses: false)
                        : .default,
                    value: viewModel.isLoading
                )
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Refresh agents")
    }

    // MARK: - Content

    @ViewBuilder
    private var content: some View {
        if let error = viewModel.error, viewModel.allAgents.isEmpty {
            errorState(error)
        } else if viewModel.isLoading && viewModel.allAgents.isEmpty {
            loadingState
        } else {
            agentList
        }
    }

    private var agentList: some View {
        List {
            if !viewModel.activeAgents.isEmpty {
                Section {
                    ForEach(viewModel.activeAgents) { agent in
                        AgentSessionCard(agent: agent)
                            .listRowInsets(EdgeInsets(top: 2, leading: 8, bottom: 2, trailing: 8))
                            .listRowSeparator(.hidden)
                    }
                } header: {
                    sectionHeader(
                        "Active Sessions",
                        count: viewModel.activeAgents.count,
                        icon: "circle.fill",
                        color: .green
                    )
                }
            }

            Section {
                if viewModel.completedAgents.isEmpty && viewModel.activeAgents.isEmpty {
                    emptyState
                } else {
                    ForEach(viewModel.completedAgents) { agent in
                        AgentSessionCard(agent: agent)
                            .listRowInsets(EdgeInsets(top: 2, leading: 8, bottom: 2, trailing: 8))
                            .listRowSeparator(.hidden)
                    }
                }
            } header: {
                sectionHeader(
                    "Recent Sessions",
                    count: viewModel.completedAgents.count,
                    icon: "clock",
                    color: .secondary
                )
            }
        }
        .listStyle(.plain)
    }

    // MARK: - Section header

    private func sectionHeader(
        _ title: String, count: Int, icon: String, color: Color
    ) -> some View {
        HStack(spacing: 5) {
            Image(systemName: icon)
                .font(.caption2)
                .foregroundStyle(color)
            Text(title)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            Text("(\(count))")
                .font(.caption2)
                .foregroundStyle(.tertiary)
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(title), \(count) sessions")
    }

    // MARK: - Empty / Error / Loading

    private var emptyState: some View {
        VStack(spacing: 8) {
            Image(systemName: "person.3").font(.title2).foregroundStyle(.tertiary)
            Text("No agents running").foregroundStyle(.secondary).font(.callout)
            Text("Sessions appear when daemon agents are active")
                .font(.caption).foregroundStyle(.tertiary).multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity)
        .padding(24)
        .listRowSeparator(.hidden)
        .accessibilityLabel("No agents running")
    }

    private var loadingState: some View {
        VStack(spacing: 12) {
            ProgressView()
            Text("Loading agents…").font(.caption).foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, minHeight: 120)
        .accessibilityLabel("Loading agents")
    }

    private func errorState(_ message: String) -> some View {
        VStack(spacing: 8) {
            Image(systemName: "exclamationmark.triangle").font(.title2).foregroundStyle(.orange)
            Text("Could not reach daemon").foregroundStyle(.secondary).font(.callout)
            Text(message).font(.caption).foregroundStyle(.tertiary)
                .multilineTextAlignment(.center).lineLimit(3)
            Button("Retry") { Task { await viewModel.fetchAgents() } }
                .font(.caption).padding(.top, 4)
        }
        .frame(maxWidth: .infinity)
        .padding(24)
        .accessibilityLabel("Error: \(message)")
    }
}
