import SwiftUI

struct MenuBarView: View {
    let model: CommandCenterAppModel

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Label("Convergio Daemon", systemImage: "bolt.fill")
                .font(.headline)

            Divider()

            Label(model.statusText, systemImage: statusIcon)
                .font(.subheadline)
                .foregroundStyle(statusColor)

            Divider()

            Button("Open Command Center") {
                NSApp.activate(ignoringOtherApps: true)
                if let window = NSApp.windows.first(where: { $0.title.contains("Command Center") }) {
                    window.makeKeyAndOrderFront(nil)
                }
            }
            .keyboardShortcut("o")

            Divider()

            Button("Quit") {
                NSApplication.shared.terminate(nil)
            }
            .keyboardShortcut("q")
        }
        .padding(12)
        .frame(width: 220)
    }

    private var statusIcon: String {
        model.requiresOnboarding ? "circle.slash" : "checkmark.circle.fill"
    }

    private var statusColor: Color {
        model.requiresOnboarding ? .secondary : .green
    }
}
