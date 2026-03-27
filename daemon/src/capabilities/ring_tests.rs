use super::ring::Ring;

#[test]
fn ring_ordering() {
    assert!(Ring::Core < Ring::Trusted);
    assert!(Ring::Trusted < Ring::Community);
    assert!(Ring::Community < Ring::Sandboxed);
}

#[test]
fn ring_from_u8_known_values() {
    assert_eq!(Ring::from_u8(0), Ring::Core);
    assert_eq!(Ring::from_u8(1), Ring::Trusted);
    assert_eq!(Ring::from_u8(2), Ring::Community);
    assert_eq!(Ring::from_u8(3), Ring::Sandboxed);
}

#[test]
fn ring_from_u8_unknown_defaults_to_sandboxed() {
    assert_eq!(Ring::from_u8(4), Ring::Sandboxed);
    assert_eq!(Ring::from_u8(255), Ring::Sandboxed);
}

#[test]
fn ring_access_control() {
    // Core can access everything
    assert!(Ring::Core.can_access(Ring::Core));
    assert!(Ring::Core.can_access(Ring::Sandboxed));

    // Sandboxed can only access Sandboxed
    assert!(Ring::Sandboxed.can_access(Ring::Sandboxed));
    assert!(!Ring::Sandboxed.can_access(Ring::Core));
    assert!(!Ring::Sandboxed.can_access(Ring::Trusted));

    // Trusted can access Trusted, Community, Sandboxed
    assert!(Ring::Trusted.can_access(Ring::Trusted));
    assert!(Ring::Trusted.can_access(Ring::Community));
    assert!(!Ring::Trusted.can_access(Ring::Core));
}

#[test]
fn ring_display() {
    assert_eq!(format!("{}", Ring::Core), "Ring 0 (Core)");
    assert_eq!(format!("{}", Ring::Sandboxed), "Ring 3 (Sandboxed)");
}

#[test]
fn ring_roundtrip() {
    for val in 0..=3 {
        let ring = Ring::from_u8(val);
        assert_eq!(ring.as_u8(), val);
    }
}
