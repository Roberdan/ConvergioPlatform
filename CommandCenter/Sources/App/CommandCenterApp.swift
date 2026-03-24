// CommandCenterApp.swift — App entry point with menu-bar daemon independence.
// Activation policy: .accessory when window is closed (lives in menu bar only),
//   .regular when "Open Command Center" is invoked.
// Why: daemon must stay alive without a visible window; menu bar provides
//   persistent access without dock presence.
import AppKit
import Observation
import SwiftUI

@main
struct CommandCenterApp: App {
    @State private var model = CommandCenterAppModel()
    @State private var onboarding = OnboardingModel()
    @State private var notifications = NotificationManager.shared
    @State private var themeManager = ThemeManager.shared

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
            // Provide ThemeManager to all child views via environment
            .environment(themeManager)
            // Honour the active theme's preferred color scheme
            .preferredColorScheme(themeManager.current == .avorio ? .light : .dark)
            .onDisappear {
                // App retreats to menu bar when the main window is closed
                NSApp.setActivationPolicy(.accessory)
            }
        }
        .windowResizability(.contentMinSize)

        MenuBarExtra("Convergio", systemImage: menuBarIcon) {
            MenuBarView(model: model)
        }
        .commands {
            CommandGroup(replacing: .appInfo) {
                Button("About Convergio Command Center") {
                    NSApp.orderFrontStandardAboutPanel()
                }
            }
        }
    }

    /// SF Symbol for the menu-bar item reflects daemon health.
    /// MenuBarViewModel drives health state; we read from model's statusText as a proxy.
    private var menuBarIcon: String {
        if model.requiresOnboarding { return "xmark.circle" }
        return model.statusText.contains("Connected") ? "checkmark.circle.fill" : "circle"
    }
}
