// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Embedded terminal view with PTY WebSocket

import SwiftUI

/// Native terminal view connected to daemon PTY via WebSocket.
/// Displays output in a monospaced text area with input field.
struct EmbeddedTerminalView: View {
    @State private var pty = PTYWebSocket()
    @State private var inputText = ""
    @FocusState private var inputFocused: Bool

    var body: some View {
        VStack(spacing: 0) {
            toolbar
            Divider()
            outputArea
            Divider()
            inputBar
        }
        .onAppear { pty.connect() }
        .onDisappear { pty.disconnect() }
    }

    // MARK: - Toolbar

    private var toolbar: some View {
        HStack(spacing: 8) {
            Image(systemName: "terminal")
                .foregroundStyle(.mint)
            Text("Terminal")
                .font(.headline)
            Spacer()
            connectionStatus
            Button(pty.isConnected ? "Disconnect" : "Connect") {
                if pty.isConnected { pty.disconnect() } else { pty.connect() }
            }
            .controlSize(.small)
            .accessibilityLabel(pty.isConnected ? "Disconnect terminal" : "Connect terminal")
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    private var connectionStatus: some View {
        HStack(spacing: 4) {
            Circle()
                .fill(pty.isConnected ? .green : .gray)
                .frame(width: 8, height: 8)
            Text(pty.isConnected ? "Connected" : "Disconnected")
                .font(.caption2)
                .foregroundStyle(.secondary)
        }
    }

    // MARK: - Output

    private var outputArea: some View {
        ScrollViewReader { proxy in
            ScrollView {
                Text(pty.output.isEmpty ? "Connecting to daemon PTY..." : pty.output)
                    .font(.system(.caption, design: .monospaced))
                    .foregroundStyle(pty.output.isEmpty ? .tertiary : .primary)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(8)
                    .textSelection(.enabled)
                    .id("bottom")
            }
            .onChange(of: pty.output) {
                proxy.scrollTo("bottom", anchor: .bottom)
            }
        }
        .background(Color(nsColor: .textBackgroundColor))
        .accessibilityLabel("Terminal output")
    }

    // MARK: - Input

    private var inputBar: some View {
        HStack(spacing: 8) {
            Text("$")
                .font(.system(.body, design: .monospaced))
                .foregroundStyle(.green)
            TextField("Enter command...", text: $inputText)
                .font(.system(.body, design: .monospaced))
                .textFieldStyle(.plain)
                .focused($inputFocused)
                .onSubmit { sendCommand() }
                .accessibilityLabel("Terminal command input")
            Button {
                sendCommand()
            } label: {
                Image(systemName: "arrow.right.circle.fill")
                    .foregroundStyle(.blue)
            }
            .disabled(inputText.isEmpty || !pty.isConnected)
            .accessibilityLabel("Send command")
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    private func sendCommand() {
        guard !inputText.isEmpty else { return }
        pty.send(inputText + "\n")
        inputText = ""
        inputFocused = true
    }
}
