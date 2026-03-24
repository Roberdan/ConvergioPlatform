import SwiftUI

struct TmuxSessionPicker: View {
    let peer: String
    let sessions: [String]
    let isLoading: Bool
    let errorMessage: String?
    @Binding var newSessionName: String
    let onRefresh: () -> Void
    let onSelect: (String) -> Void
    let onCreate: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            Text("Tmux Sessions")
                .font(.title2.weight(.semibold))
            Text("Attach to \(peer) or create a new named tmux session for the current node.")
                .foregroundStyle(.secondary)

            HStack(spacing: 12) {
                Button("Refresh", action: onRefresh)
                if isLoading {
                    ProgressView()
                }
            }

            if let errorMessage {
                Label(errorMessage, systemImage: "exclamationmark.triangle")
                    .foregroundStyle(.orange)
            }

            if sessions.isEmpty, !isLoading {
                ContentUnavailableView("No tmux sessions", systemImage: "rectangle.stack")
                    .frame(maxWidth: .infinity)
            } else {
                List(sessions, id: \.self) { session in
                    Button(session) {
                        onSelect(session)
                    }
                    .buttonStyle(.plain)
                }
                .frame(minHeight: 180)
            }

            Divider()

            Text("New session")
                .font(.headline)
            TextField("session-name", text: $newSessionName)
                .textFieldStyle(.roundedBorder)
            Button("Open Session", action: onCreate)
                .keyboardShortcut(.defaultAction)
        }
        .padding(24)
        .frame(width: 420)
    }
}
