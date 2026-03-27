// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Agents tab: agent roster and live session catalog

import SwiftUI

/// Agents tab — wraps AgentCatalogView which fetches directly from the daemon REST API.
struct AgentsTab: View {
    var body: some View {
        AgentCatalogView()
    }
}
