// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Command palette (Cmd+K)

import SwiftUI

// MARK: - Action Model

/// Categorized action executable from the command palette.
struct PaletteAction: Identifiable {
    let id = UUID()
    let category: ActionCategory
    let label: String
    let icon: String
    let shortcut: String?
    let handler: () -> Void
}

enum ActionCategory: String, CaseIterable {
    case mesh = "Mesh", plans = "Plans", navigation = "Navigation", system = "System"

    var icon: String {
        switch self {
        case .mesh: return "network"
        case .plans: return "list.clipboard"
        case .navigation: return "arrow.triangle.swap"
        case .system: return "gearshape"
        }
    }
}

// MARK: - Action Registry

/// Central registry for command palette actions.
/// Views register actions on appear; the palette filters and executes them.
final class ActionRegistry: ObservableObject {
    @Published private(set) var actions: [PaletteAction] = []

    func registerAction(
        category: ActionCategory, label: String, icon: String,
        shortcut: String? = nil, handler: @escaping () -> Void
    ) {
        actions.append(PaletteAction(
            category: category, label: label, icon: icon,
            shortcut: shortcut, handler: handler
        ))
    }

    func clearActions() { actions.removeAll() }

    /// Filter actions by search query against label and category name.
    func filtered(by query: String) -> [PaletteAction] {
        guard !query.isEmpty else { return actions }
        let q = query.lowercased()
        return actions.filter {
            $0.label.lowercased().contains(q) || $0.category.rawValue.lowercased().contains(q)
        }
    }

    /// Return actions grouped by category, preserving CaseIterable order.
    func grouped(by query: String) -> [(ActionCategory, [PaletteAction])] {
        let matches = filtered(by: query)
        return ActionCategory.allCases.compactMap { cat in
            let items = matches.filter { $0.category == cat }
            return items.isEmpty ? nil : (cat, items)
        }
    }
}

// MARK: - Command Palette View

/// Overlay command palette triggered by Cmd+K.
/// Provides search, keyboard navigation, and categorized action list.
struct CommandPalette: View {
    @Binding var isPresented: Bool
    @ObservedObject var registry: ActionRegistry
    @State private var query: String = ""
    @State private var selectedIndex: Int = 0
    @FocusState private var isSearchFocused: Bool

    var body: some View {
        ZStack {
            Color.black.opacity(0.4)
                .ignoresSafeArea()
                .onTapGesture { dismiss() }
            VStack(spacing: 0) {
                searchField
                Divider()
                actionList
            }
            .frame(width: 480, maxHeight: 400)
            .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 12))
            .shadow(color: .black.opacity(0.3), radius: 20, y: 10)
            .padding(.top, 80)
        }
        .onAppear { query = ""; selectedIndex = 0; isSearchFocused = true }
    }

    // MARK: - Search Field

    private var searchField: some View {
        HStack(spacing: 8) {
            Image(systemName: "magnifyingglass").foregroundStyle(.secondary)
            TextField("Search actions...", text: $query)
                .textFieldStyle(.plain).font(.title3)
                .focused($isSearchFocused)
                .onChange(of: query) { _ in selectedIndex = 0 }
                .onSubmit { executeSelected() }
            if !query.isEmpty {
                Button { query = "" } label: {
                    Image(systemName: "xmark.circle.fill").foregroundStyle(.secondary)
                }.buttonStyle(.plain)
            }
            shortcutBadge("esc").onTapGesture { dismiss() }
        }
        .padding(12)
    }

    // MARK: - Action List

    private var actionList: some View {
        let groups = registry.grouped(by: query)
        let flat = groups.flatMap { $0.1 }
        return ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 0) {
                    if flat.isEmpty {
                        emptyState
                    } else {
                        ForEach(Array(groups.enumerated()), id: \.offset) { _, group in
                            categorySection(group.0, actions: group.1, flat: flat)
                        }
                    }
                }
                .padding(.vertical, 4)
            }
            .onChange(of: selectedIndex) { idx in
                if idx >= 0, idx < flat.count { proxy.scrollTo(flat[idx].id, anchor: .center) }
            }
            .onKeyPress(.upArrow) { moveSelection(by: -1, count: flat.count); return .handled }
            .onKeyPress(.downArrow) { moveSelection(by: 1, count: flat.count); return .handled }
            .onKeyPress(.escape) { dismiss(); return .handled }
        }
    }

    private func categorySection(
        _ category: ActionCategory, actions: [PaletteAction], flat: [PaletteAction]
    ) -> some View {
        Section {
            ForEach(actions) { action in
                let idx = flat.firstIndex(where: { $0.id == action.id }) ?? 0
                actionRow(action, isSelected: idx == selectedIndex)
                    .id(action.id).onTapGesture { execute(action) }
            }
        } header: {
            HStack(spacing: 4) {
                Image(systemName: category.icon).font(.caption)
                Text(category.rawValue.uppercased()).font(.caption.bold())
            }
            .foregroundStyle(.secondary)
            .padding(.horizontal, 12).padding(.top, 8).padding(.bottom, 2)
        }
    }

    private func actionRow(_ action: PaletteAction, isSelected: Bool) -> some View {
        HStack {
            Image(systemName: action.icon).frame(width: 20)
                .foregroundStyle(isSelected ? .white : .primary)
            Text(action.label).foregroundStyle(isSelected ? .white : .primary)
            Spacer()
            if let shortcut = action.shortcut { shortcutBadge(shortcut) }
        }
        .padding(.horizontal, 12).padding(.vertical, 6)
        .background(isSelected ? Color.accentColor : Color.clear, in: RoundedRectangle(cornerRadius: 6))
        .contentShape(Rectangle())
    }

    private var emptyState: some View {
        VStack(spacing: 8) {
            Image(systemName: "magnifyingglass").font(.title).foregroundStyle(.tertiary)
            Text("No matching actions").foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity).padding(24)
    }

    // MARK: - Helpers

    private func shortcutBadge(_ text: String) -> some View {
        Text(text).font(.caption.monospaced())
            .padding(.horizontal, 6).padding(.vertical, 2)
            .background(.quaternary, in: RoundedRectangle(cornerRadius: 4))
    }

    private func moveSelection(by delta: Int, count: Int) {
        guard count > 0 else { return }
        selectedIndex = (selectedIndex + delta + count) % count
    }

    private func executeSelected() {
        let flat = registry.filtered(by: query)
        guard selectedIndex >= 0, selectedIndex < flat.count else { return }
        execute(flat[selectedIndex])
    }

    private func execute(_ action: PaletteAction) { dismiss(); action.handler() }
    private func dismiss() { isPresented = false }
}
