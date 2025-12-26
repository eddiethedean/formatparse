use pyo3::prelude::*;
use regex::Regex;
use std::collections::HashMap;

/// Month name to number mapping (abbreviated and full names)
pub fn get_month_map() -> HashMap<&'static str, u8> {
    [
        ("Jan", 1), ("Feb", 2), ("Mar", 3), ("Apr", 4),
        ("May", 5), ("Jun", 6), ("Jul", 7), ("Aug", 8),
        ("Sep", 9), ("Oct", 10), ("Nov", 11), ("Dec", 12),
        ("January", 1), ("February", 2), ("March", 3), ("April", 4),
        ("June", 6), ("July", 7), ("August", 8),
        ("September", 9), ("October", 10), ("November", 11), ("December", 12),
    ].iter().cloned().collect()
}

/// Month name to number mapping (abbreviated only)
pub fn get_abbreviated_month_map() -> HashMap<&'static str, u8> {
    [
        ("Jan", 1), ("Feb", 2), ("Mar", 3), ("Apr", 4),
        ("May", 5), ("Jun", 6), ("Jul", 7), ("Aug", 8),
        ("Sep", 9), ("Oct", 10), ("Nov", 11), ("Dec", 12),
    ].iter().cloned().collect()
}

/// Create a FixedTzOffset from offset minutes
pub fn create_fixed_tz(py: Python, offset_minutes: i32, name: &str) -> PyResult<PyObject> {
    let fixed_tz_module = py.import_bound("structparse")?;
    let fixed_tz_class = fixed_tz_module.getattr("FixedTzOffset")?;
    let tz = fixed_tz_class.call1((offset_minutes, name.to_string(),))?;
    Ok(tz.to_object(py))
}

/// Parse timezone string into FixedTzOffset
/// Handles formats: +1:00, +10:00, +10:30, +1000, etc.
pub fn parse_timezone(py: Python, tz_str: &str) -> PyResult<PyObject> {
    // Handle formats: +1:00, +10:00, +10:30, +1000, etc.
    if let Ok(re) = Regex::new(r"([+-])(\d{1,2}):?(\d{2})") {
        if let Some(caps) = re.captures(tz_str) {
            if let (Some(sign_match), Some(hour_match), Some(min_match)) = (caps.get(1), caps.get(2), caps.get(3)) {
                let sign = if sign_match.as_str() == "+" { 1 } else { -1 };
                let hour: i32 = hour_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone"))?;
                let min: i32 = min_match.as_str().parse().unwrap_or(0);
                let offset_minutes = sign * (hour * 60 + min);
                return create_fixed_tz(py, offset_minutes, "");
            }
        }
    }
    // Also handle 4-digit format: +1000 (1 hour, 00 minutes)
    if let Ok(re) = Regex::new(r"([+-])(\d{4})") {
        if let Some(caps) = re.captures(tz_str) {
            if let (Some(sign_match), Some(tz_match)) = (caps.get(1), caps.get(2)) {
                let sign = if sign_match.as_str() == "+" { 1 } else { -1 };
                let tz_str = tz_match.as_str();
                // Regex ensures exactly 4 digits, but add defensive check
                if tz_str.len() >= 4 {
                    let hour: i32 = tz_str[..2].parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone"))?;
                    let min: i32 = tz_str[2..4].parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone"))?;
                    let offset_minutes = sign * (hour * 60 + min);
                    return create_fixed_tz(py, offset_minutes, "");
                }
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
        if let Ok(re) = Regex::new(r"(\d{1,2}):(\d{2})(?::(\d{2}))?") {
            if let Some(caps) = re.captures(time_part) {
                hour = caps.get(1).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
                minute = caps.get(2).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
                second = caps.get(3).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0);
                // 12 AM becomes 0 (midnight), other AM hours stay as-is
                if hour == 12 {
                    hour = 0;
                }
            }
        }
    } else if let Some(pm_idx) = time_str.to_uppercase().find("PM") {
        let time_part = &time_str[..pm_idx].trim();
        if let Ok(re) = Regex::new(r"(\d{1,2}):(\d{2})(?::(\d{2}))?") {
            if let Some(caps) = re.captures(time_part) {
                hour = caps.get(1).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
                minute = caps.get(2).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
                second = caps.get(3).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0);
                // 12 PM stays as 12 (noon), other PM hours add 12
                if hour != 12 {
                    hour += 12;
                }
            }
        }
    } else {
        // 24-hour format
        if let Ok(re) = Regex::new(r"(\d{1,2}):(\d{2})(?::(\d{2}))?") {
            if let Some(caps) = re.captures(time_str) {
                hour = caps.get(1).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
                minute = caps.get(2).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
                second = caps.get(3).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0);
            }
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
    padded.parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid microsecond: {}", micros_str)))
}

/// Extract microseconds from a capture group, handling padding
pub fn extract_microseconds(cap: Option<regex::Match>) -> u32 {
    cap.map(|m| {
        let s = m.as_str();
        let padded = format!("{:0<6}", s);
        padded[..6.min(padded.len())].parse().unwrap_or(0)
    }).unwrap_or(0)
}

