// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// TDD tests for kernel/monitor_peer_tracker.rs.

#[cfg(test)]
mod tests {
    use crate::kernel::monitor_peer_tracker::PeerFailureTracker;

    #[test]
    fn no_alert_below_threshold() {
        let mut tracker = PeerFailureTracker::new();
        // Two consecutive failures — threshold is 3.
        assert!(tracker.record_result("http://peer:8420", false).is_none());
        assert!(tracker.record_result("http://peer:8420", false).is_none());
    }

    #[test]
    fn alert_at_threshold() {
        let mut tracker = PeerFailureTracker::new();
        tracker.record_result("http://peer-1:8420", false);
        tracker.record_result("http://peer-1:8420", false);
        let alert = tracker.record_result("http://peer-1:8420", false);
        assert_eq!(alert, Some("peer-1".to_string()), "should alert at 3 consecutive failures");
    }

    #[test]
    fn resets_on_success() {
        let mut tracker = PeerFailureTracker::new();
        tracker.record_result("http://peer-2:8420", false);
        tracker.record_result("http://peer-2:8420", false);
        tracker.record_result("http://peer-2:8420", true); // success resets
        assert!(tracker.record_result("http://peer-2:8420", false).is_none());
        assert!(tracker.record_result("http://peer-2:8420", false).is_none());
    }

    #[test]
    fn manual_reset_clears_count() {
        let mut tracker = PeerFailureTracker::new();
        tracker.record_result("http://peer-3:8420", false);
        tracker.record_result("http://peer-3:8420", false);
        tracker.reset("http://peer-3:8420");
        assert!(tracker.record_result("http://peer-3:8420", false).is_none());
        assert!(tracker.record_result("http://peer-3:8420", false).is_none());
    }

    #[test]
    fn tracks_multiple_peers_independently() {
        let mut tracker = PeerFailureTracker::new();
        tracker.record_result("http://peer-a:8420", false);
        tracker.record_result("http://peer-a:8420", false);
        let alert_a = tracker.record_result("http://peer-a:8420", false);
        assert_eq!(alert_a, Some("peer-a".to_string()));
        // Peer B: only 2 failures — should not alert.
        tracker.record_result("http://peer-b:8420", false);
        let no_alert = tracker.record_result("http://peer-b:8420", false);
        assert!(no_alert.is_none(), "peer-b at 2 failures should not alert");
    }

    #[test]
    fn no_double_alert_after_threshold() {
        let mut tracker = PeerFailureTracker::new();
        tracker.record_result("http://peer:8420", false);
        tracker.record_result("http://peer:8420", false);
        tracker.record_result("http://peer:8420", false); // alerts
        // 4th failure should NOT re-alert (threshold fires exactly once).
        let second = tracker.record_result("http://peer:8420", false);
        assert!(second.is_none(), "should not re-alert after threshold already fired");
    }
}
