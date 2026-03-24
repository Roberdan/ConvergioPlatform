import Foundation
import Observation

@MainActor
@Observable
final class TerminalSession: Identifiable {
    let id = UUID()
    let peer: String
    let tmuxSession: String

    private let client: DaemonClient
    private let urlSession: URLSession
    private var socketTask: URLSessionWebSocketTask?
    private var receiveTask: Task<Void, Never>?

    var output = ""
    var statusText = "Disconnected"
    var errorMessage: String?
    var isConnected = false

    init(
        peer: String,
        tmuxSession: String = "",
        client: DaemonClient,
        urlSession: URLSession = .shared
    ) {
        self.peer = peer
        self.tmuxSession = tmuxSession
        self.client = client
        self.urlSession = urlSession
    }

    var title: String {
        let base = peer == "local" ? "Local" : peer
        if tmuxSession.isEmpty {
            return "\(base) Shell"
        }
        return "\(base) · \(tmuxSession)"
    }

    func connect() async {
        do {
            var queryItems = [URLQueryItem(name: "peer", value: peer)]
            if !tmuxSession.isEmpty {
                queryItems.append(URLQueryItem(name: "tmux_session", value: tmuxSession))
            }

            var request = try client.makeRequest(
                path: "/ws/pty",
                method: "GET",
                queryItems: queryItems,
                requiresAuth: true
            )
            if let token = try client.loadStoredToken(), !token.isEmpty {
                request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
            }

            let socketTask = urlSession.webSocketTask(with: request)
            self.socketTask = socketTask
            statusText = "Connecting…"
            errorMessage = nil
            socketTask.resume()
            isConnected = true
            statusText = "Connected"

            receiveTask?.cancel()
            receiveTask = Task { [weak self] in
                await self?.receiveLoop(from: socketTask)
            }
        } catch {
            register(error)
        }
    }

    func disconnect() {
        receiveTask?.cancel()
        receiveTask = nil
        socketTask?.cancel(with: .goingAway, reason: nil)
        socketTask = nil
        isConnected = false
        statusText = "Disconnected"
    }

    func send(text: String) async {
        guard let socketTask else { return }
        do {
            try await socketTask.send(.string(text))
        } catch {
            register(error)
        }
    }

    func send(data: Data) async {
        guard let socketTask else { return }
        do {
            try await socketTask.send(.data(data))
        } catch {
            register(error)
        }
    }

    func resize(cols: Int, rows: Int) async {
        let payload = "{\"type\":\"resize\",\"cols\":\(cols),\"rows\":\(rows)}"
        await send(text: payload)
    }

    private func receiveLoop(from socketTask: URLSessionWebSocketTask) async {
        while !Task.isCancelled {
            do {
                let message = try await socketTask.receive()
                switch message {
                case .string(let text):
                    append(text)
                case .data(let data):
                    append(String(decoding: data, as: UTF8.self))
                @unknown default:
                    break
                }
            } catch {
                if Task.isCancelled { break }
                register(error)
                break
            }
        }
    }

    private func append(_ chunk: String) {
        guard !chunk.isEmpty else { return }
        output += chunk
        if output.count > 60_000 {
            output.removeFirst(output.count - 60_000)
        }
    }

    private func register(_ error: Error) {
        isConnected = false
        statusText = "Disconnected"
        errorMessage = error.localizedDescription
    }
}
