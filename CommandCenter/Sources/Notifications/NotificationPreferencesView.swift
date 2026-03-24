import SwiftUI

struct NotificationPreferencesView: View {
    let manager: NotificationManager

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Notifications")
                .font(.title3.weight(.semibold))

            Label("Permission: \(manager.authorizationStatusText)", systemImage: "bell.badge")
                .font(.subheadline)
                .foregroundStyle(.secondary)

            if let errorMessage = manager.errorMessage {
                Label(errorMessage, systemImage: "exclamationmark.triangle")
                    .font(.footnote)
                    .foregroundStyle(.orange)
            }

            Toggle("Thor approve/reject alerts", isOn: thorBinding)
            Toggle("Task completion alerts", isOn: taskBinding)
            Toggle("Mesh peer alerts", isOn: meshBinding)
            Toggle("Plan state alerts", isOn: planBinding)

            Button("Request Permission Again") {
                Task { await manager.requestPermission() }
            }

            Text("Delivered this session: \(manager.deliveredCount)")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(20)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 24, style: .continuous))
    }

    private var thorBinding: Binding<Bool> {
        Binding(get: { manager.thorResultEnabled }, set: { manager.thorResultEnabled = $0 })
    }

    private var taskBinding: Binding<Bool> {
        Binding(get: { manager.taskCompleteEnabled }, set: { manager.taskCompleteEnabled = $0 })
    }

    private var meshBinding: Binding<Bool> {
        Binding(get: { manager.meshEventEnabled }, set: { manager.meshEventEnabled = $0 })
    }

    private var planBinding: Binding<Bool> {
        Binding(get: { manager.planStateEnabled }, set: { manager.planStateEnabled = $0 })
    }
}
