// Tests for mesh::auto_update

#[cfg(test)]
mod tests {
    use crate::mesh::auto_update::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn test_quiet_hours() {
        let hour = chrono::Local::now().hour();
        let expected = hour >= 23 || hour < 7;
        assert_eq!(is_quiet_hours(), expected);
    }

    use chrono::Timelike;

    #[test]
    fn test_rate_limit_fresh() {
        // With LAST_UPDATE at 0, should not be rate-limited
        LAST_UPDATE.store(0, Ordering::Relaxed);
        assert!(!is_rate_limited());
    }

    #[test]
    fn test_rate_limit_recent() {
        // Set LAST_UPDATE to now — should be rate-limited
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        LAST_UPDATE.store(now, Ordering::Relaxed);
        assert!(is_rate_limited());
        // Cleanup
        LAST_UPDATE.store(0, Ordering::Relaxed);
    }

    #[test]
    fn test_rate_limit_old() {
        // Set LAST_UPDATE to 1 hour ago — should NOT be rate-limited
        let old = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 3600;
        LAST_UPDATE.store(old, Ordering::Relaxed);
        assert!(!is_rate_limited());
        LAST_UPDATE.store(0, Ordering::Relaxed);
    }

    #[test]
    fn test_version_comparison() {
        // Verify semver ordering via the same logic used in api_mesh_update
        fn cmp_ver(a: &str, b: &str) -> std::cmp::Ordering {
            let parse = |s: &str| -> Vec<u64> {
                s.split('.')
                    .map(|p| p.parse::<u64>().unwrap_or(0))
                    .collect()
            };
            parse(a).cmp(&parse(b))
        }
        assert_eq!(cmp_ver("20.3.0", "20.4.0"), std::cmp::Ordering::Less);
        assert_eq!(cmp_ver("20.4.0", "20.4.0"), std::cmp::Ordering::Equal);
        assert_eq!(cmp_ver("20.4.1", "20.4.0"), std::cmp::Ordering::Greater);
        assert_eq!(cmp_ver("1.0.0", "2.0.0"), std::cmp::Ordering::Less);
    }

    #[test]
    fn test_run_cmd_success() {
        let result = run_cmd("echo", &["hello"]);
        assert_eq!(result.unwrap(), "hello");
    }

    #[test]
    fn test_run_cmd_failure() {
        let result = run_cmd("false", &[]);
        assert!(result.is_err());
    }
}
