import Observation
import SwiftUI

struct OnboardingView: View {
    @Bindable var model: OnboardingModel
    let onComplete: (String) -> Void

    var body: some View {
        ZStack(alignment: .top) {
            // Brand background
            ConvergioTokens.Surface.surface
                .ignoresSafeArea()

            // Subtle giallo-ferrari gradient accent at the top
            LinearGradient(
                colors: [ConvergioTokens.Brand.gialloFerrari.opacity(0.18), .clear],
                startPoint: .top,
                endPoint: .bottom
            )
            .frame(height: 200)
            .ignoresSafeArea()

            cardContent
                .modifier(ConvergioCardModifier())
                .frame(maxWidth: 720)
                .padding(Spacing.xxl)
        }
    }

    // MARK: - Card content

    private var cardContent: some View {
        VStack(alignment: .leading, spacing: Spacing.lg) {
            header
            stepIndicator
            tokenSection
            urlSection
            feedbackRow
            connectButton
        }
    }

    // MARK: - Header

    private var header: some View {
        HStack(spacing: Spacing.md) {
            Image(systemName: "bolt.shield.fill")
                .font(.system(size: 44, weight: .regular))
                .foregroundStyle(ConvergioTokens.Brand.gialloFerrari)
                .accessibilityLabel("Command Center shield icon")

            VStack(alignment: .leading, spacing: Spacing.xxs) {
                Text("Welcome to Command Center")
                    .font(.heroTitle)
                    .foregroundStyle(ConvergioTokens.Text.textPrimary)

                Text(
                    "Generate a daemon token, set CONVERGIO_AUTH_TOKEN on the server, "
                        + "then verify the connection before entering the native UI."
                )
                .font(.bodyMedium)
                .foregroundStyle(ConvergioTokens.Text.textMuted)
            }
        }
    }

    // MARK: - Step indicator

    private var stepIndicator: some View {
        let stepLabels = [
            "Copy the generated 32-byte token.",
            "Set CONVERGIO_AUTH_TOKEN on the daemon host.",
            "Confirm the daemon URL and run verification.",
            "Token is stored in Keychain and onboarding dismisses.",
        ]
        let currentStep = model.verificationSucceeded ? 5 :
            (model.isVerifying ? 3 : (model.daemonURL.isEmpty ? 1 : 2))

        return HStack(alignment: .top, spacing: 0) {
            ForEach(Array(stepLabels.enumerated()), id: \.offset) { index, label in
                let stepNum = index + 1
                let isCompleted = stepNum < currentStep
                let isCurrent = stepNum == currentStep

                VStack(spacing: Spacing.xs) {
                    HStack(spacing: 0) {
                        if index > 0 {
                            Rectangle()
                                .fill(isCompleted
                                      ? ConvergioTokens.Brand.gialloFerrari
                                      : ConvergioTokens.Border.borderSubtle)
                                .frame(maxWidth: .infinity, maxHeight: 2)
                        }
                        stepCircle(number: stepNum, isCompleted: isCompleted, isCurrent: isCurrent)
                        if index < stepLabels.count - 1 {
                            Rectangle()
                                .fill(isCompleted
                                      ? ConvergioTokens.Brand.gialloFerrari
                                      : ConvergioTokens.Border.borderSubtle)
                                .frame(maxWidth: .infinity, maxHeight: 2)
                        }
                    }
                    Text(label)
                        .font(.label)
                        .foregroundStyle(isCurrent
                                         ? ConvergioTokens.Text.textPrimary
                                         : ConvergioTokens.Text.textMuted)
                        .multilineTextAlignment(.center)
                        .frame(maxWidth: 140)
                }
            }
        }
    }

