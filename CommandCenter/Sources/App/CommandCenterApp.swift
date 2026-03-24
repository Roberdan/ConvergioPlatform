import Observation
import SwiftUI

@main
struct CommandCenterApp: App {
    @State private var model = CommandCenterAppModel()
    @State private var onboarding = OnboardingModel()
    @State private var notifications = NotificationManager.shared

    var body: some Scene {
        WindowGroup("Command Center") {
            Group {
                if model.requiresOnboarding {
                    OnboardingView(model: onboarding) { daemonURL in
                        model.completeOnboarding(daemonURL: daemonURL)
                    }
                } else {
                    ContentView(model: model, notificationManager: notifications)
                }
            }
                .frame(minWidth: 1_100, minHeight: 720)
        }
        .windowResizability(.contentMinSize)
    }
}
