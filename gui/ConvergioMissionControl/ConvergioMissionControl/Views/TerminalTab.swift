// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Terminal tab: embedded PTY via WebSocket

import SwiftUI

/// Terminal tab — wraps EmbeddedTerminalView with PTY WebSocket.
struct TerminalTab: View {
    var body: some View {
        EmbeddedTerminalView()
    }
}
