import SwiftUI

struct ContentView: View {
    // Daemon connection status — will be populated by DaemonClient in future waves
    @State private var daemonStatus: DaemonStatus = .unknown

    var body: some View {
        VStack(spacing: 24) {
            Text("Convergio Command Center")
                .font(.largeTitle)
                .fontWeight(.semibold)

            Text("v1.0.0")
                .font(.subheadline)
                .foregroundStyle(.secondary)

            Divider()
                .frame(maxWidth: 300)

            HStack(spacing: 8) {
                Circle()
                    .fill(daemonStatus.color)
                    .frame(width: 10, height: 10)
                Text(daemonStatus.label)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding(40)
        .task {
            // Connection check will be implemented in T1-02 (DaemonClient)
            daemonStatus = .connecting
        }
    }
}

// MARK: - Daemon status placeholder

enum DaemonStatus {
    case unknown
    case connecting
    case connected
    case disconnected

    var label: String {
        switch self {
        case .unknown: return "Daemon: unknown"
        case .connecting: return "Daemon: connecting…"
        case .connected: return "Daemon: connected"
        case .disconnected: return "Daemon: disconnected"
        }
    }

    var color: Color {
        switch self {
        case .unknown: return .gray
        case .connecting: return .yellow
        case .connected: return .green
        case .disconnected: return .red
        }
    }
}

#Preview {
    ContentView()
        .frame(width: 600, height: 400)
}
