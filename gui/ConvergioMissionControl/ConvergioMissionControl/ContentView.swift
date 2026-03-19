// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Tab container

import SwiftUI

/// Six-tab navigation for the mission control panel.
/// Each tab maps to a daemon subsystem: Brain, Mesh, Plans, Agents, Chat, Terminal.
/// Tab views use WebViewBridge to load embedded dashboard views from the daemon.
struct ContentView: View {
    @State private var selectedTab: Tab = .brain
    @State private var showCommandPalette: Bool = false
    @StateObject private var actionRegistry = ActionRegistry()

    var body: some View {
        ZStack {
            TabView(selection: $selectedTab) {
                BrainTab()
                    .tabItem { Label("Brain", systemImage: "brain") }
                    .tag(Tab.brain)

                MeshTab()
                    .tabItem { Label("Mesh", systemImage: "network") }
                    .tag(Tab.mesh)

                PlansTab()
                    .tabItem { Label("Plans", systemImage: "list.clipboard") }
                    .tag(Tab.plans)

                AgentsTab()
                    .tabItem { Label("Agents", systemImage: "person.3") }
                    .tag(Tab.agents)

                ChatTab()
                    .tabItem { Label("Chat", systemImage: "bubble.left.and.bubble.right") }
                    .tag(Tab.chat)

                TerminalTab()
                    .tabItem { Label("Terminal", systemImage: "terminal") }
                    .tag(Tab.terminal)
            }
            .padding(8)

            if showCommandPalette {
                CommandPalette(isPresented: $showCommandPalette, registry: actionRegistry)
                    .transition(.opacity.combined(with: .scale(scale: 0.95)))
            }
        }
        .animation(.easeOut(duration: 0.15), value: showCommandPalette)
        .onAppear { registerDefaultActions() }
        .background {
            // Invisible button to capture Cmd+K globally
            Button("") { showCommandPalette.toggle() }
                .keyboardShortcut("k", modifiers: .command)
                .hidden()
        }
    }

    /// Populate the action registry with platform-wide actions.
    private func registerDefaultActions() {
        actionRegistry.clearActions()

        // Mesh actions
        actionRegistry.registerAction(category: .mesh, label: "Sync All Nodes", icon: "arrow.triangle.2.circlepath", shortcut: "Cmd+Shift+S") {}
        actionRegistry.registerAction(category: .mesh, label: "Heartbeat Check", icon: "heart.fill") {}
        actionRegistry.registerAction(category: .mesh, label: "Delegate Task", icon: "paperplane") {}
        actionRegistry.registerAction(category: .mesh, label: "Provision Node", icon: "plus.circle") {}

        // Plan actions
        actionRegistry.registerAction(category: .plans, label: "View Plans", icon: "list.clipboard") {
            selectedTab = .plans
        }
        actionRegistry.registerAction(category: .plans, label: "Start Plan", icon: "play.fill") {}
        actionRegistry.registerAction(category: .plans, label: "Cancel Plan", icon: "xmark.circle") {}

        // Navigation actions
        for tab in Tab.allCases {
            actionRegistry.registerAction(
                category: .navigation,
                label: "Go to \(tab.rawValue.capitalized)",
                icon: tab.icon
            ) { selectedTab = tab }
        }

        // System actions
        actionRegistry.registerAction(category: .system, label: "Open Dashboard", icon: "globe", shortcut: "Cmd+D") {}
        actionRegistry.registerAction(category: .system, label: "Quit", icon: "power", shortcut: "Cmd+Q") {
            NSApplication.shared.terminate(nil)
        }
    }
}

// MARK: - Tab Enum

enum Tab: String, CaseIterable {
    case brain, mesh, plans, agents, chat, terminal

    var icon: String {
        switch self {
        case .brain: return "brain"
        case .mesh: return "network"
        case .plans: return "list.clipboard"
        case .agents: return "person.3"
        case .chat: return "bubble.left.and.bubble.right"
        case .terminal: return "terminal"
        }
    }
}
