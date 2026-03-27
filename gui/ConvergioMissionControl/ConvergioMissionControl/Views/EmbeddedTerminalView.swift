// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Terminal view: run cvg commands via daemon

import SwiftUI

/// Terminal view that runs cvg commands via the daemon REST API.
/// Executes shell commands locally and displays output.
struct EmbeddedTerminalView: View {
    @State private var inputText = ""
    @State private var output = "Welcome to Convergio Terminal\nType a command (e.g. cvg status, cvg plan list)\n\n"
    @State private var isRunning = false
    @FocusState private var inputFocused: Bool

    var body: some View {
        VStack(spacing: 0) {
            toolbar
            Divider()
            outputArea
            Divider()
            inputBar
        }
    }

    private var toolbar: some View {
        HStack(spacing: 8) {
            Image(systemName: "terminal")
                .foregroundStyle(.mint)
            Text("Terminal")
                .font(.headline)
            Spacer()
            Button("Clear") { output = "" }
                .controlSize(.small)
                .accessibilityLabel("Clear terminal output")
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    private var outputArea: some View {
        ScrollViewReader { proxy in
            ScrollView {
                Text(output)
                    .font(.system(.caption, design: .monospaced))
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(8)
                    .textSelection(.enabled)
                    .id("bottom")
            }
            .onChange(of: output) {
                proxy.scrollTo("bottom", anchor: .bottom)
            }
        }
        .background(Color(nsColor: .textBackgroundColor))
        .accessibilityLabel("Terminal output")
    }

    private var inputBar: some View {
        HStack(spacing: 8) {
            Text("$")
                .font(.system(.body, design: .monospaced))
                .foregroundStyle(.green)
            TextField("cvg status, cvg plan list...", text: $inputText)
                .font(.system(.body, design: .monospaced))
                .textFieldStyle(.plain)
                .focused($inputFocused)
                .onSubmit { runCommand() }
                .disabled(isRunning)
                .accessibilityLabel("Command input")
            if isRunning {
                ProgressView()
                    .controlSize(.small)
            } else {
                Button {
                    runCommand()
                } label: {
                    Image(systemName: "arrow.right.circle.fill")
                        .foregroundStyle(.blue)
                }
                .disabled(inputText.isEmpty)
                .accessibilityLabel("Run command")
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    private func runCommand() {
        let cmd = inputText.trimmingCharacters(in: .whitespaces)
        guard !cmd.isEmpty else { return }
        output += "$ \(cmd)\n"
        inputText = ""
        isRunning = true

        Task {
            let result = await executeShell(cmd)
            output += result + "\n"
            isRunning = false
            inputFocused = true
        }
    }

    private func executeShell(_ command: String) async -> String {
        let process = Process()
        let pipe = Pipe()
        process.executableURL = URL(fileURLWithPath: "/bin/zsh")
        process.arguments = ["-c", command]
        process.standardOutput = pipe
        process.standardError = pipe
        process.environment = ProcessInfo.processInfo.environment

        do {
            try process.run()
            process.waitUntilExit()
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            return String(data: data, encoding: .utf8)?.trimmingCharacters(in: .newlines) ?? ""
        } catch {
            return "Error: \(error.localizedDescription)"
        }
    }
}
