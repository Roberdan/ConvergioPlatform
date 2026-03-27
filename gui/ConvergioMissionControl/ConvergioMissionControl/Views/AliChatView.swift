// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Chat with Ali (daemon chat API)

import SwiftUI

/// Chat message model
struct ChatMessage: Identifiable, Equatable {
    let id = UUID()
    let role: Role
    let content: String
    let timestamp: Date

    enum Role: String {
        case user, assistant
    }
}

/// Native chat view connected to daemon chat API.
/// Sends messages to Ali orchestrator via POST /api/chat/message.
struct AliChatView: View {
    @State private var messages: [ChatMessage] = []
    @State private var inputText = ""
    @State private var isSending = false
    @FocusState private var inputFocused: Bool
    private let baseURL = URL(string: "http://localhost:8420")!

    var body: some View {
        VStack(spacing: 0) {
            toolbar
            Divider()
            messageList
            Divider()
            inputBar
        }
    }

    // MARK: - Toolbar

    private var toolbar: some View {
        HStack(spacing: 8) {
            Image(systemName: "bubble.left.and.bubble.right")
                .foregroundStyle(.teal)
            Text("Chat with Ali")
                .font(.headline)
            Spacer()
            Text("\(messages.count) messages")
                .font(.caption2)
                .foregroundStyle(.secondary)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    // MARK: - Messages

    private var messageList: some View {
        ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(spacing: 8) {
                    if messages.isEmpty {
                        emptyState
                    }
                    ForEach(messages) { msg in
                        MessageBubble(message: msg)
                    }
                    if isSending {
                        HStack {
                            ProgressView()
                                .controlSize(.small)
                            Text("Ali is thinking...")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                        .id("sending")
                    }
                }
                .padding(12)
            }
            .onChange(of: messages.count) {
                if let last = messages.last {
                    proxy.scrollTo(last.id, anchor: .bottom)
                }
            }
        }
        .accessibilityLabel("Chat messages")
    }

    private var emptyState: some View {
        VStack(spacing: 8) {
            Image(systemName: "message")
                .font(.largeTitle)
                .foregroundStyle(.tertiary)
            Text("Start a conversation with Ali")
                .font(.subheadline)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 40)
    }

    // MARK: - Input

    private var inputBar: some View {
        HStack(spacing: 8) {
            TextField("Message Ali...", text: $inputText, axis: .vertical)
                .lineLimit(1...4)
                .textFieldStyle(.plain)
                .focused($inputFocused)
                .onSubmit { sendMessage() }
                .accessibilityLabel("Chat message input")
            Button {
                sendMessage()
            } label: {
                Image(systemName: "paperplane.fill")
                    .foregroundStyle(.blue)
            }
            .disabled(inputText.trimmingCharacters(in: .whitespaces).isEmpty || isSending)
            .accessibilityLabel("Send message")
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    // MARK: - Send

    private func sendMessage() {
        let text = inputText.trimmingCharacters(in: .whitespaces)
        guard !text.isEmpty else { return }
        messages.append(ChatMessage(role: .user, content: text, timestamp: Date()))
        inputText = ""
        isSending = true
        Task {
            do {
                let reply = try await postChat(text)
                messages.append(ChatMessage(role: .assistant, content: reply, timestamp: Date()))
            } catch {
                messages.append(ChatMessage(
                    role: .assistant,
                    content: "Error: \(error.localizedDescription)",
                    timestamp: Date()
                ))
            }
            isSending = false
            inputFocused = true
        }
    }

    private func postChat(_ text: String) async throws -> String {
        let url = baseURL.appendingPathComponent("api/chat/message")
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        let body = ["message": text, "agent": "ali"]
        request.httpBody = try JSONEncoder().encode(body)
        let (data, _) = try await URLSession.shared.data(for: request)
        let json = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        return json?["reply"] as? String ?? json?["message"] as? String ?? "No response"
    }
}

/// Individual chat message bubble.
private struct MessageBubble: View {
    let message: ChatMessage

    var body: some View {
        HStack {
            if message.role == .user { Spacer(minLength: 40) }
            VStack(alignment: message.role == .user ? .trailing : .leading, spacing: 2) {
                Text(message.content)
                    .font(.body)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 8)
                    .background(
                        message.role == .user ? Color.blue : Color(nsColor: .controlBackgroundColor),
                        in: RoundedRectangle(cornerRadius: 12)
                    )
                    .foregroundStyle(message.role == .user ? .white : .primary)
                Text(message.timestamp, style: .time)
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }
            if message.role == .assistant { Spacer(minLength: 40) }
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(message.role.rawValue): \(message.content)")
    }
}
