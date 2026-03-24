import SwiftUI

struct GhosttyTerminal: View {
    @State private var model: GhosttyTerminalModel

    init(client: DaemonClient = DaemonClient()) {
        _model = State(initialValue: GhosttyTerminalModel(client: client))
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            header
            tabStrip

            if let tab = model.selectedTab {
                terminalPane(for: tab)
            } else {
                ContentUnavailableView(
                    "No terminal sessions",
                    systemImage: "terminal",
                    description: Text("Open a local shell or attach to a tmux session to start streaming PTY output.")
                )
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .padding(24)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .task { await model.load() }
        .sheet(isPresented: pickerPresentedBinding) {
            TmuxSessionPicker(
                peer: displayPeer(model.preferredPeer),
                sessions: model.recentTmuxSessions,
                isLoading: model.isPickerLoading,
                errorMessage: model.pickerErrorMessage,
                newSessionName: newSessionNameBinding,
                onRefresh: { Task { await model.refreshTmuxSessions() } },
                onSelect: { session in Task { await model.attachListedSession(session) } },
                onCreate: { Task { await model.attachNamedSession() } }
            )
        }
        .overlay(alignment: .topTrailing) {
            if let errorMessage = model.errorMessage {
                Text(errorMessage)
                    .font(.footnote)
                    .foregroundStyle(.red)
                    .padding(12)
            }
        }
    }

    private var header: some View {
        HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: 6) {
                Text("Ghostty Terminal")
                    .font(.largeTitle.weight(.bold))
                Text("WS-PTY shell streams with node-aware tabs, tmux attach, and keyboard passthrough.")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }

            Spacer()

            VStack(alignment: .trailing, spacing: 10) {
                HStack(spacing: 12) {
                    Picker("Node", selection: preferredPeerBinding) {
                        ForEach(model.availablePeerNames, id: \.self) { peer in
                            Text(displayPeer(peer)).tag(peer)
                        }
                    }
                    .labelsHidden()
                    .frame(width: 190)
                    .accessibilityLabel("Target node selector")

                    Button {
                        Task { await model.openShell() }
                    } label: {
                        Label("New Shell", systemImage: "plus.rectangle.on.rectangle")
                    }
                    .accessibilityLabel("Open new shell on \(displayPeer(model.preferredPeer))")

                    Button {
                        Task { await model.presentTmuxPicker() }
                    } label: {
                        Label("Tmux Sessions", systemImage: "rectangle.stack.badge.person.crop")
                    }
                    .accessibilityLabel("Browse and attach tmux sessions")
                }

                Text("\(model.tabs.count)/\(model.maxSessions) sessions open. The daemon enforces the 10-session PTY cap.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
    }

    private var tabStrip: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 10) {
                ForEach(model.tabs) { tab in
                    let isActive = model.selectedTabID == tab.id
                    HStack(spacing: 8) {
                        Button(tab.title) {
                            model.select(tabID: tab.id)
                        }
                        .buttonStyle(.plain)
                        .foregroundStyle(
                            isActive
                                ? ConvergioTokens.Surface.surface
                                : ConvergioTokens.Text.textPrimary
                        )
                        .accessibilityLabel("Terminal tab: \(tab.title)\(isActive ? ", active" : "")")

                        Button {
                            model.close(tabID: tab.id)
                        } label: {
                            Image(systemName: "xmark.circle.fill")
                                .foregroundStyle(
                                    isActive
                                        ? ConvergioTokens.Surface.surface.opacity(0.7)
                                        : ConvergioTokens.Text.textMuted
                                )
                        }
                        .buttonStyle(.plain)
                        .accessibilityLabel("Close tab \(tab.title)")
                    }
                    .padding(.horizontal, 12)
                    .padding(.vertical, 8)
                    .background(
                        isActive
                            ? ConvergioTokens.Brand.gialloFerrari
                            : ConvergioTokens.Surface.surfaceRaised,
                        in: Capsule()
                    )
                }
            }
        }
    }

    private func terminalPane(for tab: TerminalSession) -> some View {
        VStack(spacing: 0) {
            GeometryReader { geometry in
                let viewport = terminalViewport(for: geometry.size)
                TerminalOutputView(text: tab.output) { data in
                    Task { await tab.send(data: data) }
                }
                .clipShape(RoundedRectangle(cornerRadius: 20, style: .continuous))
                .onAppear {
                    Task { await tab.resize(cols: viewport.cols, rows: viewport.rows) }
                }
                .onChange(of: viewport) { _, newViewport in
                    Task { await tab.resize(cols: newViewport.cols, rows: newViewport.rows) }
                }
            }
            .frame(minHeight: 520)

            HStack(spacing: 12) {
                // Peer status indicator: green dot = connected, red = disconnected
                Circle()
                    .fill(tab.isConnected ? ConvergioTokens.Status.success : ConvergioTokens.Brand.rossoCorsa)
                    .frame(width: 7, height: 7)
                    .accessibilityLabel("Status: \(tab.isConnected ? "Connected" : "Disconnected")")
                Label(displayPeer(tab.peer), systemImage: tab.peer == "local" ? "desktopcomputer" : "point.3.connected.trianglepath.dotted")
                if !tab.tmuxSession.isEmpty {
                    Label(tab.tmuxSession, systemImage: "rectangle.stack")
                }
                Spacer()
                Text(tab.statusText)
                    .foregroundStyle(
                        tab.isConnected
                            ? ConvergioTokens.Status.success
                            : ConvergioTokens.Text.textMuted
                    )
            }
            .font(.caption)
            .modifier(ConvergioCardModifier())

            if let errorMessage = tab.errorMessage {
                Text(errorMessage)
                    .font(.footnote)
                    .foregroundStyle(.red)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.top, 8)
            }
        }
    }

    private var preferredPeerBinding: Binding<String> {
        Binding(
            get: { model.preferredPeer },
            set: { model.preferredPeer = $0 }
        )
    }

    private var pickerPresentedBinding: Binding<Bool> {
        Binding(
            get: { model.isPickerPresented },
            set: { model.isPickerPresented = $0 }
        )
    }

    private var newSessionNameBinding: Binding<String> {
        Binding(
            get: { model.newSessionName },
            set: { model.newSessionName = $0 }
        )
    }

    private func displayPeer(_ peer: String) -> String {
        peer == "local" ? "Local Node" : peer
    }

    private func terminalViewport(for size: CGSize) -> TerminalViewport {
        let cols = max(Int(size.width / 8), 40)
        let rows = max(Int(size.height / 18), 12)
        return TerminalViewport(cols: cols, rows: rows)
    }
}

private struct TerminalViewport: Equatable {
    let cols: Int
    let rows: Int
}
