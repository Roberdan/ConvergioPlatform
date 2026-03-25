// TDD tests for notify — written BEFORE implementation (RED phase).
// F-28: Phone notifications.

#[cfg(test)]
mod tests {
    use super::super::notify::{NotifySeverity, NotifyMessage};

    #[test]
    fn notify_severity_from_str_critical() {
        let s: NotifySeverity = "critical".parse().unwrap();
        assert!(matches!(s, NotifySeverity::Critical));
    }

    #[test]
    fn notify_severity_from_str_warning() {
        let s: NotifySeverity = "warning".parse().unwrap();
        assert!(matches!(s, NotifySeverity::Warning));
    }

    #[test]
    fn notify_severity_from_str_info() {
        let s: NotifySeverity = "info".parse().unwrap();
        assert!(matches!(s, NotifySeverity::Info));
    }

    #[test]
    fn notify_severity_from_str_invalid() {
        let result: Result<NotifySeverity, _> = "unknown".parse();
        assert!(result.is_err());
    }

    #[test]
    fn notify_message_builds() {
        let msg = NotifyMessage {
            title: "Agent blocked".to_string(),
            message: "T2-04 stalled for 5min".to_string(),
            severity: NotifySeverity::Warning,
        };
        assert_eq!(msg.title, "Agent blocked");
        assert!(matches!(msg.severity, NotifySeverity::Warning));
    }

    #[test]
    fn notify_severity_display() {
        assert_eq!(format!("{}", NotifySeverity::Critical), "critical");
        assert_eq!(format!("{}", NotifySeverity::Warning), "warning");
        assert_eq!(format!("{}", NotifySeverity::Info), "info");
    }

    #[test]
    fn notify_severity_priority_value_critical_highest() {
        assert!(NotifySeverity::Critical.priority() > NotifySeverity::Warning.priority());
        assert!(NotifySeverity::Warning.priority() > NotifySeverity::Info.priority());
    }
}
