import AppKit
import Observation
import SwiftUI

struct OnboardingView: View {
    @Bindable var model: OnboardingModel
    let onComplete: (String) -> Void

    var body: some View {
        ZStack(alignment: .top) {
            ConvergioTokens.Surface.surface
                .ignoresSafeArea()

            // Warm radial glow behind the hero area
            RadialGradient(
                colors: [ConvergioTokens.Brand.gialloFerrari.opacity(0.2), .clear],
                center: .top,
                startRadius: 20,
                endRadius: 400
            )
            .frame(height: 320)
            .ignoresSafeArea()

            cardContent
                .modifier(ConvergioCardModifier())
                .overlay(
                    RoundedRectangle(cornerRadius: CornerRadius.lg, style: .continuous)
                        .strokeBorder(ConvergioTokens.Brand.gialloFerrari.opacity(0.12), lineWidth: 1)
                )
                .shadow(color: ConvergioTokens.Brand.gialloFerrari.opacity(0.08), radius: 24, y: 8)
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

    // MARK: - Header with golden glow icon

    private var header: some View {
        HStack(spacing: Spacing.md) {
            Image(systemName: "bolt.shield.fill")
                .font(.system(size: 52, weight: .regular))
                .foregroundStyle(ConvergioTokens.Brand.gialloFerrari)
                .shadow(color: ConvergioTokens.Brand.gialloFerrari.opacity(0.6), radius: 16)
                .shadow(color: ConvergioTokens.Brand.gialloFerrari.opacity(0.3), radius: 28)
                .accessibilityLabel("Command Center shield icon")

            VStack(alignment: .leading, spacing: Spacing.xxs) {
                Text("Welcome to Command Center")
                    .font(.heroTitle)
                    .foregroundStyle(ConvergioTokens.Text.textPrimary)

                Text(
                    "Generate a daemon token, set CONVERGIO_AUTH_TOKEN on the server, "
                    + "then verify the connection."
                )
                .font(.bodyMedium)
                .foregroundStyle(ConvergioTokens.Text.textMuted)
            }
        }
    }

    // MARK: - Step indicator with progress line

    private var stepIndicator: some View {
        let stepLabels = [
            "Copy the generated 32-byte token.",
            "Set CONVERGIO_AUTH_TOKEN on the daemon host.",
            "Confirm the daemon URL and run verification.",
            "Token stored in Keychain — onboarding dismisses.",
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
                .fill(isCompleted ? ConvergioTokens.Brand.gialloFerrari : .clear)
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

    // MARK: - Token section (code-style card)

    private var tokenSection: some View {
        VStack(alignment: .leading, spacing: Spacing.xs) {
            Text("Generated token")
                .font(.cardTitle)
                .foregroundStyle(ConvergioTokens.Text.textPrimary)

            HStack {
                Text(model.generatedToken)
                    .font(.mono)
                    .foregroundStyle(ConvergioTokens.Text.textPrimary)
                    .textSelection(.enabled)

                Spacer()

                Button {
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(model.generatedToken, forType: .string)
                } label: {
                    Image(systemName: "doc.on.doc")
                        .font(.caption)
                        .foregroundStyle(ConvergioTokens.Text.textMuted)
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Copy token to clipboard")
            }
            .padding(Spacing.sm)
            .background(
                ConvergioTokens.Surface.surfaceSunken,
                in: RoundedRectangle(cornerRadius: CornerRadius.sm, style: .continuous)
            )
            .overlay(
                RoundedRectangle(cornerRadius: CornerRadius.sm, style: .continuous)
                    .strokeBorder(ConvergioTokens.Border.border, lineWidth: 1)
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

    // MARK: - Feedback row with success animation

    @ViewBuilder
    private var feedbackRow: some View {
        if model.verificationSucceeded {
            OnboardingSuccessBadge(helperText: model.helperText)
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

    // MARK: - Connect button (gradient, larger)

    private var connectButton: some View {
        HStack {
            Spacer()
            Button(model.isVerifying ? "Verifying..." : "Connect") {
                Task {
                    await model.verifyConnection()
                    if model.verificationSucceeded { onComplete(model.daemonURL) }
                }
            }
            .buttonStyle(OnboardingConnectButtonStyle())
            .disabled(model.isVerifying)
            .accessibilityLabel(
                model.isVerifying ? "Verifying connection" : "Verify daemon connection"
            )
        }
    }
}
