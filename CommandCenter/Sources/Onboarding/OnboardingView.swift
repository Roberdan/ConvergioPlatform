import Observation
import SwiftUI

struct OnboardingView: View {
    @Bindable var model: OnboardingModel
    let onComplete: (String) -> Void

    var body: some View {
        ZStack {
            LinearGradient(
                colors: [.indigo.opacity(0.18), .cyan.opacity(0.12), .clear],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )
            .ignoresSafeArea()

            card
                .frame(maxWidth: 760)
                .padding(40)
        }
        .background(Color(nsColor: .windowBackgroundColor))
    }

    private var card: some View {
        Group {
            if #available(macOS 26.0, *) {
                cardContent
                    .glassEffect()
            } else {
                cardContent
                    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 32, style: .continuous))
            }
        }
    }

    private var cardContent: some View {
        VStack(alignment: .leading, spacing: 24) {
            Text("Welcome to Command Center")
                .font(.system(size: 32, weight: .bold, design: .rounded))

            Text(
                "Generate a daemon token, set CONVERGIO_AUTH_TOKEN on the server, "
                    + "then verify the connection before entering the native UI."
            )
            .foregroundStyle(.secondary)

            steps

            VStack(alignment: .leading, spacing: 10) {
                Text("Generated token")
                    .font(.headline)
                Text(model.generatedToken)
                    .font(.system(.body, design: .monospaced))
                    .textSelection(.enabled)
                Button("Generate another token") {
                    model.regenerateToken()
                }
            }

            VStack(alignment: .leading, spacing: 10) {
                Text("Daemon URL")
                    .font(.headline)
                TextField("http://localhost:8420", text: $model.daemonURL)
                    .textFieldStyle(.roundedBorder)
            }

            if let errorMessage = model.errorMessage {
                Text(errorMessage)
                    .foregroundStyle(.red)
            } else {
                Text(model.helperText)
                    .foregroundStyle(model.verificationSucceeded ? .green : .secondary)
            }

            HStack {
                Spacer()
                Button(model.isVerifying ? "Verifying..." : "Verify and continue") {
                    Task {
                        await model.verifyConnection()
                        if model.verificationSucceeded {
                            onComplete(model.daemonURL)
                        }
                    }
                }
                .buttonStyle(.borderedProminent)
                .disabled(model.isVerifying)
            }
        }
        .padding(32)
    }

    private var steps: some View {
        VStack(alignment: .leading, spacing: 14) {
            step(index: 1, body: "Copy the generated 32-byte token.")
            step(index: 2, body: "Set CONVERGIO_AUTH_TOKEN on the daemon host.")
            step(index: 3, body: "Confirm the daemon URL and run verification.")
            step(index: 4, body: "Token is stored in Keychain and onboarding dismisses.")
        }
    }

    private func step(index: Int, body: String) -> some View {
        HStack(alignment: .top, spacing: 12) {
            Text("\(index)")
                .font(.headline)
                .frame(width: 28, height: 28)
                .background(.blue.opacity(0.14), in: Circle())
            Text(body)
        }
    }
}
