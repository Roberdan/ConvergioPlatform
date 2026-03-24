import SwiftUI

// MARK: - Success badge with green ring expansion animation

struct OnboardingSuccessBadge: View {
    let helperText: String
    @State private var ringScale: CGFloat = 0.4

    var body: some View {
        HStack(spacing: Spacing.xs) {
            ZStack {
                Circle()
                    .stroke(ConvergioTokens.Brand.verdeRacing.opacity(0.3), lineWidth: 3)
                    .frame(width: 28, height: 28)
                    .scaleEffect(ringScale)
                Image(systemName: "checkmark.circle.fill")
                    .foregroundStyle(ConvergioTokens.Brand.verdeRacing)
                    .font(.title3)
            }
            .accessibilityLabel("Status: Verification successful")
            Text(helperText)
                .foregroundStyle(ConvergioTokens.Brand.verdeRacing)
                .font(.bodyMedium)
        }
        .onAppear {
            withAnimation(.spring(response: 0.5, dampingFraction: 0.5)) {
                ringScale = 1.4
            }
        }
    }
}

// MARK: - Gradient connect button style

struct OnboardingConnectButtonStyle: ButtonStyle {
    @State private var isHovered = false

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.headline)
            .foregroundStyle(ConvergioTokens.Surface.surface)
            .padding(.horizontal, Spacing.xl)
            .padding(.vertical, Spacing.sm + 2)
            .background(
                LinearGradient(
                    colors: [
                        ConvergioTokens.Brand.gialloFerrari,
                        Color(hex: 0xE6A800),
                    ],
                    startPoint: .topLeading,
                    endPoint: .bottomTrailing
                ),
                in: RoundedRectangle(cornerRadius: CornerRadius.sm, style: .continuous)
            )
            .shadow(
                color: ConvergioTokens.Brand.gialloFerrari.opacity(isHovered ? 0.5 : 0.3),
                radius: 12,
                y: 4
            )
            .scaleEffect(configuration.isPressed ? 0.97 : (isHovered ? 1.02 : 1.0))
            .animation(.spring(response: 0.2, dampingFraction: 0.7), value: configuration.isPressed)
            .animation(.easeInOut(duration: 0.15), value: isHovered)
            .onHover { hovering in isHovered = hovering }
    }
}
