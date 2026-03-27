// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — PTY WebSocket client for embedded terminal

import Foundation
import Observation

/// WebSocket client for daemon PTY endpoint (ws://localhost:8420/ws/pty).
/// Sends user input, receives terminal output as text frames.
@Observable
@MainActor
final class PTYWebSocket {
    private(set) var isConnected = false
    private(set) var output: String = ""
    private(set) var error: String?

    private var webSocket: URLSessionWebSocketTask?
    private let baseURL: URL

    init(baseURL: URL = URL(string: "ws://localhost:8420")!) {
        self.baseURL = baseURL
    }

    // MARK: - Lifecycle

    func connect() {
        let url = baseURL.appendingPathComponent("ws/pty")
        let session = URLSession(configuration: .default)
        webSocket = session.webSocketTask(with: url)
        webSocket?.resume()
        isConnected = true
        error = nil
        receiveLoop()
    }

    func disconnect() {
        webSocket?.cancel(with: .normalClosure, reason: nil)
        webSocket = nil
        isConnected = false
    }

    func send(_ text: String) {
        guard let ws = webSocket else { return }
        let message = URLSessionWebSocketTask.Message.string(text)
        ws.send(message) { [weak self] err in
            if let err {
                Task { @MainActor in
                    self?.error = "Send failed: \(err.localizedDescription)"
                }
            }
        }
    }

    // MARK: - Private

    private func receiveLoop() {
        webSocket?.receive { [weak self] result in
            Task { @MainActor in
                switch result {
                case .success(let message):
                    switch message {
                    case .string(let text):
                        self?.output += text
                        // Trim output buffer to avoid unbounded growth.
                        if let out = self?.output, out.count > 50_000 {
                            self?.output = String(out.suffix(40_000))
                        }
                    case .data(let data):
                        if let text = String(data: data, encoding: .utf8) {
                            self?.output += text
                        }
                    @unknown default:
                        break
                    }
                    self?.receiveLoop()
                case .failure(let err):
                    self?.isConnected = false
                    self?.error = "Connection lost: \(err.localizedDescription)"
                }
            }
        }
    }
}
