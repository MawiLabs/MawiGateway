//! Shared utility functions
//!
//! Common helpers used across the core crate.

use chrono::{Datelike, TimeZone};

/// Calculate timestamp for the first day of next month (UTC)
pub fn next_month_timestamp() -> i64 {
    let now = chrono::Utc::now();
    let next_month = if now.month() == 12 {
        chrono::Utc
            .with_ymd_and_hms(now.year() + 1, 1, 1, 0, 0, 0)
            .single()
            .expect("Invalid date calculation for next year January")
    } else {
        chrono::Utc
            .with_ymd_and_hms(now.year(), now.month() + 1, 1, 0, 0, 0)
            .single()
            .expect("Invalid date calculation for next month")
    };
    next_month.timestamp()
}

/// Mask an API key for safe display
/// Shows only the last 4 characters, or "****" if key is too short
pub fn mask_api_key(key: &str) -> String {
    if key.len() <= 4 {
        "****".to_string()
    } else {
        let visible_part = &key[key.len() - 4..];
        format!("****{}", visible_part)
    }
}

/// Calculate percentage safely, avoiding division by zero
pub fn safe_percentage(used: f64, total: f64) -> u8 {
    if total <= 0.0 {
        0
    } else {
        ((used / total) * 100.0).min(100.0).max(0.0) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_api_key() {
        assert_eq!(mask_api_key("sk-1234567890abcdef"), "****cdef");
        assert_eq!(mask_api_key("abc"), "****");
        assert_eq!(mask_api_key(""), "****");
    }

    #[test]
    fn test_safe_percentage() {
        assert_eq!(safe_percentage(50.0, 100.0), 50);
        assert_eq!(safe_percentage(0.0, 0.0), 0);
        assert_eq!(safe_percentage(150.0, 100.0), 100);
        assert_eq!(safe_percentage(-10.0, 100.0), 0);
    }

    #[test]
    fn test_next_month_timestamp() {
        let ts = next_month_timestamp();
        let now = chrono::Utc::now().timestamp();
        assert!(ts > now);
    }
}
