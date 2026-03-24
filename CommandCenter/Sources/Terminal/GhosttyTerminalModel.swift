import Foundation
import Observation

@MainActor
@Observable
final class GhosttyTerminalModel {
    private let client: DaemonClient

    var peers: [PeerSummary] = []
    var tabs: [TerminalSession] = []
    var selectedTabID: UUID?
    var preferredPeer = "local"
    var isPickerPresented = false
    var pickerSessions: [String] = []
    var isPickerLoading = false
    var pickerErrorMessage: String?
    var newSessionName = ""
    var errorMessage: String?

    let maxSessions = 10

    init(client: DaemonClient = DaemonClient()) {
        self.client = client
    }

    var selectedTab: TerminalSession? {
        tabs.first { $0.id == selectedTabID }
    }

    var availablePeerNames: [String] {
        let names = Set(peers.map(\.peerName)).subtracting(["local"])
        return ["local"] + names.sorted()
    }

    var recentTmuxSessions: [String] {
        let names = Set(pickerSessions + tabs.map(\.tmuxSession).filter { !$0.isEmpty })
        return names.sorted()
    }

    func load() async {
        await refreshPeers()
        if tabs.isEmpty {
            await openShell()
        }
    }

    func refreshPeers() async {
        do {
            peers = try await client.peers().peers.sorted { lhs, rhs in
                if lhs.isLocal != rhs.isLocal {
                    return lhs.isLocal && !rhs.isLocal
                }
                return lhs.peerName < rhs.peerName
            }
            if !availablePeerNames.contains(preferredPeer) {
                preferredPeer = "local"
            }
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func openShell() async {
        await openSession(peer: preferredPeer, tmuxSession: "")
    }

    func presentTmuxPicker() async {
        isPickerPresented = true
        await refreshTmuxSessions()
    }

    func refreshTmuxSessions() async {
        isPickerLoading = true
        pickerErrorMessage = nil
        defer { isPickerLoading = false }

        do {
            pickerSessions = try await probeTmuxSessions(peer: preferredPeer)
        } catch {
            pickerErrorMessage = error.localizedDescription
        }
    }

    func attachListedSession(_ sessionName: String) async {
        isPickerPresented = false
        await openSession(peer: preferredPeer, tmuxSession: sessionName)
    }

    func attachNamedSession() async {
        let name = sanitizeTmuxSession(newSessionName)
        guard !name.isEmpty else {
            pickerErrorMessage = "Use only A–Z, 0–9, hyphen, or underscore for the tmux session name."
            return
        }

        newSessionName = ""
        isPickerPresented = false
        await openSession(peer: preferredPeer, tmuxSession: name)
    }

    func select(tabID: UUID) {
        selectedTabID = tabID
    }

    func close(tabID: UUID) {
        guard let index = tabs.firstIndex(where: { $0.id == tabID }) else { return }
        let tab = tabs.remove(at: index)
        tab.disconnect()
        if selectedTabID == tabID {
            selectedTabID = tabs.first?.id
        }
    }

    private func openSession(peer: String, tmuxSession: String) async {
        guard tabs.count < maxSessions else {
            errorMessage = "The daemon caps PTY sessions at \(maxSessions). Close a tab before opening another."
            return
        }

        let session = TerminalSession(peer: peer, tmuxSession: tmuxSession, client: client)
        tabs.append(session)
        selectedTabID = session.id
        await session.connect()
    }

    private func probeTmuxSessions(peer: String) async throws -> [String] {
        var request = try client.makeRequest(
            path: "/ws/pty",
            method: "GET",
            queryItems: [URLQueryItem(name: "peer", value: peer)],
            requiresAuth: true
        )
        if let token = try client.loadStoredToken(), !token.isEmpty {
            request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        }

        let socket = URLSession.shared.webSocketTask(with: request)
        socket.resume()

        let collector = Task { () -> String in
            var output = ""
            while !Task.isCancelled {
                do {
                    let message = try await socket.receive()
                    switch message {
                    case .string(let text):
                        output += text
                    case .data(let data):
                        output += String(decoding: data, as: UTF8.self)
                    @unknown default:
                        break
                    }
                } catch {
                    break
                }
            }
            return output
        }

        try? await Task.sleep(for: .milliseconds(250))
        try await socket.send(.string("tmux list-sessions -F \"#{session_name}\" 2>/dev/null || true\n"))
        try? await Task.sleep(for: .milliseconds(650))
        try await socket.send(.string("exit\n"))
        try? await Task.sleep(for: .milliseconds(200))

        collector.cancel()
        socket.cancel(with: .normalClosure, reason: nil)
        let output = await collector.value
        return parseSessions(from: output)
    }

    private func parseSessions(from output: String) -> [String] {
        let regex = try? NSRegularExpression(pattern: "^[A-Za-z0-9_-]{1,64}$")
        let sessions = output
            .split(whereSeparator: \.isNewline)
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { line in
                guard !line.isEmpty else { return false }
                let range = NSRange(line.startIndex..<line.endIndex, in: line)
                return regex?.firstMatch(in: line, range: range) != nil
            }
        return Array(Set(sessions)).sorted()
    }

    private func sanitizeTmuxSession(_ raw: String) -> String {
        let filtered = raw.filter { $0.isASCII && ($0.isLetter || $0.isNumber || $0 == "-" || $0 == "_") }
        return String(filtered.prefix(64))
    }
}
