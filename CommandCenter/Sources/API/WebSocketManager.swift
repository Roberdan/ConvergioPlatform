import Foundation

enum WebSocketChannel: String, Sendable {
    case brain = "/ws/brain"
    case dashboard = "/ws/dashboard"
    case pty = "/ws/pty"
}

struct WebSocketEnvelope: Codable, Sendable {
    let kind: String?
    let type: String?
    let eventType: String?
    let message: String?
    let status: String?
    let runId: Int?
    let payload: JSONValue?
}

enum WebSocketEvent: Sendable {
    case json(WebSocketChannel, WebSocketEnvelope)
    case text(WebSocketChannel, String)
    case binary(WebSocketChannel, Data)
    case disconnected(WebSocketChannel, Error?)
}

actor WebSocketManager {
    typealias EventHandler = @Sendable (WebSocketEvent) -> Void

    private let client: DaemonClient
    private let session: URLSession
    private let decoder = JSONDecoder.convergio
    private var tasks: [WebSocketChannel: URLSessionWebSocketTask] = [:]
    private var handlers: [WebSocketChannel: EventHandler] = [:]
    private var delays: [WebSocketChannel: TimeInterval] = [:]

    init(client: DaemonClient, session: URLSession = .shared) {
        self.client = client
        self.session = session
    }

    func connect(
        _ channel: WebSocketChannel,
        queryItems: [URLQueryItem] = [],
        onEvent: @escaping EventHandler
    ) async throws {
        handlers[channel] = onEvent
        delays[channel] = 1

        var request = try client.makeRequest(
            path: channel.rawValue,
            method: "GET",
            queryItems: queryItems,
            requiresAuth: channel != .dashboard && channel != .brain ? true : false
        )
        if let currentToken = try client.loadStoredToken(), !currentToken.isEmpty {
            request.setValue("Bearer \(currentToken)", forHTTPHeaderField: "Authorization")
        }

        let task = session.webSocketTask(with: request)
        tasks[channel] = task
        task.resume()
        receiveNextMessage(from: channel)
    }

    func disconnect(_ channel: WebSocketChannel) {
        tasks[channel]?.cancel(with: .goingAway, reason: nil)
        tasks[channel] = nil
        handlers[channel]?(.disconnected(channel, nil))
        handlers[channel] = nil
    }

    func send(text: String, to channel: WebSocketChannel) async throws {
        guard let task = tasks[channel] else { throw WebSocketManagerError.notConnected }
        try await task.send(.string(text))
    }

    func send(data: Data, to channel: WebSocketChannel) async throws {
        guard let task = tasks[channel] else { throw WebSocketManagerError.notConnected }
        try await task.send(.data(data))
    }

    private func receiveNextMessage(from channel: WebSocketChannel) {
        guard let task = tasks[channel] else { return }
        task.receive { [weak self] result in
            guard let self else { return }
            Task { await self.handle(result, from: channel) }
        }
    }

    private func handle(
        _ result: Result<URLSessionWebSocketTask.Message, Error>,
        from channel: WebSocketChannel
    ) async {
        switch result {
        case .success(.string(let text)):
            if let data = text.data(using: .utf8), let envelope = try? decoder.decode(WebSocketEnvelope.self, from: data) {
                handlers[channel]?(.json(channel, envelope))
            } else {
                handlers[channel]?(.text(channel, text))
            }
            delays[channel] = 1
            receiveNextMessage(from: channel)
        case .success(.data(let data)):
            handlers[channel]?(.binary(channel, data))
            delays[channel] = 1
            receiveNextMessage(from: channel)
        case .failure(let error):
            handlers[channel]?(.disconnected(channel, error))
            await reconnect(channel, after: nextDelay(for: channel))
        @unknown default:
            handlers[channel]?(.disconnected(channel, nil))
            await reconnect(channel, after: nextDelay(for: channel))
        }
    }

    private func reconnect(_ channel: WebSocketChannel, after delay: TimeInterval) async {
        try? await Task.sleep(for: .seconds(delay))
        guard let handler = handlers[channel] else { return }
        try? await connect(channel, onEvent: handler)
    }

    private func nextDelay(for channel: WebSocketChannel) -> TimeInterval {
        let current = delays[channel] ?? 1
        let next = min(current * 2, 30)
        delays[channel] = next
        return current
    }
}

enum WebSocketManagerError: LocalizedError {
    case notConnected

    var errorDescription: String? {
        "The requested WebSocket channel is not connected."
    }
}
