// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// PeerFailureTracker — tracks consecutive remote-node health failures.
// Extracted from monitor.rs to stay under the 250-line file limit.

use std::collections::HashMap;

use super::monitor::peer_name_from_url;

/// Tracks consecutive health-check failures per peer URL.
/// Alerts (via `record_result` return value) when a peer reaches the failure threshold.
pub struct PeerFailureTracker {
    /// Map of peer_url → consecutive failure count.
    counts: HashMap<String, u32>,
}

/// Number of consecutive failures before an alert is emitted.
const ALERT_THRESHOLD: u32 = 3;

impl PeerFailureTracker {
    pub fn new() -> Self {
        Self { counts: HashMap::new() }
    }

    /// Record a health-check result for `peer_url`.
    ///
    /// - `ok = true` → reset the counter for this peer, return `None`.
    /// - `ok = false` → increment counter; return `Some(peer_name)` when threshold is reached.
    ///
    /// Only returns `Some` exactly when the counter hits the threshold (not every subsequent call).
    pub fn record_result(&mut self, peer_url: &str, ok: bool) -> Option<String> {
        if ok {
            self.counts.remove(peer_url);
            return None;
        }
        let count = self.counts.entry(peer_url.to_string()).or_insert(0);
        *count += 1;
        if *count == ALERT_THRESHOLD {
            Some(peer_name_from_url(peer_url))
        } else {
            None
        }
    }

    /// Manual reset for a peer (e.g., after the operator acknowledges an alert).
    pub fn reset(&mut self, peer_url: &str) {
        self.counts.remove(peer_url);
    }
}

impl Default for PeerFailureTracker {
    fn default() -> Self {
        Self::new()
    }
}
