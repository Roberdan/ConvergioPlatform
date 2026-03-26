// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — Wave Codable model

import Foundation

/// A plan wave (execution phase) as returned by the daemon API.
struct Wave: Codable, Identifiable {
    let id: Int
    let waveId: String
    let name: String
    let status: String
    let planId: Int?

    enum CodingKeys: String, CodingKey {
        case id
        case waveId = "wave_id"
        case name, status
        case planId = "plan_id"
    }
}
