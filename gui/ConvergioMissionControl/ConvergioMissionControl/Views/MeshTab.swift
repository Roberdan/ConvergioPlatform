// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Mesh tab: P2P topology and delegation status

import SwiftUI

/// Mesh tab — wraps MeshTopologyView which fetches from daemon REST API.
struct MeshTab: View {
    var body: some View {
        MeshTopologyView()
    }
}
