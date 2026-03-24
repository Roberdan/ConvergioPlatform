// MenuBarView.swift — Rich menu-bar popover for Convergio Command Center
// Features: health LED with pulse animation, plan progress, agent count,
//           Open/Stop Daemon/Quit actions, dark-glass ConvergioCard style.
// Why: Users need at-a-glance daemon status without opening the main window.
import AppKit
import SwiftUI

// MARK: - Health state

enum DaemonHealthState {
    case healthy, degraded, offline

    var ledColor: Color {
        switch self {
        case .healthy:  ConvergioTokens.Status.success          // green
        case .degraded: ConvergioTokens.Brand.arancioWarm       // orange
        case .offline:  ConvergioTokens.Text.textMuted          // gray
        }
    }

    var sfSymbol: String {
        switch self {
        case .healthy:  "checkmark.circle.fill"
        case .degraded: "exclamationmark.triangle.fill"
        case .offline:  "xmark.circle"
        }
    }

    var label: String {
        switch self {
        case .healthy:  "Daemon healthy"
        case .degraded: "Daemon degraded"
        case .offline:  "Daemon offline"
        }
    }
}

// MARK: - ViewModel

@MainActor
@Observable
final class MenuBarViewModel {
    var health: DaemonHealthState = .offline
    var agentCount: Int = 0
    var activePlanName: String? = nil
    var activePlanProgress: Double? = nil
    var showShutdownConfirmation = false

    private let client: DaemonClient
    private var pollTask: Task<Void, Never>?

    init(client: DaemonClient = DaemonClient()) {
        self.client = client
    }

    func startPolling() {
        pollTask?.cancel()
        pollTask = Task {
            while !Task.isCancelled {
                await probe()
                try? await Task.sleep(for: .seconds(10))
            }
        }
    }

    func stopPolling() {
        pollTask?.cancel()
        pollTask = nil
    }

    func probe() async {
        let result = await client.probeHealth()
        switch result {
        case .connected(let response):
            health = response.ok ? .healthy : .degraded
            agentCount = response.agentActivity ? agentCount : 0
        case .daemonOffline:
            health = .offline
        case .failed:
            health = .degraded
        }
        // Fetch active agents count separately (best-effort)
        if health != .offline, let agents = try? await client.agents() {
            agentCount = agents.stats.activeCount
        }
    }

    func sendShutdown() async {
        struct Empty: Encodable {}
        struct ShutdownResponse: Decodable { let ok: Bool }
        _ = try? await client.post("/api/shutdown", body: Empty()) as ShutdownResponse
    }
}

// MARK: - MenuBarView

struct MenuBarView: View {
    let model: CommandCenterAppModel
    @State private var vm = MenuBarViewModel()
    @State private var pulse = false

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
            Divider().background(ConvergioTokens.Border.border)
            statusSection
            Divider().background(ConvergioTokens.Border.border)
            actionsSection
        }
        .frame(width: 260)
        .modifier(ConvergioCardModifier())
        .task { vm.startPolling() }
        .onDisappear { vm.stopPolling() }
        .alert("Stop Daemon?", isPresented: $vm.showShutdownConfirmation) {
            Button("Stop Daemon", role: .destructive) {
                Task { await vm.sendShutdown() }
            }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("This will stop the Convergio daemon process. The app will remain running.")
        }
    }

    // MARK: Header

    private var header: some View {
        HStack(spacing: Spacing.sm) {
            // Daemon LED with pulse on healthy
            ZStack {
                if vm.health == .healthy {
                    Circle()
                        .fill(ConvergioTokens.Status.success.opacity(0.3))
                        .frame(width: 18, height: 18)
                        .scaleEffect(pulse ? 1.5 : 1.0)
                        .opacity(pulse ? 0 : 0.6)
                        .animation(.easeInOut(duration: 1.4).repeatForever(autoreverses: false), value: pulse)
                }
                Image(systemName: vm.health.sfSymbol)
                    .foregroundStyle(vm.health.ledColor)
                    .font(.system(size: 14, weight: .medium))
            }
            .onAppear { pulse = true }
            .accessibilityLabel("Daemon status: \(vm.health.label)")

            VStack(alignment: .leading, spacing: 2) {
                Text("Convergio Daemon")
                    .font(.headline)
                    .foregroundStyle(ConvergioTokens.Text.textPrimary)
                Text(vm.health.label)
                    .font(.caption)
                    .foregroundStyle(vm.health.ledColor)
            }
            Spacer()
        }
        .padding(.vertical, Spacing.sm)
    }

    // MARK: Status section

    private var statusSection: some View {
        VStack(alignment: .leading, spacing: Spacing.xs) {
            // Active agents
            HStack {
                Label("\(vm.agentCount) agents running", systemImage: "person.2.fill")
                    .font(.subheadline)
                    .foregroundStyle(ConvergioTokens.Text.textMuted)
                Spacer()
            }

            // Active plan progress (if available)
            if let name = vm.activePlanName, let progress = vm.activePlanProgress {
                VStack(alignment: .leading, spacing: 4) {
                    Text(name)
                        .font(.caption.weight(.medium))
                        .foregroundStyle(ConvergioTokens.Text.textPrimary)
                        .lineLimit(1)
                    ProgressView(value: min(max(progress, 0), 1))
                        .tint(ConvergioTokens.Brand.gialloFerrari)
                }
            }
        }
        .padding(.vertical, Spacing.sm)
    }

    // MARK: Actions section

    private var actionsSection: some View {
        VStack(spacing: 4) {
            // Open Command Center
            Button {
                NSApp.setActivationPolicy(.regular)
                NSApp.activate(ignoringOtherApps: true)
                if let window = NSApp.windows.first(where: { $0.title.contains("Command Center") }) {
                    window.makeKeyAndOrderFront(nil)
                }
            } label: {
                Label("Open Command Center", systemImage: "macwindow")
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .keyboardShortcut("o")
            .buttonStyle(.plain)
            .foregroundStyle(ConvergioTokens.Text.textPrimary)
            .padding(.vertical, 4)
            .accessibilityLabel("Open Command Center")

            // Stop Daemon — destructive
            Button {
                vm.showShutdownConfirmation = true
            } label: {
                Label("Stop Daemon", systemImage: "stop.circle.fill")
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .foregroundStyle(ConvergioTokens.Status.error)
            }
            .buttonStyle(.plain)
            .padding(.vertical, 4)
            .accessibilityLabel("Stop Daemon")

            Divider().background(ConvergioTokens.Border.border)

            // Quit App
            Button {
                NSApp.terminate(nil)
            } label: {
                Label("Quit App", systemImage: "power")
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .foregroundStyle(ConvergioTokens.Text.textMuted)
            }
            .keyboardShortcut("q")
            .buttonStyle(.plain)
            .padding(.vertical, 4)
            .accessibilityLabel("Quit App")
        }
        .padding(.vertical, Spacing.xs)
    }
}
