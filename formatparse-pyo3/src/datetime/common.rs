use once_cell::sync::Lazy;
use pyo3::prelude::*;
use regex::Regex;
use std::collections::HashMap;

// Cached regex patterns for timezone and time parsing
pub(crate) static RE_TZ_COLON: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([+-])(\d{1,2}):?(\d{2})").unwrap());

pub(crate) static RE_TZ_4DIGIT: Lazy<Regex> = Lazy::new(|| Regex::new(r"([+-])(\d{4})").unwrap());

pub(crate) static RE_TZ_IN_STRING: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\s+([+-]\d{2}:?\d{2})$").unwrap());

pub(crate) static RE_TZ_IN_STRING_EXTENDED: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\s+([+-]\d{2}:?\d{2,4})$").unwrap());

pub(crate) static RE_TIME_24H: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(\d{1,2}):(\d{2})(?::(\d{2}))?").unwrap());

/// Month name to number mapping (abbreviated and full names)
pub fn get_month_map() -> HashMap<&'static str, u8> {
    [
        ("Jan", 1),
        ("Feb", 2),
        ("Mar", 3),
        ("Apr", 4),
        ("May", 5),
        ("Jun", 6),
        ("Jul", 7),
        ("Aug", 8),
        ("Sep", 9),
        ("Oct", 10),
        ("Nov", 11),
        ("Dec", 12),
        ("January", 1),
        ("February", 2),
        ("March", 3),
        ("April", 4),
        ("June", 6),
        ("July", 7),
        ("August", 8),
        ("September", 9),
        ("October", 10),
        ("November", 11),
        ("December", 12),
    ]
    .iter()
    .cloned()
    .collect()
}

/// Month name to number mapping (abbreviated only)
pub fn get_abbreviated_month_map() -> HashMap<&'static str, u8> {
    [
        ("Jan", 1),
        ("Feb", 2),
        ("Mar", 3),
        ("Apr", 4),
        ("May", 5),
        ("Jun", 6),
        ("Jul", 7),
        ("Aug", 8),
        ("Sep", 9),
        ("Oct", 10),
        ("Nov", 11),
        ("Dec", 12),
    ]
    .iter()
    .cloned()
    .collect()
}

/// Create a FixedTzOffset from offset minutes
pub fn create_fixed_tz(py: Python, offset_minutes: i32, name: &str) -> PyResult<PyObject> {
    let fixed_tz_module = py.import("formatparse")?;
    let fixed_tz_class = fixed_tz_module.getattr("FixedTzOffset")?;
    let tz = fixed_tz_class.call1((offset_minutes, name.to_string()))?;
    Ok(tz.to_object(py))
}

/// Parse timezone string into FixedTzOffset
/// Handles formats: +1:00, +10:00, +10:30, +1000, etc.
pub fn parse_timezone(py: Python, tz_str: &str) -> PyResult<PyObject> {
    // Handle formats: +1:00, +10:00, +10:30, +1000, etc.
    if let Some(caps) = RE_TZ_COLON.captures(tz_str) {
        if let (Some(sign_match), Some(hour_match), Some(min_match)) =
            (caps.get(1), caps.get(2), caps.get(3))
        {
            let sign = if sign_match.as_str() == "+" { 1 } else { -1 };
            let hour: i32 = hour_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone"))?;
            let min: i32 = min_match.as_str().parse().unwrap_or(0);
            let offset_minutes = sign * (hour * 60 + min);
            return create_fixed_tz(py, offset_minutes, "");
        }
    }
    // Also handle 4-digit format: +1000 (1 hour, 00 minutes)
    if let Some(caps) = RE_TZ_4DIGIT.captures(tz_str) {
        if let (Some(sign_match), Some(tz_match)) = (caps.get(1), caps.get(2)) {
            let sign = if sign_match.as_str() == "+" { 1 } else { -1 };
            let tz_str = tz_match.as_str();
            // Regex ensures exactly 4 digits, but add defensive check
            if tz_str.len() >= 4 {
                let hour: i32 = tz_str[..2].parse().map_err(|_| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone")
                })?;
                let min: i32 = tz_str[2..4].parse().map_err(|_| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone")
                })?;
                let offset_minutes = sign * (hour * 60 + min);
                return create_fixed_tz(py, offset_minutes, "");
            }
        }
    }
    Ok(py.None())
}

/// Parse time string with optional AM/PM indicator
/// Returns (hour, minute, second) in 24-hour format
pub fn parse_time_with_ampm(time_str: &str) -> Result<(u8, u8, u8), PyErr> {
    let mut hour = 0u8;
    let mut minute = 0u8;
    let mut second = 0u8;

    if let Some(ampm_idx) = time_str.to_uppercase().find("AM") {
        let time_part = &time_str[..ampm_idx].trim();
        if let Some(caps) = RE_TIME_24H.captures(time_part) {
            hour = caps
                .get(1)
                .unwrap()
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
            minute =
                caps.get(2).unwrap().as_str().parse().map_err(|_| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute")
                })?;
            second = caps
                .get(3)
                .map(|m| m.as_str().parse().unwrap_or(0))
                .unwrap_or(0);
            // 12 AM becomes 0 (midnight), other AM hours stay as-is
            if hour == 12 {
                hour = 0;
            }
        }
    } else if let Some(pm_idx) = time_str.to_uppercase().find("PM") {
        let time_part = &time_str[..pm_idx].trim();
        if let Some(caps) = RE_TIME_24H.captures(time_part) {
            hour = caps
                .get(1)
                .unwrap()
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
            minute =
                caps.get(2).unwrap().as_str().parse().map_err(|_| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute")
                })?;
            second = caps
                .get(3)
                .map(|m| m.as_str().parse().unwrap_or(0))
                .unwrap_or(0);
            // 12 PM stays as 12 (noon), other PM hours add 12
            if hour != 12 {
                hour += 12;
            }
        }
    } else {
        // 24-hour format
        if let Some(caps) = RE_TIME_24H.captures(time_str) {
            hour = caps
                .get(1)
                .unwrap()
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
            minute =
                caps.get(2).unwrap().as_str().parse().map_err(|_| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute")
                })?;
            second = caps
                .get(3)
                .map(|m| m.as_str().parse().unwrap_or(0))
                .unwrap_or(0);
        }
    }

    Ok((hour, minute, second))
}

