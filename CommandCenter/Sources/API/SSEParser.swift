import Foundation

struct SSEEvent: Sendable, Hashable {
    let event: String
    let data: String
    let id: String?

    func decode<T: Decodable>(_ type: T.Type, with decoder: JSONDecoder = .convergio) throws -> T {
        try decoder.decode(T.self, from: Data(data.utf8))
    }
}

enum SSEEndpoint: Sendable {
    case chat(sessionID: String)
    case meshActionStream
    case meshFullSync
    case planPreflight
    case planDelegate
    case planStart
    case meshPullDatabase

    var path: String {
        switch self {
        case .chat(let sessionID):
            return "/api/chat/stream/\(sessionID)"
        case .meshActionStream:
            return "/api/mesh/action/stream"
        case .meshFullSync:
            return "/api/mesh/fullsync"
        case .planPreflight:
            return "/api/plan/preflight"
        case .planDelegate:
            return "/api/plan/delegate"
        case .planStart:
            return "/api/plan/start"
        case .meshPullDatabase:
            return "/api/mesh/pull-db"
        }
    }
}

final class SSEParser: Sendable {
    private let client: DaemonClient
    private let session: URLSession

    init(client: DaemonClient, session: URLSession = .shared) {
        self.client = client
        self.session = session
    }

    func stream(
        _ endpoint: SSEEndpoint,
        queryItems: [URLQueryItem] = [],
        requiresAuth: Bool = true
    ) -> AsyncThrowingStream<SSEEvent, Error> {
        AsyncThrowingStream { continuation in
            Task {
                do {
                    var request = try client.makeRequest(
                        path: endpoint.path,
                        method: "GET",
                        queryItems: queryItems,
                        requiresAuth: requiresAuth
                    )
                    request.setValue("text/event-stream", forHTTPHeaderField: "Accept")

                    let (bytes, response) = try await session.bytes(for: request)
                    guard let httpResponse = response as? HTTPURLResponse else {
                        throw SSEParserError.invalidResponse
                    }
                    guard (200 ... 299).contains(httpResponse.statusCode) else {
                        throw SSEParserError.httpStatus(httpResponse.statusCode)
                    }

                    var eventName = "message"
                    var eventID: String?
                    var dataLines: [String] = []

                    for try await line in bytes.lines {
                        if line.isEmpty {
                            if !dataLines.isEmpty {
                                continuation.yield(
                                    SSEEvent(event: eventName, data: dataLines.joined(separator: "\n"), id: eventID)
                                )
                            }
                            eventName = "message"
                            eventID = nil
                            dataLines.removeAll(keepingCapacity: true)
                            continue
                        }

                        if line.hasPrefix("event:") {
                            eventName = String(line.dropFirst(6)).trimmingCharacters(in: .whitespaces)
                        } else if line.hasPrefix("data:") {
                            dataLines.append(String(line.dropFirst(5)).trimmingCharacters(in: .whitespaces))
                        } else if line.hasPrefix("id:") {
                            eventID = String(line.dropFirst(3)).trimmingCharacters(in: .whitespaces)
                        }
                    }

                    continuation.finish()
                } catch {
                    continuation.finish(throwing: error)
                }
            }
        }
    }
}

enum SSEParserError: LocalizedError {
    case invalidResponse
    case httpStatus(Int)

    var errorDescription: String? {
        switch self {
        case .invalidResponse:
            return "The daemon SSE stream did not return an HTTP response."
        case .httpStatus(let code):
            return "The daemon SSE stream failed with HTTP \(code)."
        }
    }
}
