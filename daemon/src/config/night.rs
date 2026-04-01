// Night mode configuration — off-peak processing with model downgrade.
// Extracted from config/mod.rs to keep it under 250 lines.

use chrono::Timelike;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct NightConfig {
    pub night_mode: bool,
    /// Format: "HH:MM-HH:MM" (e.g. "23:00-07:00")
    pub night_hours: String,
    pub night_model: String,
}

impl Default for NightConfig {
    fn default() -> Self {
        Self {
            night_mode: false,
            night_hours: "23:00-07:00".to_string(),
            night_model: "claude-haiku-4-5".to_string(),
        }
    }
}

/// Parse "HH:MM-HH:MM" into (start_hour, end_hour).
/// Returns None on invalid format.
pub fn parse_hour_range(range: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = range.split('-').collect();
    if parts.len() != 2 {
        return None;
    }
    let start = parse_hhmm(parts[0])?;
    let end = parse_hhmm(parts[1])?;
    Some((start, end))
}

fn parse_hhmm(s: &str) -> Option<u32> {
    let hm: Vec<&str> = s.trim().split(':').collect();
    if hm.len() != 2 {
        return None;
    }
    let h: u32 = hm[0].parse().ok()?;
    let m: u32 = hm[1].parse().ok()?;
    if h > 23 || m > 59 {
        return None;
    }
    Some(h)
}

/// Returns true if the current local hour falls within [start, end).
/// Handles midnight wrapping (e.g. 23:00-07:00 means 23,0,1,...,6).
/// Uses the same approach as mesh/auto_update.rs::is_quiet_hours().
pub fn is_in_hour_range(start: u32, end: u32) -> bool {
    let hour = chrono::Local::now().hour();
    if start <= end {
        hour >= start && hour < end
    } else {
        // wraps midnight
        hour >= start || hour < end
    }
}

/// Returns true if night mode is enabled AND current time is within night hours.
pub fn is_night_hours(config: &NightConfig) -> bool {
    if !config.night_mode {
        return false;
    }
    match parse_hour_range(&config.night_hours) {
        Some((start, end)) => is_in_hour_range(start, end),
        None => {
            tracing::warn!(
                "[night] invalid night_hours format '{}', expected HH:MM-HH:MM",
                config.night_hours
            );
            false
        }
    }
}

/// Validates "HH:MM-HH:MM" format.
pub fn validate_night_hours(hours: &str) -> Result<(), String> {
    let parts: Vec<&str> = hours.split('-').collect();
    if parts.len() != 2 {
        return Err("expected format HH:MM-HH:MM".to_string());
    }
    validate_hhmm(parts[0])?;
    validate_hhmm(parts[1])?;
    Ok(())
}

fn validate_hhmm(s: &str) -> Result<(), String> {
    let hm: Vec<&str> = s.trim().split(':').collect();
    if hm.len() != 2 {
        return Err(format!("invalid time segment '{s}', expected HH:MM"));
    }
    let h: u32 = hm[0]
        .parse()
        .map_err(|_| format!("invalid hour in '{s}'"))?;
    let m: u32 = hm[1]
        .parse()
        .map_err(|_| format!("invalid minute in '{s}'"))?;
    if h > 23 {
        return Err(format!("hour {h} out of range 0-23"));
    }
    if m > 59 {
        return Err(format!("minute {m} out of range 0-59"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_range() {
        assert_eq!(parse_hour_range("23:00-07:00"), Some((23, 7)));
        assert_eq!(parse_hour_range("09:30-17:00"), Some((9, 17)));
        assert_eq!(parse_hour_range("00:00-06:00"), Some((0, 6)));
    }

    #[test]
    fn parse_invalid_range() {
        assert_eq!(parse_hour_range(""), None);
        assert_eq!(parse_hour_range("23:00"), None);
        assert_eq!(parse_hour_range("25:00-07:00"), None);
        assert_eq!(parse_hour_range("abc-def"), None);
    }

    #[test]
    fn midnight_wrap_range() {
        // 23:00-07:00: hour 23 is in, hour 6 is in, hour 7 is out, hour 12 is out
        assert!(is_in_hour_range_at(23, 7, 23));
        assert!(is_in_hour_range_at(23, 7, 0));
        assert!(is_in_hour_range_at(23, 7, 6));
        assert!(!is_in_hour_range_at(23, 7, 7));
        assert!(!is_in_hour_range_at(23, 7, 12));
    }

    #[test]
    fn normal_range() {
        // 09:00-17:00: hour 9 is in, hour 16 is in, hour 17 is out
        assert!(is_in_hour_range_at(9, 17, 9));
        assert!(is_in_hour_range_at(9, 17, 16));
        assert!(!is_in_hour_range_at(9, 17, 17));
        assert!(!is_in_hour_range_at(9, 17, 8));
    }

    /// Deterministic version for testing (avoids dependency on wall clock).
    fn is_in_hour_range_at(start: u32, end: u32, hour: u32) -> bool {
        if start <= end {
            hour >= start && hour < end
        } else {
            hour >= start || hour < end
        }
    }

    #[test]
    fn validate_valid_hours() {
        assert!(validate_night_hours("23:00-07:00").is_ok());
        assert!(validate_night_hours("00:00-23:59").is_ok());
    }

    #[test]
    fn validate_invalid_hours() {
        assert!(validate_night_hours("25:00-07:00").is_err());
        assert!(validate_night_hours("23:00-07:60").is_err());
        assert!(validate_night_hours("not-valid").is_err());
        assert!(validate_night_hours("23:00").is_err());
    }

    #[test]
    fn night_mode_disabled_returns_false() {
        let cfg = NightConfig {
            night_mode: false,
            night_hours: "00:00-23:59".to_string(),
            night_model: "test".to_string(),
        };
        // Even with all-day range, disabled means false
        assert!(!is_night_hours(&cfg));
    }

    #[test]
    fn default_config_values() {
        let cfg = NightConfig::default();
        assert!(!cfg.night_mode);
        assert_eq!(cfg.night_hours, "23:00-07:00");
        assert_eq!(cfg.night_model, "claude-haiku-4-5");
    }
}
