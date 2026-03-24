import SwiftUI

struct NotificationPreferencesView: View {
    let manager: NotificationManager

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            headerRow
            permissionRow
            errorRow
            categoryToggles
            testButton
            deliveredRow
        }
        .convergioCard()
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    // MARK: - Sub-views

    private var headerRow: some View {
        Text("Notifications")
            .font(.title3.weight(.semibold))
    }

    private var permissionRow: some View {
        HStack(spacing: 8) {
            Image(systemName: "bell.badge")
                .foregroundStyle(ConvergioTokens.Text.textMuted)
            Text("Permission")
                .font(.subheadline)
                .foregroundStyle(.secondary)
            Spacer()
            permissionBadge
        }
    }

    @ViewBuilder
    private var permissionBadge: some View {
        switch manager.authorizationStatusText {
        case "Allowed":
            StatusBadge(label: "Allowed", color: ConvergioTokens.Status.success)
        case "Denied":
            StatusBadge(label: "Denied", color: ConvergioTokens.Status.error)
        default:
            StatusBadge(label: manager.authorizationStatusText,
                        color: ConvergioTokens.Brand.gialloFerrari)
        }
    }

    @ViewBuilder
    private var errorRow: some View {
        if let errorMessage = manager.errorMessage {
            Label(errorMessage, systemImage: "exclamationmark.triangle")
                .font(.footnote)
                .foregroundStyle(ConvergioTokens.Status.warning)
        }
    }

    private var categoryToggles: some View {
        VStack(spacing: 10) {
            categoryToggle(
                label: "Thor approve/reject alerts",
                icon: "shield.checkered",
                iconColor: ConvergioTokens.Brand.gialloFerrari,
                binding: thorBinding
            )
            categoryToggle(
                label: "Task completion alerts",
                icon: "checkmark.circle",
                iconColor: ConvergioTokens.Brand.verdeRacing,
                binding: taskBinding
            )
            categoryToggle(
                label: "Mesh peer alerts",
                icon: "point.3.connected.trianglepath.dotted",
                iconColor: ConvergioTokens.Brand.azzurro,
                binding: meshBinding
            )
            categoryToggle(
                label: "Plan state alerts",
                icon: "list.bullet.rectangle",
                iconColor: ConvergioTokens.Roles.role3,
                binding: planBinding
            )
        }
    }

    private func categoryToggle(
        label: String,
        icon: String,
        iconColor: Color,
        binding: Binding<Bool>
    ) -> some View {
        Toggle(isOn: binding) {
            Label {
                Text(label)
            } icon: {
                Image(systemName: icon)
                    .foregroundStyle(iconColor)
            }
        }
        .tint(ConvergioTokens.Brand.gialloFerrari)
    }

    private var testButton: some View {
        Button("Test Notification") {
            Task { await manager.sendTestNotification() }
        }
        .buttonStyle(AccentButtonStyle())
    }

    private var deliveredRow: some View {
        Text("Delivered this session: \(manager.deliveredCount)")
            .font(.caption)
            .foregroundStyle(.secondary)
    }

    // MARK: - Bindings

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