/// Parse and pad microseconds string to 6 digits
/// Truncates if longer than 6 digits, pads with zeros on the right if shorter
pub fn parse_microseconds(micros_str: &str) -> Result<u32, PyErr> {
    let micros_str = if micros_str.len() > 6 {
        &micros_str[..6]
    } else {
        micros_str
    };
    let padded = format!("{:0<6}", micros_str);
    padded.parse().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Invalid microsecond: {}",
            micros_str
        ))
    })
}

/// Extract microseconds from a capture group, handling padding
pub fn extract_microseconds(cap: Option<regex::Match>) -> u32 {
    cap.map(|m| parse_microseconds(m.as_str()).unwrap_or(0))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_month_map() {
        let map = get_month_map();
        assert_eq!(map.get("Jan"), Some(&1));
        assert_eq!(map.get("December"), Some(&12));
        assert_eq!(map.get("February"), Some(&2));
    }

    #[test]
    fn test_get_abbreviated_month_map() {
        let map = get_abbreviated_month_map();
        assert_eq!(map.get("Jan"), Some(&1));
        assert_eq!(map.get("Dec"), Some(&12));
        assert_eq!(map.get("June"), None); // Should only have abbreviated
        assert_eq!(map.get("January"), None); // Should only have abbreviated
    }

    #[test]
    fn test_parse_time_with_ampm_am() {
        let result = parse_time_with_ampm("10:30:45 AM").unwrap();
        assert_eq!(result, (10, 30, 45));

        let result = parse_time_with_ampm("12:00:00 AM").unwrap(); // Midnight
        assert_eq!(result, (0, 0, 0));

        let result = parse_time_with_ampm("11:59:59 AM").unwrap();
        assert_eq!(result, (11, 59, 59));
    }

    #[test]
    fn test_parse_time_with_ampm_pm() {
        let result = parse_time_with_ampm("10:30:45 PM").unwrap();
        assert_eq!(result, (22, 30, 45)); // 10 PM = 22:00

        let result = parse_time_with_ampm("12:00:00 PM").unwrap(); // Noon
        assert_eq!(result, (12, 0, 0));

        let result = parse_time_with_ampm("1:00:00 PM").unwrap();
        assert_eq!(result, (13, 0, 0)); // 1 PM = 13:00
    }

    #[test]
    fn test_parse_time_with_ampm_24h() {
        let result = parse_time_with_ampm("10:30:45").unwrap();
        assert_eq!(result, (10, 30, 45));

        let result = parse_time_with_ampm("23:59:59").unwrap();
        assert_eq!(result, (23, 59, 59));

        let result = parse_time_with_ampm("00:00:00").unwrap();
        assert_eq!(result, (0, 0, 0));
    }

    #[test]
    fn test_parse_time_with_ampm_no_seconds() {
        let result = parse_time_with_ampm("10:30 AM").unwrap();
        assert_eq!(result, (10, 30, 0));

        let result = parse_time_with_ampm("10:30 PM").unwrap();
        assert_eq!(result, (22, 30, 0));
    }

    #[test]
    fn test_parse_microseconds() {
        let result = parse_microseconds("123456").unwrap();
        assert_eq!(result, 123456);

        let result = parse_microseconds("123").unwrap();
        assert_eq!(result, 123000); // Padded to 6 digits

        let result = parse_microseconds("12").unwrap();
        assert_eq!(result, 120000); // Padded to 6 digits

        let result = parse_microseconds("1234567").unwrap();
        assert_eq!(result, 123456); // Truncated to 6 digits
    }

    #[test]
    fn test_extract_microseconds() {
        use regex::Regex;
        let re = Regex::new(r"(\d+)").unwrap();

        let cap = re.captures("123456").and_then(|c| c.get(1));
        let result = extract_microseconds(cap);
        assert_eq!(result, 123456);

        let cap = re.captures("123").and_then(|c| c.get(1));
        let result = extract_microseconds(cap);
        assert_eq!(result, 123000); // Padded

        let result = extract_microseconds(None);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_regex_tz_colon() {
        assert!(RE_TZ_COLON.is_match("+10:30"));
        assert!(RE_TZ_COLON.is_match("-05:00"));
        assert!(RE_TZ_COLON.is_match("+1:00"));
    }

    #[test]
    fn test_regex_tz_4digit() {
        assert!(RE_TZ_4DIGIT.is_match("+1000"));
        assert!(RE_TZ_4DIGIT.is_match("-0530"));
    }

    #[test]
    fn test_regex_time_24h() {
        assert!(RE_TIME_24H.is_match("10:30"));
        assert!(RE_TIME_24H.is_match("10:30:45"));
        assert!(RE_TIME_24H.is_match("23:59:59"));
    }
}
