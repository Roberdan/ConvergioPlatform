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
                .foregroundStyle(ConvergioTokens.Text.textPrimary)
            Text("Attach to \(peer) or create a new named tmux session for the current node.")
                .foregroundStyle(ConvergioTokens.Text.textMuted)

            HStack(spacing: 12) {
                Button("Refresh", action: onRefresh)
                if isLoading {
                    ProgressView()
                }
            }

            if let errorMessage {
                Label(errorMessage, systemImage: "exclamationmark.triangle")
                    .foregroundStyle(ConvergioTokens.Status.warning)
            }

            if sessions.isEmpty, !isLoading {
                ContentUnavailableView("No tmux sessions", systemImage: "rectangle.stack")
                    .frame(maxWidth: .infinity)
            } else {
                // Each session item uses ConvergioCardModifier for consistent surface treatment
                VStack(spacing: 8) {
                    ForEach(sessions, id: \.self) { session in
                        Button {
                            onSelect(session)
                        } label: {
                            HStack {
                                Text(session)
                                    .foregroundStyle(ConvergioTokens.Text.textPrimary)
                                Spacer()
                                Image(systemName: "arrow.right.circle")
                                    .foregroundStyle(ConvergioTokens.Text.textMuted)
                            }
                        }
                        .buttonStyle(.plain)
                        .modifier(ConvergioCardModifier())
                    }
                }
                .frame(minHeight: 180)
            }

            Divider()
                .overlay(ConvergioTokens.Border.border)

            Text("New session")
                .font(.headline)
                .foregroundStyle(ConvergioTokens.Text.textPrimary)
            TextField("session-name", text: $newSessionName)
                .textFieldStyle(.roundedBorder)

            // AccentButtonStyle applies gialloFerrari brand treatment to primary action
            Button("Open Session", action: onCreate)
                .buttonStyle(AccentButtonStyle())
                .keyboardShortcut(.defaultAction)
        }
        .padding(24)
        .frame(width: 420)
        .background(ConvergioTokens.Surface.surfaceRaised)
    }
}
