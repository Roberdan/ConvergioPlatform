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
                    description: Text("Open a local shell or attach to a tmux session.")
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

    // MARK: - Header

    private var header: some View {
        HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: 6) {
                Text("Ghostty Terminal")
                    .font(.largeTitle.weight(.bold))
                Text("PTY shell streams with node-aware tabs and tmux attach.")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }

            Spacer()

            VStack(alignment: .trailing, spacing: 10) {
                HStack(spacing: 12) {
                    peerSelector
                    headerButtons
                }
                sessionCountBadge
            }
        }
    }

    private var peerSelector: some View {
        Picker("Node", selection: preferredPeerBinding) {
            ForEach(model.availablePeerNames, id: \.self) { peer in
                Text(displayPeer(peer)).tag(peer)
            }
        }
        .labelsHidden()
        .frame(width: 190)
        .padding(.vertical, 4)
        .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .strokeBorder(ConvergioTokens.Border.borderSubtle, lineWidth: 1)
        )
        .accessibilityLabel("Target node selector")
    }

    private var headerButtons: some View {
        HStack(spacing: 8) {
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
    }

    private var sessionCountBadge: some View {
        HStack(spacing: 6) {
            Text("\(model.tabs.count)")
                .font(.caption.weight(.bold).monospacedDigit())
                .foregroundStyle(ConvergioTokens.Surface.surface)
                .padding(.horizontal, 8)
                .padding(.vertical, 2)
                .background(ConvergioTokens.Brand.gialloFerrari, in: Capsule())

            Text("/ \(model.maxSessions) sessions")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }

    // MARK: - Tab strip

    private var tabStrip: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 6) {
                ForEach(model.tabs) { tab in
                    TerminalTabPill(
                        tab: tab,
                        isActive: model.selectedTabID == tab.id,
                        onSelect: { model.select(tabID: tab.id) },
                        onClose: { model.close(tabID: tab.id) }
                    )
                }
            }
            .padding(.vertical, 4)
            .padding(.horizontal, 2)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 6)
        .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .strokeBorder(ConvergioTokens.Border.borderSubtle, lineWidth: 1)
        )
    }

    // MARK: - Terminal pane

    private func terminalPane(for tab: TerminalSession) -> some View {
        VStack(spacing: 0) {
            GeometryReader { geometry in
                let viewport = terminalViewport(for: geometry.size)
                TerminalOutputView(text: tab.output) { data in
                    Task { await tab.send(data: data) }
                }
                .clipShape(RoundedRectangle(cornerRadius: 20, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: 20, style: .continuous)
                        .strokeBorder(ConvergioTokens.Border.border.opacity(0.5), lineWidth: 1)
                )
                .onAppear {
                    Task { await tab.resize(cols: viewport.cols, rows: viewport.rows) }
                }
                .onChange(of: viewport) { _, newViewport in
                    Task { await tab.resize(cols: newViewport.cols, rows: newViewport.rows) }
                }
            }
            .frame(minHeight: 520)

            terminalStatusBar(for: tab)

            if let errorMessage = tab.errorMessage {
                Text(errorMessage)
                    .font(.footnote)
                    .foregroundStyle(.red)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.top, 8)
            }
        }
    }

    private func terminalStatusBar(for tab: TerminalSession) -> some View {
        VStack(spacing: 0) {
            Rectangle()
                .fill(ConvergioTokens.Border.borderSubtle)
                .frame(height: 1)

            HStack(spacing: 12) {
                // Peer status dot with glow
                Circle()
                    .fill(tab.isConnected ? ConvergioTokens.Status.success : ConvergioTokens.Brand.rossoCorsa)
                    .frame(width: 7, height: 7)
                    .shadow(
                        color: (tab.isConnected ? ConvergioTokens.Status.success : ConvergioTokens.Brand.rossoCorsa).opacity(0.7),
                        radius: 4
                    )
                    .accessibilityLabel("Status: \(tab.isConnected ? "Connected" : "Disconnected")")

                Label(displayPeer(tab.peer), systemImage: tab.peer == "local" ? "desktopcomputer" : "point.3.connected.trianglepath.dotted")
                if !tab.tmuxSession.isEmpty {
                    Label(tab.tmuxSession, systemImage: "rectangle.stack")
                }
                Spacer()
                Text(tab.statusText)
                    .foregroundStyle(
                        tab.isConnected ? ConvergioTokens.Status.success : ConvergioTokens.Text.textMuted
                    )
            }
            .font(.caption)
            .padding(.horizontal, Spacing.md)
            .padding(.vertical, Spacing.xs)
            .background(.ultraThinMaterial, in: Capsule())
            .overlay(Capsule().strokeBorder(ConvergioTokens.Border.borderSubtle, lineWidth: 1))
            .padding(.top, 8)
        }
    }

    // MARK: - Bindings + helpers

    private var preferredPeerBinding: Binding<String> {
        Binding(get: { model.preferredPeer }, set: { model.preferredPeer = $0 })
    }

    private var pickerPresentedBinding: Binding<Bool> {
        Binding(get: { model.isPickerPresented }, set: { model.isPickerPresented = $0 })
    }

    private var newSessionNameBinding: Binding<String> {
        Binding(get: { model.newSessionName }, set: { model.newSessionName = $0 })
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
