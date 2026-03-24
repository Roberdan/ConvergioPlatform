import Foundation
import Observation
import Security

@MainActor
@Observable
final class OnboardingModel {
    var daemonURL: String = "http://localhost:8420"
    var generatedToken: String = OnboardingModel.makeToken()
    var isVerifying = false
    var errorMessage: String?
    var helperText = "Export CONVERGIO_AUTH_TOKEN on the daemon before verifying."
    var verificationSucceeded = false

    func regenerateToken() {
        generatedToken = Self.makeToken()
        errorMessage = nil
        verificationSucceeded = false
    }

    func verifyConnection() async {
        isVerifying = true
        errorMessage = nil
        verificationSucceeded = false

        defer { isVerifying = false }

        guard let url = URL(string: daemonURL) else {
            errorMessage = "Enter a valid daemon URL."
            return
        }

        let client = DaemonClient(baseURL: url)
        do {
            _ = try await client.health()
            try client.storeToken(generatedToken)
            _ = try await client.heartbeatStatus()
            verificationSucceeded = true
            helperText = "Daemon verified. The auth token is now stored in Keychain."
        } catch {
            errorMessage = error.localizedDescription
            try? KeychainTokenStore.shared.deleteToken()
        }
    }

    private static func makeToken() -> String {
        var bytes = [UInt8](repeating: 0, count: 32)
        let status = SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes)
        guard status == errSecSuccess else {
            return UUID().uuidString.replacingOccurrences(of: "-", with: "")
        }
        return Data(bytes).map { String(format: "%02x", $0) }.joined()
    }
}