    @ViewBuilder
    private func stepCircle(number: Int, isCompleted: Bool, isCurrent: Bool) -> some View {
        ZStack {
            Circle()
                .fill(isCompleted
                      ? ConvergioTokens.Brand.gialloFerrari
                      : .clear)
                .overlay(
                    Circle()
                        .strokeBorder(
                            isCurrent
                                ? ConvergioTokens.Brand.gialloFerrari
                                : ConvergioTokens.Border.borderSubtle,
                            lineWidth: 2
                        )
                )
                .frame(width: 28, height: 28)

            if isCompleted {
                Image(systemName: "checkmark")
                    .font(.system(size: 12, weight: .bold))
                    .foregroundStyle(ConvergioTokens.Surface.surface)
            } else {
                Text("\(number)")
                    .font(.label)
                    .foregroundStyle(isCurrent
                                     ? ConvergioTokens.Brand.gialloFerrari
                                     : ConvergioTokens.Text.textMuted)
            }
        }
        .accessibilityLabel("Step \(number): \(isCompleted ? "completed" : isCurrent ? "current" : "upcoming")")
    }

    // MARK: - Token section

    private var tokenSection: some View {
        VStack(alignment: .leading, spacing: Spacing.xs) {
            Text("Generated token")
                .font(.cardTitle)
                .foregroundStyle(ConvergioTokens.Text.textPrimary)

            Text(model.generatedToken)
                .font(.mono)
                .foregroundStyle(ConvergioTokens.Text.textPrimary)
                .textSelection(.enabled)
                .padding(Spacing.sm)
                .background(ConvergioTokens.Surface.surfaceRaised,
                            in: RoundedRectangle(cornerRadius: CornerRadius.sm, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: CornerRadius.sm, style: .continuous)
                        .strokeBorder(ConvergioTokens.Border.borderSubtle, lineWidth: 1)
                )
                .accessibilityLabel("Generated daemon authentication token")

            Button("Generate another token") { model.regenerateToken() }
                .font(.label)
                .foregroundStyle(ConvergioTokens.Brand.gialloFerrari)
                .buttonStyle(.plain)
                .accessibilityLabel("Generate a new daemon authentication token")
        }
    }

    // MARK: - URL section

    private var urlSection: some View {
        VStack(alignment: .leading, spacing: Spacing.xs) {
            Text("Daemon URL")
                .font(.cardTitle)
                .foregroundStyle(ConvergioTokens.Text.textPrimary)
            TextField("http://localhost:8420", text: $model.daemonURL)
                .textFieldStyle(.roundedBorder)
                .accessibilityLabel("Daemon URL")
        }
    }

    // MARK: - Feedback row

    @ViewBuilder
    private var feedbackRow: some View {
        if model.verificationSucceeded {
            HStack(spacing: Spacing.xs) {
                Image(systemName: "checkmark.circle.fill")
                    .foregroundStyle(ConvergioTokens.Brand.verdeRacing)
                    .font(.title3)
                    .scaleEffect(model.verificationSucceeded ? 1.0 : 0.1)
                    .animation(.spring(response: 0.4, dampingFraction: 0.55), value: model.verificationSucceeded)
                    .accessibilityLabel("Status: Verification successful")
                Text(model.helperText)
                    .foregroundStyle(ConvergioTokens.Brand.verdeRacing)
                    .font(.bodyMedium)
            }
        } else if let err = model.errorMessage {
            Text(err)
                .foregroundStyle(ConvergioTokens.Status.error)
                .font(.bodyMedium)
                .accessibilityLabel("Status: Error — \(err)")
        } else {
            Text(model.helperText)
                .foregroundStyle(ConvergioTokens.Text.textMuted)
                .font(.bodyMedium)
                .accessibilityLabel("Status: \(model.helperText)")
        }
    }

    // MARK: - Connect button

    private var connectButton: some View {
        HStack {
            Spacer()
            Button(model.isVerifying ? "Verifying..." : "Connect") {
                Task {
                    await model.verifyConnection()
                    if model.verificationSucceeded {
                        onComplete(model.daemonURL)
                    }
                }
            }
            .buttonStyle(AccentButtonStyle())
            .disabled(model.isVerifying)
            .accessibilityLabel(model.isVerifying ? "Verifying connection, please wait" : "Verify daemon connection and continue")
        }
    }
}
