import Observation

@MainActor
@Observable
final class CommandCenterAppModel {
    var requiresOnboarding: Bool
    var selection: SidebarItem? = .plans
    var daemonURL: String = "http://localhost:8420"
    var statusText: String = "Daemon disconnected"

    init(tokenStore: KeychainTokenStore = .shared) {
        if let token = try? tokenStore.loadToken(), !token.isEmpty {
            requiresOnboarding = false
            statusText = "Ready for live daemon data"
        } else {
            requiresOnboarding = true
            statusText = "Complete onboarding to connect"
        }
    }

    var selectedItem: SidebarItem {
        selection ?? .plans
    }

    func select(_ item: SidebarItem) {
        selection = item
    }

    func completeOnboarding(daemonURL: String) {
        self.daemonURL = daemonURL
        requiresOnboarding = false
        statusText = "Connected to \(daemonURL)"
    }
}
